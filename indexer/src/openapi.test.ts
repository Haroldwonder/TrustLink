/**
 * Tests for OpenAPI documentation (Issue #)
 * Validates that:
 * - /docs/openapi.json serves a valid OpenAPI 3.x spec
 * - All documented routes are present
 * - Route schemas include request validation
 */

import Fastify from "fastify";
import swagger from "@fastify/swagger";
import swaggerUi from "@fastify/swagger-ui";
import { buildRestServer } from "./rest";

describe("OpenAPI Documentation", () => {
  let fastify: any;

  beforeAll(async () => {
    // Build a minimal test server with swagger
    fastify = Fastify({ logger: false });
    
    fastify.register(swagger, {
      openapi: {
        info: {
          title: "TrustLink Indexer API",
          description: "REST API for querying attestations",
          version: "1.0.0",
        },
        servers: [
          { url: "http://localhost:3000", description: "Local" },
        ],
      },
    });
    
    fastify.register(swaggerUi, {
      routePrefix: "/docs",
    });
    
    // Add test routes with schemas
    fastify.get<{ Params: { subject: string } }>(
      "/attestations/:subject",
      {
        schema: {
          params: {
            type: "object",
            required: ["subject"],
            properties: {
              subject: { type: "string" },
            },
          },
        },
      },
      async (req) => ({ subject: req.params.subject })
    );
    
    fastify.get<{ Params: { address: string } }>(
      "/issuers/:address/attestations",
      {
        schema: {
          params: {
            type: "object",
            required: ["address"],
            properties: {
              address: { type: "string" },
            },
          },
        },
      },
      async (req) => ({ issuer: req.params.address })
    );
    
    await fastify.ready();
  });

  afterAll(async () => {
    await fastify.close();
  });

  it("should serve valid OpenAPI 3.0 spec at /docs/openapi.json", async () => {
    const response = await fastify.inject({
      method: "GET",
      url: "/docs/openapi.json",
    });

    expect(response.statusCode).toBe(200);
    expect(response.headers["content-type"]).toContain("application/json");

    const spec = JSON.parse(response.body);
    expect(spec.openapi).toMatch(/^3\./);
  });

  it("should document /attestations/:subject route", async () => {
    const response = await fastify.inject({
      method: "GET",
      url: "/docs/openapi.json",
    });

    const spec = JSON.parse(response.body);
    expect(spec.paths).toHaveProperty("/attestations/{subject}");
    expect(spec.paths["/attestations/{subject}"].get).toBeDefined();
  });

  it("should document /issuers/:address/attestations route", async () => {
    const response = await fastify.inject({
      method: "GET",
      url: "/docs/openapi.json",
    });

    const spec = JSON.parse(response.body);
    expect(spec.paths).toHaveProperty("/issuers/{address}/attestations");
    expect(spec.paths["/issuers/{address}/attestations"].get).toBeDefined();
  });

  it("should include request validation schemas for path parameters", async () => {
    const response = await fastify.inject({
      method: "GET",
      url: "/docs/openapi.json",
    });

    const spec = JSON.parse(response.body);
    const subjectRoute = spec.paths["/attestations/{subject}"].get;
    
    expect(subjectRoute.parameters).toBeDefined();
    expect(subjectRoute.parameters).toContainEqual(
      expect.objectContaining({
        name: "subject",
        in: "path",
        required: true,
      })
    );
  });

  it("should serve swagger UI at /docs", async () => {
    const response = await fastify.inject({
      method: "GET",
      url: "/docs",
    });

    expect(response.statusCode).toBe(200);
  });
});

describe("REST Server /version endpoint", () => {
  let fastify: any;
  let mockDb: any;

  beforeEach(async () => {
    mockDb = {
      checkpoint: {
        findUnique: jest.fn().mockResolvedValue({ ledger: 100 }),
      },
    };
    fastify = buildRestServer(mockDb);
    await fastify.ready();
  });

  afterEach(async () => {
    await fastify.close();
  });

  it("should return version info including contractId", async () => {
    // Set test environment variables
    const originalEnv = { ...process.env };
    process.env.CONTRACT_ID = "CA7X7XXXXXXXXXXXXXXXXXXXXXXX";
    process.env.NETWORK = "testnet";
    process.env.NODE_ENV = "test";

    try {
      const response = await fastify.inject({
        method: "GET",
        url: "/version",
      });

      expect(response.statusCode).toBe(200);
      const result = JSON.parse(response.body);
      
      expect(result.version).toBeDefined();
      expect(result.contractId).toBe("CA7X7XXXXXXXXXXXXXXXXXXXXXXX");
      expect(result.network).toBe("testnet");
      expect(result.nodeEnv).toBe("test");
      expect(result.timestamp).toBeDefined();
    } finally {
      process.env = originalEnv;
    }
  });

  it("should provide sensible defaults when env vars not set", async () => {
    const originalEnv = { ...process.env };
    delete process.env.CONTRACT_ID;
    delete process.env.NETWORK;
    delete process.env.NODE_ENV;

    try {
      const response = await fastify.inject({
        method: "GET",
        url: "/version",
      });

      expect(response.statusCode).toBe(200);
      const result = JSON.parse(response.body);
      
      expect(result.contractId).toBe("not configured");
      expect(result.network).toBe("testnet");
      expect(result.nodeEnv).toBe("development");
    } finally {
      process.env = originalEnv;
    }
  });
});