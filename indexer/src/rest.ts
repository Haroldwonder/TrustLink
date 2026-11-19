import { PrismaClient } from "@prisma/client";
import Fastify, { FastifyInstance } from "fastify";
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

let rpcClient: any | null = null;

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

export function buildRestServer(db: PrismaClient): FastifyInstance {
  const fastify = Fastify({ logger: false });

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
    const MAX_LAG = parseInt(process.env.MAX_LAG_LEDGERS ?? "10", 10);
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

  fastify.get("/metrics", async () => getMetrics());

  fastify.get<{ Params: { subject: string }; Querystring: { limit?: string; after?: string } }>(
    "/attestations/:subject",
    async (req) => {
      const limit = Math.min(parseInt(req.query.limit ?? "50", 10) || 50, 500);
      const after = req.query.after;

      const where: Record<string, any> = { subject: req.params.subject };
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
    async (req, reply) => {
      // Get ledger time from latest checkpoint for consistency with on-chain behavior
      const checkpoint = await db.checkpoint.findUnique({ where: { id: 1 } });
      // Use ledger sequence as the time source (falls back to 0 if no checkpoint)
      const ledgerTime = checkpoint?.ledger ?? 0;

      const attestation = await db.attestation.findFirst({
        where: {
          subject: req.params.address,
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
    async (req) => {
      const limit = Math.min(parseInt(req.query.limit ?? "50", 10) || 50, 500);
      const after = req.query.after;

      const where: Record<string, any> = { issuer: req.params.address };
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

    const where = buildAttestationExportWhere({
      issuer: req.query.issuer,
      subject: req.query.subject,
      claimType: req.query.claim_type,
      fromTimestamp,
      toTimestamp,
    });

    const contentType = format === "csv" ? "text/csv; charset=utf-8" : "application/json; charset=utf-8";
    const extension = format === "csv" ? "csv" : "json";

    reply
      .header("Content-Type", contentType)
      .header("Content-Disposition", `attachment; filename="attestations.${extension}"`)
      .header("Transfer-Encoding", "chunked");

    return reply.send(createAttestationExportStream(db, where, format));
  });

  fastify.get("/stats", async () => {
    const [total, revoked, issuers] = await Promise.all([
      db.attestation.count(),
      db.attestation.count({ where: { isRevoked: true } }),
      db.attestation.findMany({
        distinct: ["issuer"],
        select: { issuer: true },
      }),
    ]);
    return {
      total_attestations: total,
      total_revocations: revoked,
      total_issuers: issuers.length,
    };
  });

  fastify.get("/webhooks", async () =>
    db.webhook.findMany({
      select: { id: true, url: true, active: true, rotatedAt: true, createdAt: true },
      orderBy: { createdAt: "desc" },
    }),
  );

  fastify.post<{ Body: { url: string; secret?: string } }>(
    "/webhooks",
    async (req, reply) => {
      const { url, secret: clientSecret } = req.body ?? {};
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
          },
        });
        
        reply.code(201);
        // Return the plaintext secret ONLY once on creation
        return {
          id: webhook.id,
          url: webhook.url,
          secret: secret, // Returned ONLY on creation
          active: webhook.active,
          rotatedAt: webhook.rotatedAt,
        };
      } catch (err) {
        reply.code(400);
        return { error: err instanceof Error ? err.message : "failed to create webhook" };
      }
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
        
        reply.code(200);
        // Return the NEW plaintext secret ONLY once
        return {
          id: webhook.id,
          url: webhook.url,
          secret: newSecret, // New secret returned once
          rotatedAt: new Date(),
          graceMessage: "Old secret will be accepted for 7 days via X-TrustLink-Signature-Next header",
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
    const offset = parseInt(offsetStr ?? "0", 10) || 0;
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
