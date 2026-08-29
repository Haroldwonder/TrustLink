/**
 * Integration-style test for the process entrypoint (index.ts).
 *
 * index.ts wires the REST server via `buildRestServer(db)` and exposes it on
 * PORT. This test boots that same server factory with a mocked database and
 * asserts the HTTP server comes up and answers the health check.
 */
import { buildRestServer } from "./rest";

describe("indexer entrypoint: REST server boot", () => {
  let server: ReturnType<typeof buildRestServer>;

  afterEach(async () => {
    if (server) await server.close();
  });

  it("boots and responds ok on GET /health when the db is reachable", async () => {
    const db = { $queryRaw: jest.fn().mockResolvedValue([{ "?column?": 1 }]) } as any;
    server = buildRestServer(db);
    await server.ready();

    const res = await server.inject({ method: "GET", url: "/health" });

    expect(res.statusCode).toBe(200);
    expect(res.json()).toMatchObject({ status: "ok", db: "connected" });
  });

  it("reports 503 on GET /health when the db is unreachable", async () => {
    const db = { $queryRaw: jest.fn().mockRejectedValue(new Error("no db")) } as any;
    server = buildRestServer(db);
    await server.ready();

    const res = await server.inject({ method: "GET", url: "/health" });

    expect(res.statusCode).toBe(503);
    expect(res.json()).toMatchObject({ status: "error", db: "disconnected" });
  });
});
