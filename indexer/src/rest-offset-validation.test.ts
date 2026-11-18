/**
 * Regression tests for Issue #1702:
 * GET /admin/webhook-failures must reject negative / non-integer offsets with 400
 * instead of forwarding them to Prisma skip (which throws and yields a 500).
 */

jest.mock("@prisma/client", () => ({
  PrismaClient: jest.fn(),
}));

import { buildRestServer } from "./rest";

function makeMockDb() {
  return {
    webhookFailure: {
      findMany: jest.fn().mockResolvedValue([]),
      count: jest.fn().mockResolvedValue(0),
    },
  };
}

describe("GET /admin/webhook-failures offset validation (Issue #1702)", () => {
  let db: ReturnType<typeof makeMockDb>;
  let fastify: ReturnType<typeof buildRestServer>;

  beforeEach(async () => {
    db = makeMockDb();
    fastify = buildRestServer(db as never);
    await fastify.ready();
  });

  afterEach(async () => {
    await fastify.close();
  });

  const invalidOffsets = ["-5", "-1", "1.5", "abc", "1e3", " 3 "];

  it.each(invalidOffsets)("returns 400 for invalid offset %p", async (offset) => {
    const response = await fastify.inject({
      method: "GET",
      url: `/admin/webhook-failures?offset=${encodeURIComponent(offset)}`,
    });
    expect(response.statusCode).toBe(400);
    expect(db.webhookFailure.findMany).not.toHaveBeenCalled();
  });

  it("accepts a valid non-negative integer offset", async () => {
    const response = await fastify.inject({
      method: "GET",
      url: "/admin/webhook-failures?offset=10",
    });
    expect(response.statusCode).toBe(200);
    expect(db.webhookFailure.findMany).toHaveBeenCalledWith(
      expect.objectContaining({ skip: 10 }),
    );
  });

  it("defaults to offset 0 when omitted", async () => {
    const response = await fastify.inject({
      method: "GET",
      url: "/admin/webhook-failures",
    });
    expect(response.statusCode).toBe(200);
    expect(db.webhookFailure.findMany).toHaveBeenCalledWith(
      expect.objectContaining({ skip: 0 }),
    );
  });
});
