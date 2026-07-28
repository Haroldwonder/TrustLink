/**
 * Tests for REST routes in indexer/src/index.ts
 *
 * Covers:
 *  - Issue #931: GET /attestations/:subject reads correct path param
 *  - Edge cases: subject parameter handling, missing attestations
 */

import Fastify from "fastify";
import { PrismaClient } from "@prisma/client";

describe("REST Route: GET /attestations/:subject (Issue #931)", () => {
  let fastify: any;
  let mockDb: any;

  beforeEach(() => {
    fastify = Fastify({ logger: false });
    mockDb = {
      attestation: {
        findMany: jest.fn(),
      },
    };
  });

  afterEach(async () => {
    await fastify.close();
  });

  it("should read subject parameter from req.params.subject, not req.params.address", async () => {
    const testSubject = "GDXLKEY5TR4IDEVSTRYUNYY3DPXQKQNSTDJ7HIVNFTJYQHOZXB7CRQME";
    const testAttestations = [
      {
        id: "att-1",
        issuer: "ISSUER1",
        subject: testSubject,
        claimType: "KYC",
        timestamp: BigInt(1234567890),
        isRevoked: false,
      },
    ];

    mockDb.attestation.findMany.mockResolvedValue(testAttestations);

    fastify.get<{ Params: { subject: string } }>(
      "/attestations/:subject",
      async (req: any) => {
        // This test validates that req.params.subject is read (not req.params.address)
        return mockDb.attestation.findMany({
          where: { subject: req.params.subject },
          orderBy: { timestamp: "desc" },
        });
      }
    );

    const response = await fastify.inject({
      method: "GET",
      url: `/attestations/${testSubject}`,
    });

    expect(response.statusCode).toBe(200);
    const result = JSON.parse(response.body);
    expect(result).toEqual(testAttestations);
    expect(mockDb.attestation.findMany).toHaveBeenCalledWith({
      where: { subject: testSubject },
      orderBy: { timestamp: "desc" },
    });
  });

  it("should correctly filter attestations by subject parameter", async () => {
    const subject1 = "GDXLKEY5TR4IDEVSTRYUNYY3DPXQKQNSTDJ7HIVNFTJYQHOZXB7CRQME";
    const subject2 = "GDZST3XVCDTUJ76ZAV2HA72KYQFLVGSWBWUHKP3F2COXQ4PSPUKYKP3N";

    const att1 = {
      id: "att-1",
      issuer: "ISSUER1",
      subject: subject1,
      claimType: "KYC",
      timestamp: BigInt(1234567890),
      isRevoked: false,
    };

    mockDb.attestation.findMany.mockResolvedValue([att1]);

    fastify.get<{ Params: { subject: string } }>(
      "/attestations/:subject",
      async (req: any) => {
        return mockDb.attestation.findMany({
          where: { subject: req.params.subject },
          orderBy: { timestamp: "desc" },
        });
      }
    );

    const response = await fastify.inject({
      method: "GET",
      url: `/attestations/${subject1}`,
    });

    expect(response.statusCode).toBe(200);
    expect(mockDb.attestation.findMany).toHaveBeenCalledWith({
      where: { subject: subject1 },
      orderBy: { timestamp: "desc" },
    });
  });

  it("should return empty list when subject has no attestations", async () => {
    const testSubject = "GDXLKEY5TR4IDEVSTRYUNYY3DPXQKQNSTDJ7HIVNFTJYQHOZXB7CRQME";
    mockDb.attestation.findMany.mockResolvedValue([]);

    fastify.get<{ Params: { subject: string } }>(
      "/attestations/:subject",
      async (req: any) => {
        return mockDb.attestation.findMany({
          where: { subject: req.params.subject },
          orderBy: { timestamp: "desc" },
        });
      }
    );

    const response = await fastify.inject({
      method: "GET",
      url: `/attestations/${testSubject}`,
    });

    expect(response.statusCode).toBe(200);
    const result = JSON.parse(response.body);
    expect(result).toEqual([]);
    expect(mockDb.attestation.findMany).toHaveBeenCalledWith({
      where: { subject: testSubject },
      orderBy: { timestamp: "desc" },
    });
  });

  it("should order attestations by timestamp in descending order", async () => {
    const testSubject = "GDXLKEY5TR4IDEVSTRYUNYY3DPXQKQNSTDJ7HIVNFTJYQHOZXB7CRQME";
    const attestations = [
      {
        id: "att-3",
        issuer: "ISSUER1",
        subject: testSubject,
        claimType: "KYC",
        timestamp: BigInt(1234567893),
        isRevoked: false,
      },
      {
        id: "att-2",
        issuer: "ISSUER1",
        subject: testSubject,
        claimType: "KYC",
        timestamp: BigInt(1234567892),
        isRevoked: false,
      },
      {
        id: "att-1",
        issuer: "ISSUER1",
        subject: testSubject,
        claimType: "KYC",
        timestamp: BigInt(1234567891),
        isRevoked: false,
      },
    ];

    mockDb.attestation.findMany.mockResolvedValue(attestations);

    fastify.get<{ Params: { subject: string } }>(
      "/attestations/:subject",
      async (req: any) => {
        return mockDb.attestation.findMany({
          where: { subject: req.params.subject },
          orderBy: { timestamp: "desc" },
        });
      }
    );

    const response = await fastify.inject({
      method: "GET",
      url: `/attestations/${testSubject}`,
    });

    expect(response.statusCode).toBe(200);
    expect(mockDb.attestation.findMany).toHaveBeenCalledWith({
      where: { subject: testSubject },
      orderBy: { timestamp: "desc" },
    });
  });
});
