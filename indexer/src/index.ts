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

  fastify.get<{ Params: { subject: string } }>(
    "/attestations/:subject",
    async (req) => {
      return db.attestation.findMany({
        where: { subject: req.params.subject },
        orderBy: { timestamp: "desc" },
      });
    }
  );

  fastify.get<{ Params: { issuer: string } }>(
    "/attestations/issuer/:issuer",
    async (req) => {
      return db.attestation.findMany({
        where: { issuer: req.params.issuer },
        orderBy: { timestamp: "desc" },
      });
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
