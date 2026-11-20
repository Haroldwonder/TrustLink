/**
 * GraphQL HTTP + WebSocket glue for the indexer.
 *
 * The request handler and the WebSocket upgrade path used to live inline in
 * `index.ts`, which made them impossible to unit-test. They are extracted here
 * (issues #1687 and #1689) so behaviour can be asserted directly.
 *
 * Both the HTTP handler and the WS upgrade enforce the same `x-api-key` auth
 * (via `auth.ts`), so GraphQL subscriptions can no longer bypass the API-key
 * check that HTTP queries already required.
 */
import { randomUUID } from "crypto";
import type { IncomingMessage, ServerResponse } from "http";
import type { Socket } from "net";
import { ApolloServer, HeaderMap } from "@apollo/server";
import type { GraphQLSchema, ValidationRule } from "graphql";
import { parse, validate } from "graphql";
import depthLimit from "graphql-depth-limit";
import { createComplexityRule, simpleEstimator } from "graphql-query-complexity";
import type { WebSocket, WebSocketServer } from "ws";
import { isAuthorized } from "./auth";
import { logger, requestLogger } from "./logger";
import { getTracer } from "./tracing";

export interface GraphqlHandlerParts {
  apollo: ApolloServer;
  schema: GraphQLSchema;
  /** Maximum query depth (#779). */
  maxDepth: number;
  /** Maximum query complexity (#779). */
  maxComplexity: number;
  /**
   * Build the per-request resolver context. Receives the correlation id and
   * the request-scoped child logger. Defaults to `{ correlationId, log }`;
   * `index.ts` overrides it to also inject `db`.
   */
  buildContext?: (
    correlationId: string,
    log: ReturnType<typeof requestLogger>,
  ) => Record<string, unknown> | Promise<Record<string, unknown>>;
}

const UNAUTHORIZED = "Unauthorized: valid x-api-key header required";

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    let body = "";
    req.on("data", (chunk) => (body += chunk));
    req.on("end", () => resolve(body));
    req.on("error", reject);
  });
}

/** Build the depth + complexity validation rules for the configured limits. */
function buildValidationRules(parts: GraphqlHandlerParts): ReadonlyArray<ValidationRule> {
  return [
    depthLimit(parts.maxDepth),
    createComplexityRule({
      maximumComplexity: parts.maxComplexity,
      estimators: [simpleEstimator()],
      onComplete: (cost) => logger.debug({ cost }, "graphql query complexity"),
    }),
  ] as ReadonlyArray<ValidationRule>;
}

/**
 * Create the GraphQL-over-HTTP request handler.
 *
 * The body is parsed exactly once; malformed JSON is a 400 with a
 * GraphQL-style error instead of being re-parsed inside the Apollo call where
 * a throw would reject unhandled (#1689). The whole handler is wrapped in a
 * catch that returns 500 so a failure never becomes an unhandled rejection.
 */
export function createGraphqlHttpHandler(parts: GraphqlHandlerParts) {
  const validationRules = buildValidationRules(parts);
  const buildContext =
    parts.buildContext ?? ((correlationId, log) => ({ correlationId, log }));

  return async (req: IncomingMessage, res: ServerResponse): Promise<void> => {
    if (req.url !== "/graphql") {
      res.writeHead(404);
      res.end("Not found");
      return;
    }

    if (!isAuthorized(req)) {
      res.writeHead(401, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ errors: [{ message: UNAUTHORIZED }] }));
      return;
    }

    const correlationId =
      (req.headers["x-correlation-id"] as string | undefined) ?? randomUUID();
    const reqLog = requestLogger(correlationId);
    const span = getTracer().startSpan("graphql.request", {
      attributes: {
        "http.method": req.method ?? "GET",
        "correlation.id": correlationId,
      },
    });

    try {
      const body = await readBody(req);
      const headers = new HeaderMap();
      for (const [key, value] of Object.entries(req.headers)) {
        if (value) {
          headers.set(key, Array.isArray(value) ? value.join(", ") : value);
        }
      }

      // Parse the body once. Malformed JSON is a client error (#1689): respond
      // 400 with a GraphQL-style error rather than letting Apollo re-parse.
      let parsedBody: unknown = undefined;
      if (body) {
        try {
          parsedBody = JSON.parse(body);
        } catch {
          res.writeHead(400, {
            "Content-Type": "application/json",
            "x-correlation-id": correlationId,
          });
          res.end(
            JSON.stringify({ errors: [{ message: "Request body is not valid JSON." }] }),
          );
          return;
        }
      }

      // Depth + complexity validation before execution (#779). Invalid GraphQL
      // syntax is left for Apollo to report; only JSON parse failures are 400.
      const query = (parsedBody as { query?: unknown } | null | undefined)?.query;
      if (typeof query === "string") {
        try {
          const document = parse(query);
          const validationErrors = validate(parts.schema, document, validationRules);
          if (validationErrors.length > 0) {
            res.writeHead(200, {
              "Content-Type": "application/json",
              "x-correlation-id": correlationId,
            });
            res.end(
              JSON.stringify({
                errors: validationErrors.map((e) => ({ message: e.message })),
              }),
            );
            return;
          }
        } catch {
          // malformed GraphQL query — let Apollo return its own parse error
        }
      }

      const result = await parts.apollo.executeHTTPGraphQLRequest({
        httpGraphQLRequest: {
          method: req.method ?? "GET",
          headers,
          search: new URL(req.url ?? "/graphql", "http://localhost").search,
          body: parsedBody,
        },
        context: async () => buildContext(correlationId, reqLog),
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
    } catch (err) {
      logger.error({ err }, "graphql request failed");
      if (!res.headersSent) {
        res.writeHead(500, {
          "Content-Type": "application/json",
          "x-correlation-id": correlationId,
        });
        res.end(JSON.stringify({ errors: [{ message: "Internal server error" }] }));
      }
    } finally {
      span.end();
    }
  };
}

/**
 * Create the WebSocket upgrade handler for GraphQL subscriptions.
 *
 * Applies the same `x-api-key` auth as the HTTP handler (#1687): an
 * unauthorized upgrade is answered with HTTP 401 and the socket is closed
 * before `wsServer.handleUpgrade` is reached.
 */
export function createGraphqlUpgradeHandler(wsServer: WebSocketServer) {
  return (req: IncomingMessage, socket: Socket, head: Buffer): void => {
    if (req.url !== "/graphql") {
      // Reject WS upgrades for any other path; without this the socket stays
      // open indefinitely, leaking file descriptors.
      socket.write("HTTP/1.1 404 Not Found\r\n\r\n");
      socket.destroy();
      return;
    }

    if (!isAuthorized(req)) {
      socket.write(
        "HTTP/1.1 401 Unauthorized\r\n" +
          "Content-Type: application/json\r\n" +
          "Connection: close\r\n\r\n" +
          JSON.stringify({ errors: [{ message: UNAUTHORIZED }] }),
      );
      socket.destroy();
      return;
    }

    wsServer.handleUpgrade(req, socket, head, (ws: WebSocket) => {
      wsServer.emit("connection", ws, req);
    });
  };
}
