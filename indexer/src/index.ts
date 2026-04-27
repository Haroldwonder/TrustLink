import { PrismaClient } from "@prisma/client";
import { createServer, IncomingMessage, ServerResponse } from "http";
import Fastify from "fastify";
import { ApolloServer, HeaderMap } from "@apollo/server";
import { makeExecutableSchema } from "@graphql-tools/schema";
import { WebSocketServer } from "ws";
import { useServer } from "graphql-ws/use/ws";
import { readFileSync } from "fs";
import { join } from "path";
import { startIndexer, getLastLedger } from "./indexer";
import { buildResolvers } from "./graphql";
import { getMetrics } from "./metrics";
const db = new PrismaClient();

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

  fastify.get("/health", async () => {
    let dbConnected = false;
    try {
      await db.$queryRaw`SELECT 1`;
      dbConnected = true;
    } catch {
      dbConnected = false;
    }
    return {
      status: "ok",
      lastLedger: getLastLedger(),
      dbConnected,
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

  // GET /subjects/:address/claims/:claim_type/valid - Check if subject has valid claim
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

  // GET /issuers/:address/attestations - Get all attestations issued by an issuer
  fastify.get<{ Params: { address: string } }>(
    "/issuers/:address/attestations",
    async (req) => {
      return db.attestation.findMany({
        where: { issuer: req.params.address },
        orderBy: { timestamp: "desc" },
      });
    }
  );

  // GET /stats - Get global statistics
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

  // ── Webhook management ─────────────────────────────────────────────────────

  // GET /webhooks - List all registered webhooks (secrets redacted)
  fastify.get("/webhooks", async () => {
    const webhooks = await db.webhook.findMany({
      select: { id: true, url: true, active: true, createdAt: true },
      orderBy: { createdAt: "desc" },
    });
    return webhooks;
  });

  // POST /webhooks - Register a new webhook
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

  // DELETE /webhooks/:id - Remove a webhook
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

  const REST_PORT = Number(process.env.PORT ?? 3000);
  await fastify.listen({ port: REST_PORT, host: "0.0.0.0" });

  // ── GraphQL (Apollo Server v5 + graphql-ws) ────────────────────────────────
  const typeDefs = readFileSync(join(__dirname, "schema.graphql"), "utf-8");
  const schema = makeExecutableSchema({
    typeDefs,
    resolvers: buildResolvers(db),
  });

  // 1. Create WS server (noServer — we handle the upgrade event manually)
  const wsServer = new WebSocketServer({ noServer: true });

  // 2. Wire graphql-ws onto the WS server
  const wsCleanup = useServer({ schema }, wsServer);

  // 3. Build and start Apollo (plugin references wsCleanup via closure — already assigned)
  const apollo = new ApolloServer({
    schema,
    introspection: true, // enables Apollo Sandbox at /graphql in development
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

    const body = await readBody(req);
    const headers = new HeaderMap();
    for (const [key, value] of Object.entries(req.headers)) {
      if (value) headers.set(key, Array.isArray(value) ? value.join(", ") : value);
    }

    const result = await apollo.executeHTTPGraphQLRequest({
      httpGraphQLRequest: {
        method: req.method ?? "GET",
        headers,
        search: new URL(req.url ?? "/graphql", "http://localhost").search,
        body: body ? JSON.parse(body) : undefined,
      },
      context: async () => ({ db }),
    });

    res.writeHead(result.status ?? 200, Object.fromEntries(result.headers));

    if (result.body.kind === "complete") {
      res.end(result.body.string);
    } else {
      for await (const chunk of result.body.asyncIterator) {
        res.write(chunk);
      }
      res.end();
    }
  });

  // Upgrade HTTP → WebSocket for subscriptions
  httpServer.on("upgrade", (req, socket, head) => {
    if (req.url === "/graphql") {
      wsServer.handleUpgrade(req, socket, head, (ws) => {
        wsServer.emit("connection", ws, req);
      });
    }
  });

  const GQL_PORT = Number(process.env.GQL_PORT ?? 4000);
  httpServer.listen(GQL_PORT, "0.0.0.0", () => {
    console.log(`GraphQL endpoint:   http://0.0.0.0:${GQL_PORT}/graphql`);
    console.log(`GraphQL Playground: http://localhost:${GQL_PORT}/graphql`);
    console.log(`Subscriptions:      ws://localhost:${GQL_PORT}/graphql`);
  });

  // ── Indexer ────────────────────────────────────────────────────────────────
  startIndexer(db).catch((err) => {
    console.error("Indexer error:", err);
    process.exit(1);
  });
}

main();
