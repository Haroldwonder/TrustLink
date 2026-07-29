import { initTracing, getTracer } from "./tracing";
initTracing(); // must be first — instruments http, pg, etc.

import { randomUUID } from "crypto";
import { PrismaClient } from "@prisma/client";
import { createServer, IncomingMessage, ServerResponse } from "http";
import { ApolloServer, HeaderMap } from "@apollo/server";
import { makeExecutableSchema } from "@graphql-tools/schema";
import { WebSocketServer } from "ws";
import { useServer } from "graphql-ws/use/ws";
import { readFileSync } from "fs";
import { join } from "path";
import { validate } from "graphql";
import depthLimit from "graphql-depth-limit";
import { createComplexityLimitRule } from "graphql-query-complexity";
import { startIndexer } from "./indexer";
import { buildResolvers } from "./graphql";
import Redis from "ioredis";
import { isAuthorized } from "./auth";
import { logger, requestLogger } from "./logger";
import { buildRestServer } from "./rest";

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
  const fastify = buildRestServer(db);
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
