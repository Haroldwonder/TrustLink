import { describe, it, expect, beforeEach, vi } from "vitest";
import { PubSub } from "graphql-subscriptions";
import { pubsub, ATTESTATION_CREATED, ATTESTATION_REVOKED, ISSUER_REGISTERED } from "./graphql";

// Mock PrismaClient
const mockPrisma = {
  attestation: {
    findUnique: vi.fn(),
    upsert: vi.fn(),
    updateMany: vi.fn(),
    findMany: vi.fn(),
  },
  multisigProposal: {
    upsert: vi.fn(),
    update: vi.fn(),
    findUnique: vi.fn(),
    findMany: vi.fn(),
  },
  checkpoint: {
    findUnique: vi.fn(),
    upsert: vi.fn(),
  },
  issuer: {
    upsert: vi.fn(),
    update: vi.fn(),
    findUnique: vi.fn(),
    findMany: vi.fn(),
    count: vi.fn(),
  },
  webhook: {
    findMany: vi.fn(),
    create: vi.fn(),
    delete: vi.fn(),
  },
  $queryRaw: vi.fn().mockResolvedValue([]),
};

describe("GraphQL Subscriptions", () => {
  let testPubsub: PubSub;

  beforeEach(() => {
    testPubsub = new PubSub();
  });

  it("should publish ATTESTATION_CREATED events", async () => {
    const eventPayload = {
      id: "test-att-1",
      issuer: "GABC123",
      subject: "GDEF456",
      claimType: "KYC_PASSED",
      timestamp: "1700000000",
      expiration: null,
      isRevoked: false,
      metadata: null,
      imported: false,
      bridged: false,
      sourceChain: null,
      sourceTx: null,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    // Publish the event
    const publishPromise = testPubsub.publish(ATTESTATION_CREATED, {
      onAttestationCreated: eventPayload,
    });

    // Subscribe and collect the event
    const iterator = testPubsub.asyncIterableIterator(ATTESTATION_CREATED);
    
    // Wait for publish to complete
    await publishPromise;

    // Get the next value from iterator
    const result = await iterator.next();
    
    expect(result.done).toBe(false);
    expect(result.value).toHaveProperty("onAttestationCreated");
    expect(result.value.onAttestationCreated.id).toBe("test-att-1");
  });

  it("should publish ATTESTATION_REVOKED events", async () => {
    const eventPayload = {
      id: "test-att-1",
      issuer: "GABC123",
      revokedAt: new Date().toISOString(),
    };

    await testPubsub.publish(ATTESTATION_REVOKED, {
      onAttestationRevoked: eventPayload,
    });

    const iterator = testPubsub.asyncIterableIterator(ATTESTATION_REVOKED);
    const result = await iterator.next();

    expect(result.done).toBe(false);
    expect(result.value.onAttestationRevoked.id).toBe("test-att-1");
  });

  it("should publish ISSUER_REGISTERED events", async () => {
    const eventPayload = {
      issuer: "GABC123",
      registeredAt: new Date().toISOString(),
    };

    await testPubsub.publish(ISSUER_REGISTERED, {
      onIssuerRegistered: eventPayload,
    });

    const iterator = testPubsub.asyncIterableIterator(ISSUER_REGISTERED);
    const result = await iterator.next();

    expect(result.done).toBe(false);
    expect(result.value.onIssuerRegistered.issuer).toBe("GABC123");
  });

  it("should filter subscriptions by subject", async () => {
    const eventPayload1 = {
      id: "test-att-1",
      issuer: "GABC123",
      subject: "GDEF456",
      claimType: "KYC_PASSED",
      timestamp: "1700000000",
      expiration: null,
      isRevoked: false,
      metadata: null,
      imported: false,
      bridged: false,
      sourceChain: null,
      sourceTx: null,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    const eventPayload2 = {
      ...eventPayload1,
      id: "test-att-2",
      subject: "GHIJ789", // different subject
    };

    // Publish both events
    await testPubsub.publish(ATTESTATION_CREATED, {
      onAttestationCreated: eventPayload1,
    });
    await testPubsub.publish(ATTESTATION_CREATED, {
      onAttestationCreated: eventPayload2,
    });

    // Filter by subject GDEF456
    const targetSubject = "GDEF456";
    const iter = testPubsub.asyncIterableIterator<{
      onAttestationCreated: typeof eventPayload1;
    }>(ATTESTATION_CREATED);

    // Get first event (should be GDEF456)
    const result1 = await iter.next();
    expect(result1.value?.onAttestationCreated.subject).toBe(targetSubject);

    // Get second event - should skip GHIJ789 since we filter
    const result2 = await iter.next();
    expect(result2.value?.onAttestationCreated.subject).toBe(targetSubject);
  });
});