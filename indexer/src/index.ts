import { initTracing } from "./tracing";
initTracing(); // must be first — instruments http, pg, etc.

import { PrismaClient } from "@prisma/client";
import { createServer } from "http";
import { ApolloServer } from "@apollo/server";
import { makeExecutableSchema } from "@graphql-tools/schema";
import { WebSocketServer } from "ws";
import { useServer } from "graphql-ws/use/ws";
import { readFileSync } from "fs";
import { join } from "path";
import { startIndexer } from "./indexer";
import { buildResolvers } from "./graphql";
import Redis from "ioredis";
import { logger } from "./logger";
import { buildRestServer } from "./rest";
import { createGraphqlHttpHandler, createGraphqlUpgradeHandler } from "./graphql-server";

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

async function main() {
  await db.$connect();

  // ── REST (Fastify) ─────────────────────────────────────────────────────────
  const fastify = buildRestServer(db);
  const REST_PORT = Number(process.env.PORT ?? 3000);
  await fastify.listen({ port: REST_PORT, host: "0.0.0.0" });

  // ── GraphQL (Apollo Server v5 + graphql-ws) ────────────────────────────────
  const typeDefs = readFileSync(join(__dirname, "schema.graphql"), "utf-8");
  const schema = makeExecutableSchema({
    typeDefs,
    resolvers: buildResolvers(db, redis),
  });

  const MAX_DEPTH = Number(process.env.GRAPHQL_MAX_DEPTH ?? 7);
  const MAX_COMPLEXITY = Number(process.env.GRAPHQL_MAX_COMPLEXITY ?? 1000);

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

  // HTTP + WebSocket handlers: API-key auth and body parsing live in
  // graphql-server.ts (#1687, #1689) so they can be unit-tested.
  const httpServer = createServer(
    createGraphqlHttpHandler({
      apollo,
      schema,
      maxDepth: MAX_DEPTH,
      maxComplexity: MAX_COMPLEXITY,
      buildContext: async (correlationId, log) => ({ db, correlationId, log }),
    }),
  );

  httpServer.on("upgrade", createGraphqlUpgradeHandler(wsServer));

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
