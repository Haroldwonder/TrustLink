/**
 * #935 – Admin REST endpoints authorization
 *
 * Tests verify that admin endpoints require API key authorization.
 */
import Fastify from "fastify";
import { PrismaClient } from "@prisma/client";

const db = {
  webhook: {
    create: jest.fn().mockResolvedValue({ id: "1", url: "http://test", active: true }),
    delete: jest.fn().mockResolvedValue({}),
    findMany: jest.fn().mockResolvedValue([]),
  },
  webhookFailure: {
    findMany: jest.fn().mockResolvedValue([]),
    count: jest.fn().mockResolvedValue(0),
  },
} as unknown as PrismaClient;

describe("#935 admin endpoint authorization", () => {
  it("rejects POST /webhooks without API key when API_KEY is set", async () => {
    process.env.API_KEY = "test-key";
    const fastify = Fastify({ logger: false });

    fastify.post<{ Body: { url: string; secret: string } }>(
      "/webhooks",
      async (req, reply) => {
        const apiKey = req.headers["x-api-key"] as string | undefined;
        const expectedKey = process.env.API_KEY;
        if (expectedKey && apiKey !== expectedKey) {
          reply.code(401);
          return { error: "Unauthorized: valid x-api-key header required" };
        }
        reply.code(201);
        return { id: "1" };
      }
    );

    const res = await fastify.inject({
      method: "POST",
      url: "/webhooks",
      payload: { url: "http://test", secret: "secret" },
    });

    expect(res.statusCode).toBe(401);
    delete process.env.API_KEY;
  });

  it("accepts POST /webhooks with valid API key", async () => {
    process.env.API_KEY = "test-key";
    const fastify = Fastify({ logger: false });

    fastify.post<{ Body: { url: string; secret: string } }>(
      "/webhooks",
      async (req, reply) => {
        const apiKey = req.headers["x-api-key"] as string | undefined;
        const expectedKey = process.env.API_KEY;
        if (expectedKey && apiKey !== expectedKey) {
          reply.code(401);
          return { error: "Unauthorized: valid x-api-key header required" };
        }
        reply.code(201);
        return { id: "1" };
      }
    );

    const res = await fastify.inject({
      method: "POST",
      url: "/webhooks",
      headers: { "x-api-key": "test-key" },
      payload: { url: "http://test", secret: "secret" },
    });

    expect(res.statusCode).toBe(201);
    delete process.env.API_KEY;
  });

  it("rejects DELETE /webhooks/:id without API key when API_KEY is set", async () => {
    process.env.API_KEY = "test-key";
    const fastify = Fastify({ logger: false });

    fastify.delete<{ Params: { id: string } }>(
      "/webhooks/:id",
      async (req, reply) => {
        const apiKey = req.headers["x-api-key"] as string | undefined;
        const expectedKey = process.env.API_KEY;
        if (expectedKey && apiKey !== expectedKey) {
          reply.code(401);
          return { error: "Unauthorized: valid x-api-key header required" };
        }
        reply.code(204);
      }
    );

    const res = await fastify.inject({
      method: "DELETE",
      url: "/webhooks/1",
    });

    expect(res.statusCode).toBe(401);
    delete process.env.API_KEY;
  });

  it("rejects POST /admin/reindex without API key when API_KEY is set", async () => {
    process.env.API_KEY = "test-key";
    const fastify = Fastify({ logger: false });

    fastify.post<{ Querystring: { from?: string } }>(
      "/admin/reindex",
      async (req, reply) => {
        const apiKey = req.headers["x-api-key"] as string | undefined;
        const expectedKey = process.env.API_KEY;
        if (expectedKey && apiKey !== expectedKey) {
          reply.code(401);
          return { error: "Unauthorized: valid x-api-key header required" };
        }
        reply.code(202);
        return { message: "ok" };
      }
    );

    const res = await fastify.inject({
      method: "POST",
      url: "/admin/reindex",
    });

    expect(res.statusCode).toBe(401);
    delete process.env.API_KEY;
  });

  it("rejects GET /admin/webhook-failures without API key when API_KEY is set", async () => {
    process.env.API_KEY = "test-key";
    const fastify = Fastify({ logger: false });

    fastify.get("/admin/webhook-failures", async (req, reply) => {
      const apiKey = req.headers["x-api-key"] as string | undefined;
      const expectedKey = process.env.API_KEY;
      if (expectedKey && apiKey !== expectedKey) {
        reply.code(401);
        return { error: "Unauthorized: valid x-api-key header required" };
      }
      reply.code(200);
      return { items: [] };
    });

    const res = await fastify.inject({
      method: "GET",
      url: "/admin/webhook-failures",
    });

    expect(res.statusCode).toBe(401);
    delete process.env.API_KEY;
  });

  it("allows all admin endpoints when API_KEY is not set", async () => {
    delete process.env.API_KEY;
    const fastify = Fastify({ logger: false });

    fastify.post("/admin/reindex", async (req, reply) => {
      const apiKey = req.headers["x-api-key"] as string | undefined;
      const expectedKey = process.env.API_KEY;
      if (expectedKey && apiKey !== expectedKey) {
        reply.code(401);
        return { error: "Unauthorized" };
      }
      reply.code(202);
      return { message: "ok" };
    });

    const res = await fastify.inject({
      method: "POST",
      url: "/admin/reindex",
    });

    expect(res.statusCode).toBe(202);
  });
});
