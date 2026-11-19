/**
 * #1687 / #1689 — tests for the extracted GraphQL HTTP + WebSocket handlers.
 *
 * `index.ts` used to inline these handlers, which made them impossible to
 * exercise. They now live in `graphql-server.ts` and are driven here against a
 * real `http.Server` + a trivial Apollo schema (no database required).
 */
import { createServer, type Server } from "http";
import { makeExecutableSchema } from "@graphql-tools/schema";
import { ApolloServer } from "@apollo/server";
import { WebSocketServer, WebSocket } from "ws";
import { createGraphqlHttpHandler, createGraphqlUpgradeHandler } from "./graphql-server";

const ORIGINAL_KEYS = process.env.GRAPHQL_API_KEYS;

afterAll(() => {
  if (ORIGINAL_KEYS === undefined) {
    delete process.env.GRAPHQL_API_KEYS;
  } else {
    process.env.GRAPHQL_API_KEYS = ORIGINAL_KEYS;
  }
});

/** Disable API-key auth for a test (no configured keys => auth disabled). */
function disableAuth() {
  delete process.env.GRAPHQL_API_KEYS;
}

function startHttpServer(handler: ReturnType<typeof createGraphqlHttpHandler>): Promise<Server> {
  const server = createServer(handler);
  return new Promise((resolve) => {
    server.listen(0, "127.0.0.1", () => resolve(server));
  });
}

function serverPort(server: Server): number {
  return (server.address() as { port: number }).port;
}

function close(server: Server): Promise<void> {
  return new Promise((resolve) => server.close(() => resolve()));
}

async function gqlPost(
  port: number,
  body: string,
  headers: Record<string, string> = {},
): Promise<{ status: number; json: unknown }> {
  const res = await fetch(`http://127.0.0.1:${port}/graphql`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...headers },
    body,
  });
  const text = await res.text();
  let json: unknown = null;
  try {
    json = JSON.parse(text);
  } catch {
    json = text;
  }
  return { status: res.status, json };
}

describe("createGraphqlHttpHandler — malformed JSON (#1689)", () => {
  let apollo: ApolloServer;
  let server: Server;

  beforeAll(async () => {
    disableAuth();
    const schema = makeExecutableSchema({
      typeDefs: "type Query { hello: String }",
      resolvers: { Query: { hello: () => "world" } },
    });
    apollo = new ApolloServer({ schema, introspection: true });
    await apollo.start();
    server = await startHttpServer(
      createGraphqlHttpHandler({ apollo, schema, maxDepth: 7, maxComplexity: 1000 }),
    );
  });

  afterAll(async () => {
    await close(server);
    await apollo.stop();
  });

  it("returns 400 with a GraphQL-style error for an invalid JSON body", async () => {
    const { status, json } = await gqlPost(serverPort(server), "{not valid json");
    expect(status).toBe(400);
    expect((json as { errors: { message: string }[] }).errors[0].message).toMatch(/JSON/i);
  });

  it("returns 200 with data for a valid query", async () => {
    const { status, json } = await gqlPost(
      serverPort(server),
      JSON.stringify({ query: "{ hello }" }),
    );
    expect(status).toBe(200);
    expect((json as { data: { hello: string } }).data.hello).toBe("world");
  });

  it("returns 404 for non-/graphql paths", async () => {
    const res = await fetch(`http://127.0.0.1:${serverPort(server)}/nope`, {
      method: "POST",
      body: "{}",
    });
    expect(res.status).toBe(404);
  });
});

describe("createGraphqlHttpHandler — 500 on handler failure (#1689)", () => {
  let server: Server;

  beforeAll(async () => {
    disableAuth();
    const schema = makeExecutableSchema({
      typeDefs: "type Query { hello: String }",
      resolvers: { Query: { hello: () => "world" } },
    });
    // Apollo that always fails: exercises the outer catch that returns 500.
    const failingApollo = {
      executeHTTPGraphQLRequest: async () => {
        throw new Error("boom");
      },
    } as unknown as ApolloServer;
    server = await startHttpServer(
      createGraphqlHttpHandler({
        apollo: failingApollo,
        schema,
        maxDepth: 7,
        maxComplexity: 1000,
      }),
    );
  });

  afterAll(async () => {
    await close(server);
  });

  it("returns 500 instead of an unhandled rejection when execution throws", async () => {
    const { status, json } = await gqlPost(
      serverPort(server),
      JSON.stringify({ query: "{ hello }" }),
    );
    expect(status).toBe(500);
    expect((json as { errors: { message: string }[] }).errors[0].message).toMatch(/internal/i);
  });
});

describe("createGraphqlHttpHandler — API-key auth", () => {
  let apollo: ApolloServer;
  let server: Server;

  beforeAll(async () => {
    process.env.GRAPHQL_API_KEYS = "secret";
    const schema = makeExecutableSchema({
      typeDefs: "type Query { hello: String }",
      resolvers: { Query: { hello: () => "world" } },
    });
    apollo = new ApolloServer({ schema, introspection: true });
    await apollo.start();
    server = await startHttpServer(
      createGraphqlHttpHandler({ apollo, schema, maxDepth: 7, maxComplexity: 1000 }),
    );
  });

  afterAll(async () => {
    await close(server);
    await apollo.stop();
  });

  it("rejects requests without the x-api-key header with 401", async () => {
    const { status, json } = await gqlPost(serverPort(server), JSON.stringify({ query: "{ hello }" }));
    expect(status).toBe(401);
    expect((json as { errors: { message: string }[] }).errors[0].message).toMatch(/unauthorized/i);
  });

  it("accepts requests with a valid x-api-key header", async () => {
    const { status, json } = await gqlPost(
      serverPort(server),
      JSON.stringify({ query: "{ hello }" }),
      { "x-api-key": "secret" },
    );
    expect(status).toBe(200);
    expect((json as { data: { hello: string } }).data.hello).toBe("world");
  });
});

describe("createGraphqlUpgradeHandler — WS auth (#1687)", () => {
  let server: Server;
  let wsServer: WebSocketServer;

  function wsUpgrade(url: string, headers: Record<string, string>): Promise<{ open: boolean; status?: number }> {
    return new Promise((resolve) => {
      const ws = new WebSocket(url, [], { headers });
      let done = false;
      let timer: ReturnType<typeof setTimeout>;
      const finish = (v: { open: boolean; status?: number }) => {
        if (done) return;
        done = true;
        clearTimeout(timer);
        try {
          ws.close();
        } catch {
          /* ignore */
        }
        resolve(v);
      };
      timer = setTimeout(() => finish({ open: false }), 3000);
      ws.on("open", () => finish({ open: true }));
      ws.on("unexpected-response", (_req, res) => finish({ open: false, status: res.statusCode }));
      ws.on("error", () => finish({ open: false }));
    });
  }

  beforeAll(() => {
    process.env.GRAPHQL_API_KEYS = "secret";
    wsServer = new WebSocketServer({ noServer: true });
    server = createServer((_req, res) => {
      res.writeHead(404);
      res.end();
    });
    server.on("upgrade", createGraphqlUpgradeHandler(wsServer));
    return new Promise<void>((resolve) => server.listen(0, "127.0.0.1", () => resolve()));
  });

  afterAll(async () => {
    await close(server);
    await new Promise<void>((resolve) => wsServer.close(() => resolve()));
  });

  it("rejects WS upgrades without the x-api-key header (no 101)", async () => {
    const port = serverPort(server);
    const result = await wsUpgrade(`ws://127.0.0.1:${port}/graphql`, {});
    expect(result.open).toBe(false);
    expect(result.status).toBe(401);
  });

  it("accepts WS upgrades with a valid x-api-key header (101)", async () => {
    const port = serverPort(server);
    const result = await wsUpgrade(`ws://127.0.0.1:${port}/graphql`, { "x-api-key": "secret" });
    expect(result.open).toBe(true);
  });
});
