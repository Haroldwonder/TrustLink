import { PrismaClient } from "@prisma/client";
import Fastify, { FastifyInstance } from "fastify";
import swagger from "@fastify/swagger";
import swaggerUi from "@fastify/swagger-ui";
import { requireApiKey } from "./auth";
import {
  buildAttestationExportWhere,
  createAttestationExportStream,
  parseExportFormat,
  parseTimestampQuery,
} from "./export";
import { getLastLedger } from "./indexer-state";
import { getMetrics } from "./metrics";
import { validateWebhookUrl } from "./webhook-validation";
import { getVersionInfo } from "./version";
import { StrKey } from "@stellar/stellar-sdk";

let rpcClient: any | null = null;

// Parse and validate MAX_LAG_LEDGERS at startup (fail fast on invalid values)
let MAX_LAG: number;
try {
  const lagStr = process.env.MAX_LAG_LEDGERS ?? "10";
  const parsed = parseInt(lagStr, 10);
  if (isNaN(parsed) || parsed < 0) {
    throw new Error(`Invalid MAX_LAG_LEDGERS: "${lagStr}" is not a valid non-negative integer`);
  }
  MAX_LAG = parsed;
} catch (err) {
  console.error("Failed to parse MAX_LAG_LEDGERS:", err instanceof Error ? err.message : err);
  process.exit(1);
}

// Helper to validate Stellar addresses (Ed25519 public key or contract)
function isValidStellarAddress(address: string): boolean {
  return StrKey.isValidEd25519PublicKey(address) || StrKey.isValidContract(address);
}

async function getRpcClient() {
  if (!rpcClient) {
    const { rpc } = await import("@stellar/stellar-sdk");
    const allowHttp = process.env.RPC_ALLOW_HTTP === "true" || process.env.NODE_ENV !== "production";
    rpcClient = new rpc.Server(
      process.env.RPC_URL ?? "https://soroban-testnet.stellar.org",
      { allowHttp },
    );
  }
  return rpcClient;
}

// Schema definitions for OpenAPI documentation
const attestationsBySubjectSchema = {
  params: {
    type: "object",
    required: ["subject"],
    properties: {
      subject: { type: "string", description: "Subject wallet address" },
    },
  },
  querystring: {
    type: "object",
    properties: {
      limit: { type: "string", default: "50" },
      after: { type: "string", description: "Cursor for pagination" },
      claim_type: { type: "string", description: "Filter by claim type" },
      issuer: { type: "string", description: "Filter by issuer address" },
      revoked: { type: "string", enum: ["true", "false"], description: "Filter by revocation status" },
    },
  },
  response: {
    200: {
      type: "object",
      properties: {
        items: {
          type: "array",
          items: {
            type: "object",
            properties: {
              id: { type: "string" },
              issuer: { type: "string" },
              subject: { type: "string" },
              claimType: { type: "string" },
              timestamp: { type: "string" },
              expiration: { type: ["string", "null"] },
              isRevoked: { type: "boolean" },
            },
          },
        },
        nextCursor: { type: ["string", "null"] },
      },
    },
  },
};

const attestationsByIssuerSchema = {
  params: {
    type: "object",
    required: ["address"],
    properties: {
      address: { type: "string", description: "Issuer wallet address" },
    },
  },
  querystring: {
    type: "object",
    properties: {
      limit: { type: "string", default: "50" },
      after: { type: "string", description: "Cursor for pagination" },
    },
  },
  response: {
    200: {
      type: "object",
      properties: {
        items: { type: "array" },
        nextCursor: { type: ["string", "null"] },
      },
    },
  },
};

const validClaimSchema = {
  params: {
    type: "object",
    required: ["address", "claim_type"],
    properties: {
      address: { type: "string", description: "Subject wallet address" },
      claim_type: { type: "string", description: "Claim type identifier" },
    },
  },
  response: {
    200: {
      type: "object",
      properties: {
        valid: { type: "boolean" },
        attestationId: { type: ["string", "null"] },
      },
    },
  },
};

const statsSchema = {
  response: {
    200: {
      type: "object",
      properties: {
        total_attestations: { type: "integer" },
        total_revocations: { type: "integer" },
        total_issuers: { type: "integer" },
      },
    },
  },
};

const versionSchema = {
  response: {
    200: {
      type: "object",
      properties: {
        version: { type: "string", description: "Indexer package version" },
        buildTimestamp: { type: "string", description: "Build timestamp" },
        contractId: { type: "string", description: "Stellar contract ID" },
        network: { type: "string", description: "Network (testnet/mainnet)" },
        nodeEnv: { type: "string", description: "Node environment" },
        timestamp: { type: "string", description: "Current server time" },
      },
    },
  },
};

export function buildRestServer(db: PrismaClient): FastifyInstance {
  const fastify = Fastify({
    logger: {
      level: logger.level,
      transport: {
        target: "pino/file",
        options: {
          destination: 1, // stdout
        },
      },
      serializers: {
        req(request) {
          return {
            method: request.method,
            url: request.url,
            headers: {
              ...request.headers,
              "x-api-key": request.headers["x-api-key"] ? "[REDACTED]" : undefined,
            },
            correlationId: request.headers["x-correlation-id"],
          };
        },
      },
    },
  });

  // Register OpenAPI/Swagger documentation
  fastify.register(swagger, {
    openapi: {
      info: {
        title: "TrustLink Indexer API",
        description: "REST API for querying attestations, managing webhooks, and monitoring the indexer",
        version: "1.0.0",
      },
      servers: [
        { url: "https://indexer.trustlink.io", description: "Production" },
        { url: "https://indexer-testnet.trustlink.io", description: "Testnet" },
        { url: "http://localhost:3000", description: "Local development" },
      ],
      components: {
        securitySchemes: {
          ApiKey: {
            type: "apiKey",
            in: "header",
            name: "X-API-Key",
            description: "API key for admin endpoints",
          },
        },
      },
      tags: [
        { name: "Attestations", description: "Query attestations by subject or issuer" },
        { name: "Webhooks", description: "Manage webhook subscriptions" },
        { name: "Admin", description: "Admin operations and monitoring" },
        { name: "Health", description: "Health check and status endpoints" },
      ],
    },
  });

  fastify.register(swaggerUi, {
    routePrefix: "/docs",
    uiConfig: {
      docExpansion: "list",
      deepLinking: false,
    },
  });

  fastify.get("/health", async (_request, reply) => {
    let dbConnected = false;
    try {
      await db.$queryRaw`SELECT 1`;
      dbConnected = true;
    } catch {
      dbConnected = false;
    }

    if (!dbConnected) {
      reply.code(503);
      return {
        status: "error",
        db: "disconnected",
        lastLedger: getLastLedger(),
      };
    }

    return {
      status: "ok",
      db: "connected",
      lastLedger: getLastLedger(),
    };
  });

  fastify.get("/ready", async (_request, reply) => {
    try {
      const checkpoint = await db.checkpoint.findUnique({ where: { id: 1 } });
      const rpc = await getRpcClient();
      const { sequence: tip } = await rpc.getLatestLedger();
      const lag = tip - (checkpoint?.ledger ?? 0);
      
      if (lag <= MAX_LAG) {
        return { status: "ready", lag };
      } else {
        reply.code(503);
        return { status: "unavailable", reason: "indexer_lag_exceeded", lag, max: MAX_LAG };
      }
    } catch (err) {
      reply.code(503);
      const reason = err instanceof Error ? err.message : "unknown_error";
      return { status: "unavailable", reason };
    }
  });

  fastify.get("/version", { schema: versionSchema }, async () => {
    return getVersionInfo();
  });

  fastify.get("/metrics", async () => getMetrics());

  fastify.get<{ Params: { subject: string }; Querystring: { limit?: string; after?: string } }>(
    "/attestations/:subject",
    { schema: attestationsBySubjectSchema },
    async (req, reply) => {
      const { subject } = req.params;
      
      // Validate address format
      if (!isValidStellarAddress(subject)) {
        reply.code(400);
        return { error: "Invalid address: must be a valid Ed25519 public key or contract" };
      }

      const limit = Math.min(parseInt(req.query.limit ?? "50", 10) || 50, 500);
      const after = req.query.after;

      // Validate cursor format - must be a positive integer if provided
      if (after !== undefined) {
        const cursorNum = parseInt(after, 10);
        if (isNaN(cursorNum) || cursorNum <= 0) {
          reply.code(400);
          return { error: "Invalid cursor: must be a positive integer" };
        }
      }

      const where: Record<string, any> = { subject };
      const cursor = after ? { id: after } : undefined;

      const attestations = await db.attestation.findMany({
        where,
        orderBy: [{ timestamp: "desc" }, { id: "desc" }],
        take: limit + 1,
        skip: cursor ? 1 : 0,
        cursor,
      });

      const hasMore = attestations.length > limit;
      const items = attestations.slice(0, limit);
      const nextCursor = hasMore ? items[items.length - 1]?.id : null;

      return { items, nextCursor };
    },
  );

  fastify.get<{ Params: { address: string; claim_type: string } }>(
    "/subjects/:address/claims/:claim_type/valid",
    { schema: validClaimSchema },
    async (req, reply) => {
      const { address } = req.params;
      
      // Validate address format
      if (!isValidStellarAddress(address)) {
        reply.code(400);
        return { error: "Invalid address: must be a valid Ed25519 public key or contract" };
      }

      // Get ledger time from latest checkpoint for consistency with on-chain behavior
      const checkpoint = await db.checkpoint.findUnique({ where: { id: 1 } });
      // Use ledger sequence as the time source (falls back to 0 if no checkpoint)
      const ledgerTime = checkpoint?.ledger ?? 0;

      const attestation = await db.attestation.findFirst({
        where: {
          subject: address,
          claimType: req.params.claim_type,
          isRevoked: false,
          // Filter out expired attestations: expiration IS NULL OR expiration > ledgerTime
          OR: [
            { expiration: null },
            { expiration: { gt: ledgerTime } },
          ],
          // TODO: Add validFrom field to schema and index on (subject, claimType, isRevoked, expiration)
          // When validFrom is added, add: OR: [{ validFrom: null }, { validFrom: { lte: ledgerTime } }]
        },
      });

      if (!attestation) {
        return { valid: false, attestationId: null };
      }

      return { valid: true, attestationId: attestation.id };
    },
  );

  fastify.get<{ Params: { address: string }; Querystring: { limit?: string; after?: string } }>(
    "/issuers/:address/attestations",
    { schema: attestationsByIssuerSchema },
    async (req, reply) => {
      const { address } = req.params;
      
      // Validate address format
      if (!isValidStellarAddress(address)) {
        reply.code(400);
        return { error: "Invalid address: must be a valid Ed25519 public key or contract" };
      }

      const limit = Math.min(parseInt(req.query.limit ?? "50", 10) || 50, 500);
      const after = req.query.after;

      // Validate cursor format - must be a positive integer if provided
      if (after !== undefined) {
        const cursorNum = parseInt(after, 10);
        if (isNaN(cursorNum) || cursorNum <= 0) {
          reply.code(400);
          return { error: "Invalid cursor: must be a positive integer" };
        }
      }

      const where: Record<string, any> = { issuer: address };
      const cursor = after ? { id: after } : undefined;

      const attestations = await db.attestation.findMany({
        where,
        orderBy: [{ timestamp: "desc" }, { id: "desc" }],
        take: limit + 1,
        skip: cursor ? 1 : 0,
        cursor,
      });

      const hasMore = attestations.length > limit;
      const items = attestations.slice(0, limit);
      const nextCursor = hasMore ? items[items.length - 1]?.id : null;

      return { items, nextCursor };
    },
  );

  fastify.get<{
    Querystring: {
      format?: string;
      issuer?: string;
      subject?: string;
      claim_type?: string;
      from?: string;
      to?: string;
    };
  }>("/export/attestations", async (req, reply) => {
    if (!requireApiKey(req, reply)) {
      return;
    }

    const format = parseExportFormat(req.query.format);
    if (!format) {
      reply.code(400);
      return { error: "Invalid format; use csv or json" };
    }

    const fromTimestamp = parseTimestampQuery(req.query.from);
    const toTimestamp = parseTimestampQuery(req.query.to);
    if ((req.query.from && fromTimestamp === undefined) || (req.query.to && toTimestamp === undefined)) {
      reply.code(400);
      return { error: "Invalid from/to timestamp" };
    }

    const rangeError = validateExportDateRange(fromTimestamp, toTimestamp);
    if (rangeError) {
      reply.code(400);
      return { error: rangeError };
    }

    const where = buildAttestationExportWhere({
      issuer: req.query.issuer,
      subject: req.query.subject,
      claimType: req.query.claim_type,
      fromTimestamp,
      toTimestamp,
    });

    const maxRows = getExportMaxRows();
    if (typeof db.attestation?.count === "function") {
      const totalMatching = await db.attestation.count({ where });
      if (totalMatching > maxRows) {
        reply.code(400);
        return { error: `Export exceeds maximum allowed rows (${maxRows})` };
      }
    }

    const contentType = format === "csv" ? "text/csv; charset=utf-8" : "application/json; charset=utf-8";
    const extension = format === "csv" ? "csv" : "json";

    reply
      .header("Content-Type", contentType)
      .header("Content-Disposition", `attachment; filename="attestations.${extension}"`)
      .header("Transfer-Encoding", "chunked");

    return reply.send(createAttestationExportStream(db, where, format, maxRows));
  });

  fastify.get("/stats", { schema: statsSchema }, async () => {
    const [total, revoked, issuers] = await Promise.all([
      db.attestation.count(),
      db.attestation.count({ where: { isRevoked: true } }),
      db.issuer.count(),
    ]);
    return {
      total_attestations: total,
      total_revocations: revoked,
      total_issuers: issuers,
    };
  });

  fastify.get<{ Querystring: { limit?: string; after?: string; active?: string } }>(
    "/webhooks",
    async (req) => {
      const limit = Math.min(parseInt(req.query.limit ?? "50", 10) || 50, 500);
      const after = req.query.after;
      const activeFilter = req.query.active === "false" ? false : req.query.active === "true" ? true : undefined;

      const where: Record<string, any> = {};
      if (activeFilter !== undefined) {
        where.active = activeFilter;
      }

      const cursor = after ? { id: after } : undefined;

      const webhooks = await db.webhook.findMany({
        where,
        select: { id: true, url: true, active: true, rotatedAt: true, createdAt: true },
        orderBy: { createdAt: "desc" },
        take: limit + 1,
        skip: cursor ? 1 : 0,
        cursor,
      });

      const hasMore = webhooks.length > limit;
      const items = webhooks.slice(0, limit);
      const nextCursor = hasMore ? items[items.length - 1]?.id : null;

      return { items, nextCursor };
    },
  );

  fastify.post<{ Body: { url: string; secret?: string; eventTypes?: string[] } }>(
    "/webhooks",
    async (req, reply) => {
      const { url, secret: clientSecret, eventTypes } = req.body ?? {};
      if (!url) {
        reply.code(400);
        return { error: "url is required" };
      }

      try {
        await validateWebhookUrl(url);
      } catch (err) {
        reply.code(400);
        return { error: err instanceof Error ? err.message : "invalid webhook url" };
      }

      let validatedEventTypes: string[] | undefined;
      if (eventTypes !== undefined) {
        try {
          validatedEventTypes = validateWebhookEventTypes(eventTypes);
        } catch (err) {
          reply.code(400);
          return { error: err instanceof Error ? err.message : "invalid eventTypes" };
        }
      }

      // If client provides a secret, validate it; otherwise generate server-side
      let secret: string;
      try {
        const { validateWebhookSecret, generateWebhookSecret, encryptSecret } = await import("./webhook-secret");
        
        if (clientSecret) {
          const secretBuffer = Buffer.from(clientSecret, "hex");
          validateWebhookSecret(secretBuffer);
          secret = clientSecret;
        } else {
          secret = generateWebhookSecret();
        }
        
        const secretEncrypted = encryptSecret(secret);
        const webhook = await db.webhook.create({
          data: {
            url,
            secret_encrypted: secretEncrypted,
            ...(validatedEventTypes !== undefined ? { eventTypes: validatedEventTypes } : {}),
          },
        });
        
        reply.code(201);
        // Return the plaintext secret ONLY once on creation
        return {
          id: webhook.id,
          url: webhook.url,
          secret: secret, // Returned ONLY on creation
          active: webhook.active,
          eventTypes: webhook.eventTypes ?? validatedEventTypes ?? [],
          rotatedAt: webhook.rotatedAt,
        };
      } catch (err) {
        reply.code(400);
        return { error: err instanceof Error ? err.message : "failed to create webhook" };
      }
    },
  );

  fastify.patch<{
    Params: { id: string };
    Body: { active?: boolean; url?: string; eventTypes?: string[] };
  }>(
    "/webhooks/:id",
    async (req, reply) => {
      const { active, url, eventTypes } = req.body ?? {};

      if (active === undefined && url === undefined && eventTypes === undefined) {
        reply.code(400);
        return { error: "At least one of active, url, or eventTypes must be provided" };
      }

      const updateData: { active?: boolean; url?: string; eventTypes?: string[] } = {};

      if (active !== undefined) {
        if (typeof active !== "boolean") {
          reply.code(400);
          return { error: "active must be a boolean" };
        }
        updateData.active = active;
      }

      if (url !== undefined) {
        if (typeof url !== "string" || !url) {
          reply.code(400);
          return { error: "url must be a non-empty string" };
        }
        try {
          await validateWebhookUrl(url);
        } catch (err) {
          reply.code(400);
          return { error: err instanceof Error ? err.message : "invalid webhook url" };
        }
        updateData.url = url;
      }

      if (eventTypes !== undefined) {
        try {
          updateData.eventTypes = validateWebhookEventTypes(eventTypes);
        } catch (err) {
          reply.code(400);
          return { error: err instanceof Error ? err.message : "invalid eventTypes" };
        }
      }

      const existing = await db.webhook.findUnique({ where: { id: req.params.id } });
      if (!existing) {
        reply.code(404);
        return { error: "Webhook not found" };
      }

      const updated = await db.webhook.update({
        where: { id: req.params.id },
        data: updateData,
        select: {
          id: true,
          url: true,
          active: true,
          eventTypes: true,
          rotatedAt: true,
          createdAt: true,
          updatedAt: true,
        },
      });

      return updated;
    },
  );

  fastify.delete<{ Params: { id: string } }>(
    "/webhooks/:id",
    async (req, reply) => {
      try {
        await db.webhook.delete({ where: { id: req.params.id } });
        reply.code(204);
        return;
      } catch {
        reply.code(404);
        return { error: "Webhook not found" };
      }
    },
  );

  /// Issue #1604: Rotate webhook secret with grace period for dual-signing
  fastify.post<{ Params: { id: string } }>(
    "/webhooks/:id/rotate-secret",
    async (req, reply) => {
      try {
        const { generateWebhookSecret, encryptSecret } = await import("./webhook-secret");
        
        const webhook = await db.webhook.findUnique({
          where: { id: req.params.id },
        });
        
        if (!webhook) {
          reply.code(404);
          return { error: "Webhook not found" };
        }
        
        // Generate new secret
        const newSecret = generateWebhookSecret();
        const newSecretEncrypted = encryptSecret(newSecret);
        
        // Update webhook: move current secret to next_secret and set new secret
        // This enables dual-signing during the grace period
        await db.webhook.update({
          where: { id: req.params.id },
          data: {
            next_secret_encrypted: webhook.secret_encrypted, // Old secret becomes next
            secret_encrypted: newSecretEncrypted,
            rotatedAt: new Date(),
          },
        });
        
        const gracePeriodDays = parseInt(process.env.WEBHOOK_ROTATION_GRACE_DAYS ?? "7", 10);
        
        reply.code(200);
        // Return the NEW plaintext secret ONLY once
        return {
          id: webhook.id,
          url: webhook.url,
          secret: newSecret, // New secret returned once
          rotatedAt: new Date(),
          gracePeriodDays,
          graceMessage: `Old secret will be accepted for ${gracePeriodDays} days via X-TrustLink-Signature-Next header`,
        };
      } catch (err) {
        reply.code(500);
        return { error: err instanceof Error ? err.message : "failed to rotate secret" };
      }
    },
  );

  fastify.post<{ Querystring: { from?: string } }>(
    "/admin/reindex",
    async (req, reply) => {
      const from = req.query.from ? parseInt(req.query.from, 10) : getLastLedger();
      if (isNaN(from) || from < 0) {
        reply.code(400);
        return { error: "Invalid 'from' ledger number" };
      }
      reply.code(202);
      return { message: `Reindex started from ledger ${from}` };
    },
  );

  fastify.get<{
    Querystring: {
      status?: string;
      eventType?: string;
      limit?: string;
      offset?: string;
      sort?: string;
    };
  }>("/admin/webhook-failures", async (req, reply) => {
    const { status, eventType, limit: limitStr, offset: offsetStr, sort } = req.query;
    const limit = Math.min(parseInt(limitStr ?? "50", 10) || 50, 200);
    
    // Validate offset: must be a non-negative integer
    const offset = parseInt(offsetStr ?? "0", 10);
    if (isNaN(offset) || offset < 0 || !Number.isInteger(offset)) {
      reply.code(400);
      return { error: "Invalid offset: must be a non-negative integer" };
    }
    
    const orderBy = sort === "asc" ? "asc" : "desc";

    const where: Record<string, unknown> = {};
    if (status) {
      if (!["FAILED", "RETRYING", "RECOVERED"].includes(status)) {
        reply.code(400);
        return { error: "Invalid status filter" };
      }
      where.status = status;
    }
    if (eventType) {
      where.eventType = eventType;
    }

    const [items, total] = await Promise.all([
      db.webhookFailure.findMany({
        where,
        orderBy: { failedAt: orderBy },
        skip: offset,
        take: limit,
        select: {
          id: true,
          webhookId: true,
          url: true,
          eventType: true,
          statusCode: true,
          errorMessage: true,
          attemptCount: true,
          status: true,
          failedAt: true,
          resolvedAt: true,
          updatedAt: true,
        },
      }),
      db.webhookFailure.count({ where }),
    ]);

    return { items, total, limit, offset };
  });

  fastify.post<{ Params: { id: string } }>(
    "/admin/retry-webhook/:id",
    async (req, reply) => {
      const { id } = req.params;
      if (!id) {
        reply.code(400);
        return { error: "Missing failure id" };
      }
      const { replayFailure } = await import("./webhooks");
      const result = await replayFailure(db, id);
      if (result.error === "Not found") {
        reply.code(404);
        return { error: "Webhook failure record not found" };
      }
      if (result.error === "Retry already in progress") {
        reply.code(409);
        return { error: result.error };
      }
      if (result.success) {
        return { success: true, statusCode: result.statusCode };
      }
      reply.code(502);
      return { success: false, statusCode: result.statusCode, error: result.error };
    },
  );

  return fastify;
}
