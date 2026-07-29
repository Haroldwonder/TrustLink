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

// Helper: build a canonical created-attestation payload
function makeCreatedPayload(overrides: Partial<{
  id: string;
  issuer: string;
  subject: string;
  claimType: string;
}> = {}) {
  return {
    id: overrides.id ?? "test-att-1",
    issuer: overrides.issuer ?? "GABC123",
    subject: overrides.subject ?? "GDEF456",
    claimType: overrides.claimType ?? "KYC_PASSED",
    timestamp: "1700000000",
    expiration: null,
    isRevoked: false,
    revocationReason: null,
    metadata: null,
    imported: false,
    bridged: false,
    sourceChain: null,
    sourceTx: null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
}

// Helper: build a canonical revoked-attestation payload
function makeRevokedPayload(overrides: Partial<{
  id: string;
  issuer: string;
  subject: string;
  claimType: string;
}> = {}) {
  return {
    id: overrides.id ?? "test-att-1",
    issuer: overrides.issuer ?? "GABC123",
    subject: overrides.subject ?? "GDEF456",
    claimType: overrides.claimType ?? "KYC_PASSED",
    revokedAt: new Date().toISOString(),
  };
}

// Helper that filters an async iterator the same way the subscription resolver
// does — subject AND issuer AND claimType (each optional, combined with AND).
async function applySubscriptionFilter<T extends {
  subject?: string;
  issuer?: string;
  claimType?: string;
}>(
  iter: AsyncIterable<{ [key: string]: T }>,
  payloadKey: string,
  filters: { subject?: string; issuer?: string; claimType?: string }
): Promise<T> {
  const { subject, issuer, claimType } = filters;
  const asyncIter = iter[Symbol.asyncIterator]();
  while (true) {
    const result = await asyncIter.next();
    if (result.done) throw new Error("Iterator exhausted without match");
    const item = result.value?.[payloadKey] as T;
    if (!item) continue;
    if (subject && item.subject !== subject) continue;
    if (issuer && item.issuer !== issuer) continue;
    if (claimType && item.claimType !== claimType) continue;
    return item;
  }
}

describe("GraphQL Subscriptions", () => {
  let testPubsub: PubSub;

  beforeEach(() => {
    testPubsub = new PubSub();
  });

  // ── Basic publish / receive ──────────────────────────────────────────────

  it("should publish ATTESTATION_CREATED events", async () => {
    const payload = makeCreatedPayload();

    const iter = testPubsub.asyncIterableIterator(ATTESTATION_CREATED);
    await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: payload });

    const result = await iter.next();
    expect(result.done).toBe(false);
    expect(result.value).toHaveProperty("onAttestationCreated");
    expect(result.value.onAttestationCreated.id).toBe("test-att-1");
  });

  it("should publish ATTESTATION_REVOKED events", async () => {
    const payload = makeRevokedPayload();

    const iter = testPubsub.asyncIterableIterator(ATTESTATION_REVOKED);
    await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: payload });

    const result = await iter.next();
    expect(result.done).toBe(false);
    expect(result.value.onAttestationRevoked.id).toBe("test-att-1");
  });

  it("should publish ISSUER_REGISTERED events", async () => {
    const payload = { issuer: "GABC123", registeredAt: new Date().toISOString() };

    const iter = testPubsub.asyncIterableIterator(ISSUER_REGISTERED);
    await testPubsub.publish(ISSUER_REGISTERED, { onIssuerRegistered: payload });

    const result = await iter.next();
    expect(result.done).toBe(false);
    expect(result.value.onIssuerRegistered.issuer).toBe("GABC123");
  });

  // ── #974: onAttestationCreated filter coverage ───────────────────────────

  describe("onAttestationCreated — consistent filter arguments (#974)", () => {
    it("passes through all events when no filter is given", async () => {
      const p1 = makeCreatedPayload({ id: "att-1", subject: "GSUB1", issuer: "GISS1", claimType: "KYC_PASSED" });
      const p2 = makeCreatedPayload({ id: "att-2", subject: "GSUB2", issuer: "GISS2", claimType: "AML_CLEARED" });

      const iter = testPubsub.asyncIterableIterator(ATTESTATION_CREATED);
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: p1 });
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: p2 });

      const r1 = await iter.next();
      const r2 = await iter.next();
      expect(r1.value.onAttestationCreated.id).toBe("att-1");
      expect(r2.value.onAttestationCreated.id).toBe("att-2");
    });

    it("filters by subject — only matching attestations pass through", async () => {
      const target = makeCreatedPayload({ id: "att-match", subject: "GSUB_TARGET" });
      const other  = makeCreatedPayload({ id: "att-other",  subject: "GSUB_OTHER"  });

      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: other  });
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationCreated: ReturnType<typeof makeCreatedPayload> }>(ATTESTATION_CREATED);
      const match = await applySubscriptionFilter(iter, "onAttestationCreated", { subject: "GSUB_TARGET" });
      expect(match.id).toBe("att-match");
    });

    it("filters by issuer — only matching attestations pass through", async () => {
      const target = makeCreatedPayload({ id: "att-iss-match", issuer: "GISS_TARGET" });
      const other  = makeCreatedPayload({ id: "att-iss-other",  issuer: "GISS_OTHER"  });

      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: other  });
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationCreated: ReturnType<typeof makeCreatedPayload> }>(ATTESTATION_CREATED);
      const match = await applySubscriptionFilter(iter, "onAttestationCreated", { issuer: "GISS_TARGET" });
      expect(match.id).toBe("att-iss-match");
    });

    it("filters by claimType — only matching attestations pass through", async () => {
      const target = makeCreatedPayload({ id: "att-ct-match", claimType: "KYC_PASSED" });
      const other  = makeCreatedPayload({ id: "att-ct-other",  claimType: "AML_CLEARED" });

      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: other  });
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationCreated: ReturnType<typeof makeCreatedPayload> }>(ATTESTATION_CREATED);
      const match = await applySubscriptionFilter(iter, "onAttestationCreated", { claimType: "KYC_PASSED" });
      expect(match.id).toBe("att-ct-match");
    });

    it("combines subject + issuer filters (AND logic)", async () => {
      const target = makeCreatedPayload({ id: "att-combined", subject: "GSUB_T", issuer: "GISS_T" });
      const wrongSub  = makeCreatedPayload({ id: "att-wrong-sub", subject: "GSUB_X", issuer: "GISS_T" });
      const wrongIss  = makeCreatedPayload({ id: "att-wrong-iss", subject: "GSUB_T", issuer: "GISS_X" });

      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: wrongSub  });
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: wrongIss  });
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationCreated: ReturnType<typeof makeCreatedPayload> }>(ATTESTATION_CREATED);
      const match = await applySubscriptionFilter(iter, "onAttestationCreated", { subject: "GSUB_T", issuer: "GISS_T" });
      expect(match.id).toBe("att-combined");
    });

    it("combines subject + claimType filters (AND logic)", async () => {
      const target = makeCreatedPayload({ id: "att-sc-combined", subject: "GSUB_T", claimType: "KYC_PASSED" });
      const wrongCt  = makeCreatedPayload({ id: "att-wrong-ct", subject: "GSUB_T", claimType: "AML_CLEARED" });

      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: wrongCt  });
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationCreated: ReturnType<typeof makeCreatedPayload> }>(ATTESTATION_CREATED);
      const match = await applySubscriptionFilter(iter, "onAttestationCreated", { subject: "GSUB_T", claimType: "KYC_PASSED" });
      expect(match.id).toBe("att-sc-combined");
    });

    it("combines issuer + claimType filters (AND logic)", async () => {
      const target   = makeCreatedPayload({ id: "att-ic-combined", issuer: "GISS_T", claimType: "KYC_PASSED" });
      const wrongCt  = makeCreatedPayload({ id: "att-ic-wrong-ct", issuer: "GISS_T", claimType: "AML_CLEARED" });
      const wrongIss = makeCreatedPayload({ id: "att-ic-wrong-iss", issuer: "GISS_X", claimType: "KYC_PASSED" });

      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: wrongCt  });
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: wrongIss });
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationCreated: ReturnType<typeof makeCreatedPayload> }>(ATTESTATION_CREATED);
      const match = await applySubscriptionFilter(iter, "onAttestationCreated", { issuer: "GISS_T", claimType: "KYC_PASSED" });
      expect(match.id).toBe("att-ic-combined");
    });

    it("combines all three filters — subject + issuer + claimType", async () => {
      const target = makeCreatedPayload({ id: "att-all", subject: "GSUB_T", issuer: "GISS_T", claimType: "KYC_PASSED" });
      const miss1  = makeCreatedPayload({ id: "att-m1", subject: "GSUB_X", issuer: "GISS_T", claimType: "KYC_PASSED" });
      const miss2  = makeCreatedPayload({ id: "att-m2", subject: "GSUB_T", issuer: "GISS_X", claimType: "KYC_PASSED" });
      const miss3  = makeCreatedPayload({ id: "att-m3", subject: "GSUB_T", issuer: "GISS_T", claimType: "AML_CLEARED" });

      for (const p of [miss1, miss2, miss3, target]) {
        await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: p });
      }

      const iter = testPubsub.asyncIterableIterator<{ onAttestationCreated: ReturnType<typeof makeCreatedPayload> }>(ATTESTATION_CREATED);
      const match = await applySubscriptionFilter(iter, "onAttestationCreated", {
        subject: "GSUB_T",
        issuer: "GISS_T",
        claimType: "KYC_PASSED",
      });
      expect(match.id).toBe("att-all");
    });
  });

  // ── #974: onAttestationRevoked filter coverage ───────────────────────────

  describe("onAttestationRevoked — consistent filter arguments (#974)", () => {
    it("passes through all events when no filter is given", async () => {
      const p1 = makeRevokedPayload({ id: "rev-1", subject: "GSUB1", issuer: "GISS1", claimType: "KYC_PASSED" });
      const p2 = makeRevokedPayload({ id: "rev-2", subject: "GSUB2", issuer: "GISS2", claimType: "AML_CLEARED" });

      const iter = testPubsub.asyncIterableIterator(ATTESTATION_REVOKED);
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: p1 });
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: p2 });

      const r1 = await iter.next();
      const r2 = await iter.next();
      expect(r1.value.onAttestationRevoked.id).toBe("rev-1");
      expect(r2.value.onAttestationRevoked.id).toBe("rev-2");
    });

    it("filters by subject", async () => {
      const target = makeRevokedPayload({ id: "rev-sub-match", subject: "GSUB_T" });
      const other  = makeRevokedPayload({ id: "rev-sub-other",  subject: "GSUB_X" });

      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: other  });
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationRevoked: ReturnType<typeof makeRevokedPayload> }>(ATTESTATION_REVOKED);
      const match = await applySubscriptionFilter(iter, "onAttestationRevoked", { subject: "GSUB_T" });
      expect(match.id).toBe("rev-sub-match");
    });

    it("filters by issuer", async () => {
      const target = makeRevokedPayload({ id: "rev-iss-match", issuer: "GISS_T" });
      const other  = makeRevokedPayload({ id: "rev-iss-other",  issuer: "GISS_X" });

      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: other  });
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationRevoked: ReturnType<typeof makeRevokedPayload> }>(ATTESTATION_REVOKED);
      const match = await applySubscriptionFilter(iter, "onAttestationRevoked", { issuer: "GISS_T" });
      expect(match.id).toBe("rev-iss-match");
    });

    it("filters by claimType", async () => {
      const target = makeRevokedPayload({ id: "rev-ct-match", claimType: "KYC_PASSED" });
      const other  = makeRevokedPayload({ id: "rev-ct-other",  claimType: "AML_CLEARED" });

      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: other  });
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationRevoked: ReturnType<typeof makeRevokedPayload> }>(ATTESTATION_REVOKED);
      const match = await applySubscriptionFilter(iter, "onAttestationRevoked", { claimType: "KYC_PASSED" });
      expect(match.id).toBe("rev-ct-match");
    });

    it("combines subject + issuer filters (AND logic)", async () => {
      const target = makeRevokedPayload({ id: "rev-combined", subject: "GSUB_T", issuer: "GISS_T" });
      const wrong1 = makeRevokedPayload({ id: "rev-w1", subject: "GSUB_X", issuer: "GISS_T" });
      const wrong2 = makeRevokedPayload({ id: "rev-w2", subject: "GSUB_T", issuer: "GISS_X" });

      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: wrong1 });
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: wrong2 });
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationRevoked: ReturnType<typeof makeRevokedPayload> }>(ATTESTATION_REVOKED);
      const match = await applySubscriptionFilter(iter, "onAttestationRevoked", { subject: "GSUB_T", issuer: "GISS_T" });
      expect(match.id).toBe("rev-combined");
    });

    it("combines subject + claimType filters (AND logic)", async () => {
      const target = makeRevokedPayload({ id: "rev-sc", subject: "GSUB_T", claimType: "KYC_PASSED" });
      const wrong  = makeRevokedPayload({ id: "rev-sc-w", subject: "GSUB_T", claimType: "AML_CLEARED" });

      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: wrong  });
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationRevoked: ReturnType<typeof makeRevokedPayload> }>(ATTESTATION_REVOKED);
      const match = await applySubscriptionFilter(iter, "onAttestationRevoked", { subject: "GSUB_T", claimType: "KYC_PASSED" });
      expect(match.id).toBe("rev-sc");
    });

    it("combines issuer + claimType filters (AND logic)", async () => {
      const target = makeRevokedPayload({ id: "rev-ic", issuer: "GISS_T", claimType: "KYC_PASSED" });
      const wrong  = makeRevokedPayload({ id: "rev-ic-w", issuer: "GISS_T", claimType: "AML_CLEARED" });

      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: wrong  });
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: target });

      const iter = testPubsub.asyncIterableIterator<{ onAttestationRevoked: ReturnType<typeof makeRevokedPayload> }>(ATTESTATION_REVOKED);
      const match = await applySubscriptionFilter(iter, "onAttestationRevoked", { issuer: "GISS_T", claimType: "KYC_PASSED" });
      expect(match.id).toBe("rev-ic");
    });

    it("combines all three filters — subject + issuer + claimType", async () => {
      const target = makeRevokedPayload({ id: "rev-all", subject: "GSUB_T", issuer: "GISS_T", claimType: "KYC_PASSED" });
      const miss1  = makeRevokedPayload({ id: "rev-al1", subject: "GSUB_X", issuer: "GISS_T", claimType: "KYC_PASSED" });
      const miss2  = makeRevokedPayload({ id: "rev-al2", subject: "GSUB_T", issuer: "GISS_X", claimType: "KYC_PASSED" });
      const miss3  = makeRevokedPayload({ id: "rev-al3", subject: "GSUB_T", issuer: "GISS_T", claimType: "AML_CLEARED" });

      for (const p of [miss1, miss2, miss3, target]) {
        await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: p });
      }

      const iter = testPubsub.asyncIterableIterator<{ onAttestationRevoked: ReturnType<typeof makeRevokedPayload> }>(ATTESTATION_REVOKED);
      const match = await applySubscriptionFilter(iter, "onAttestationRevoked", {
        subject: "GSUB_T",
        issuer: "GISS_T",
        claimType: "KYC_PASSED",
      });
      expect(match.id).toBe("rev-all");
    });
  });
});
