import { initTracing, getTracer } from "./tracing";
initTracing(); // must be first — instruments http, pg, etc.

import { PrismaClient } from "@prisma/client";
import { createServer, IncomingMessage, ServerResponse } from "http";
import Fastify from "fastify";
import { ApolloServer, HeaderMap } from "@apollo/server";
import { makeExecutableSchema } from "@graphql-tools/schema";
import { WebSocketServer } from "ws";
import { useServer } from "graphql-ws/use/ws";
import { readFileSync } from "fs";
import { join } from "path";
import { startIndexer, getLastLedger, reindex } from "./indexer";
import { buildResolvers } from "./graphql";
import { getMetrics } from "./metrics";
import Redis from "ioredis";

const db = new PrismaClient();

// #777: optional Redis client — only connect when REDIS_URL is set
const redis = process.env.REDIS_URL
  ? new Redis(process.env.REDIS_URL, { lazyConnect: true, enableOfflineQueue: false })
  : null;
if (redis) {
  redis.connect().catch((err: unknown) => {
    console.warn("Redis connection failed, caching disabled:", err);
  });
}

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    let body = "";
    req.on("data", (chunk) => (body += chunk));
    req.on("end", () => resolve(body));
    req.on("error", reject);
  });
}

async function main() {
  await db.$connect();

  // ── REST (Fastify) ─────────────────────────────────────────────────────────
  const fastify = Fastify({ logger: true });

  fastify.get("/health", async (request, reply) => {
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

  fastify.get("/ready", async () => {
    const checkpoint = await db.checkpoint.findUnique({ where: { id: 1 } });
    const rpc = new (await import("@stellar/stellar-sdk")).rpc.Server(
      process.env.RPC_URL ?? "https://soroban-testnet.stellar.org",
      { allowHttp: true }
    );
    const { sequence: tip } = await rpc.getLatestLedger();
    const lag = tip - (checkpoint?.ledger ?? 0);
    if (lag <= 10) {
      return { status: 200 };
    }
    return { status: 503 };
  });

  fastify.get("/metrics", async () => {
    const metrics = await getMetrics();
    return metrics;
  });

  fastify.get<{ Params: { subject: string } }>(
    "/attestations/:subject",
    async (req) => {
      return db.attestation.findMany({
        where: { subject: req.params.address },
        orderBy: { timestamp: "desc" },
      });
    }
  );

  fastify.get<{ Params: { address: string; claim_type: string } }>(
    "/subjects/:address/claims/:claim_type/valid",
    async (req) => {
      const attestation = await db.attestation.findFirst({
        where: {
          subject: req.params.address,
          claimType: req.params.claim_type,
          isRevoked: false,
        },
      });
      return { valid: !!attestation };
    }
  );

  fastify.get<{ Params: { address: string } }>(
    "/issuers/:address/attestations",
    async (req) => {
      return db.attestation.findMany({
        where: { issuer: req.params.address },
        orderBy: { timestamp: "desc" },
      });
    }
  );

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

  fastify.get("/webhooks", async () => {
    const webhooks = await db.webhook.findMany({
      select: { id: true, url: true, active: true, createdAt: true },
      orderBy: { createdAt: "desc" },
    });
    return webhooks;
  });

  fastify.post<{ Body: { url: string; secret: string } }>(
    "/webhooks",
    async (req, reply) => {
      const { url, secret } = req.body ?? {};
      if (!url || !secret) {
        reply.code(400);
        return { error: "url and secret are required" };
      }
      const webhook = await db.webhook.create({ data: { url, secret } });
      reply.code(201);
      return { id: webhook.id, url: webhook.url, active: webhook.active };
    }
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
    }
  );

  fastify.post<{ Querystring: { from?: string } }>(
    "/admin/reindex",
    async (req, reply) => {
      const from = req.query.from ? parseInt(req.query.from, 10) : getLastLedger();
      if (isNaN(from) || from < 0) {
        reply.code(400);
        return { error: "Invalid 'from' ledger number" };
      }
      reindex(db, from).catch((err) => {
        logger.error({ err }, "Reindex error");
      });
      reply.code(202);
      return { message: `Reindex started from ledger ${from}` };
    }
  );

  fastify.get<{
    Querystring: { status?: string; eventType?: string; limit?: string; offset?: string; sort?: string };
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
    if (eventType) where.eventType = eventType;

    const [items, total] = await Promise.all([
      db.webhookFailure.findMany({
        where,
        orderBy: { failedAt: orderBy },
        skip: offset,
        take: limit,
        select: {
          id: true, webhookId: true, url: true, eventType: true,
          statusCode: true, errorMessage: true, attemptCount: true,
          status: true, failedAt: true, resolvedAt: true, updatedAt: true,
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
    }
  );

  const REST_PORT = Number(process.env.PORT ?? 3000);
  await fastify.listen({ port: REST_PORT, host: "0.0.0.0" });

  // ── GraphQL (Apollo Server v5 + graphql-ws) ────────────────────────────────
  const typeDefs = readFileSync(join(__dirname, "schema.graphql"), "utf-8");
  const schema = makeExecutableSchema({
    typeDefs,
    resolvers: buildResolvers(db, redis),
  });

  // Query depth and complexity limits (#779)
  const MAX_DEPTH = Number(process.env.GRAPHQL_MAX_DEPTH ?? 7);
  const MAX_COMPLEXITY = Number(process.env.GRAPHQL_MAX_COMPLEXITY ?? 1000);
  const complexityRule = createComplexityLimitRule(MAX_COMPLEXITY, {
    onCost: (cost) => logger.debug({ cost }, "graphql query complexity"),
  });

  const wsServer = new WebSocketServer({ noServer: true });
  const wsCleanup = useServer({ schema }, wsServer);

  const apollo = new ApolloServer({
    schema,
    introspection: true,
    plugins: [
      {
        async serverWillStart() {
          return {
            async drainServer() {
              await wsCleanup.dispose();
            },
          };
        },
      },
    ],
  });

  await apollo.start();

  // 4. HTTP server — handles both GraphQL POST/GET and WS upgrades on /graphql
  const httpServer = createServer(async (req: IncomingMessage, res: ServerResponse) => {
    if (req.url !== "/graphql") {
      res.writeHead(404);
      res.end("Not found");
      return;
    }

    // #778 — API key check
    if (!isAuthorized(req)) {
      res.writeHead(401, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ errors: [{ message: "Unauthorized: valid x-api-key header required" }] }));
      return;
    }

    const correlationId = (req.headers["x-correlation-id"] as string | undefined) ?? randomUUID();
    const reqLog = requestLogger(correlationId);

    const tracer = getTracer();
    const span = tracer.startSpan("graphql.request", {
      attributes: { "http.method": req.method ?? "GET", "correlation.id": correlationId },
    });

    const body = await readBody(req);
    const headers = new HeaderMap();
    for (const [key, value] of Object.entries(req.headers)) {
      if (value) headers.set(key, Array.isArray(value) ? value.join(", ") : value);
    }

    // #779 — depth + complexity validation before execution
    let parsedDocument: ReturnType<typeof import("graphql").parse> | undefined;
    if (body) {
      try {
        const parsed = JSON.parse(body) as { query?: string };
        if (parsed.query) {
          const { parse } = await import("graphql");
          parsedDocument = parse(parsed.query);
          const validationErrors = validate(schema, parsedDocument, [
            depthLimit(MAX_DEPTH),
            complexityRule,
          ]);
          if (validationErrors.length > 0) {
            span.end();
            res.writeHead(200, { "Content-Type": "application/json" });
            res.end(JSON.stringify({ errors: validationErrors.map((e) => ({ message: e.message })) }));
            return;
          }
        }
      } catch {
        // malformed JSON — let Apollo return its own parse error
      }
    }

    try {
      const result = await apollo.executeHTTPGraphQLRequest({
        httpGraphQLRequest: {
          method: req.method ?? "GET",
          headers,
          search: new URL(req.url ?? "/graphql", "http://localhost").search,
          body: body ? JSON.parse(body) : undefined,
        },
        context: async () => ({ db, correlationId, log: reqLog }),
      });

      res.writeHead(result.status ?? 200, {
        ...Object.fromEntries(result.headers),
        "x-correlation-id": correlationId,
      });

      if (result.body.kind === "complete") {
        res.end(result.body.string);
      } else {
        for await (const chunk of result.body.asyncIterator) {
          res.write(chunk);
        }
        res.end();
      }
    } finally {
      span.end();
    }
  });

  httpServer.on("upgrade", (req, socket, head) => {
    if (req.url === "/graphql") {
      wsServer.handleUpgrade(req, socket, head, (ws) => {
        wsServer.emit("connection", ws, req);
      });
    }
  });

  const GQL_PORT = Number(process.env.GQL_PORT ?? 4000);
  httpServer.listen(GQL_PORT, "0.0.0.0", () => {
    logger.info({ port: GQL_PORT }, "GraphQL endpoint listening");
  });

  // ── Indexer ────────────────────────────────────────────────────────────────
  startIndexer(db, redis).catch((err) => {
    console.error("Indexer error:", err);
    process.exit(1);
  });
}

main();
