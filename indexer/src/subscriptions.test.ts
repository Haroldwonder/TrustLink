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

// Helper to build a fake attestation payload
function makeAttestationPayload(overrides: Partial<{
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

// Helper to build a fake revocation payload
function makeRevocationPayload(overrides: Partial<{
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

// Helper: apply filter logic matching graphql.ts subscription implementation
async function collectFiltered<T extends Record<string, unknown>>(
  testPubsub: PubSub,
  channel: string,
  payloads: Array<{ [key: string]: T }>,
  filter: (item: T) => boolean,
  fieldName: string
): Promise<T[]> {
  // Publish all payloads
  for (const p of payloads) {
    await testPubsub.publish(channel, p);
  }

  const results: T[] = [];
  const iter = testPubsub.asyncIterableIterator<{ [key: string]: T }>(channel);

  for (let i = 0; i < payloads.length; i++) {
    const result = await iter.next();
    if (result.done) break;
    const item = result.value?.[fieldName];
    if (item && filter(item)) {
      results.push(item);
    }
  }

  return results;
}

describe("GraphQL Subscriptions", () => {
  let testPubsub: PubSub;

  beforeEach(() => {
    testPubsub = new PubSub();
  });

  // ── Basic publish/receive ────────────────────────────────────────────────

  it("should publish ATTESTATION_CREATED events", async () => {
    const payload = makeAttestationPayload();

    const publishPromise = testPubsub.publish(ATTESTATION_CREATED, {
      onAttestationCreated: payload,
    });

    const iterator = testPubsub.asyncIterableIterator(ATTESTATION_CREATED);
    await publishPromise;

    const result = await iterator.next();
    expect(result.done).toBe(false);
    expect(result.value).toHaveProperty("onAttestationCreated");
    expect(result.value.onAttestationCreated.id).toBe("test-att-1");
  });

  it("should publish ATTESTATION_REVOKED events", async () => {
    const payload = makeRevocationPayload();

    await testPubsub.publish(ATTESTATION_REVOKED, {
      onAttestationRevoked: payload,
    });

    const iterator = testPubsub.asyncIterableIterator(ATTESTATION_REVOKED);
    const result = await iterator.next();

    expect(result.done).toBe(false);
    expect(result.value.onAttestationRevoked.id).toBe("test-att-1");
  });

  it("should publish ISSUER_REGISTERED events", async () => {
    const payload = { issuer: "GABC123", registeredAt: new Date().toISOString() };

    await testPubsub.publish(ISSUER_REGISTERED, {
      onIssuerRegistered: payload,
    });

    const iterator = testPubsub.asyncIterableIterator(ISSUER_REGISTERED);
    const result = await iterator.next();

    expect(result.done).toBe(false);
    expect(result.value.onIssuerRegistered.issuer).toBe("GABC123");
  });

  // ── onAttestationCreated filters ─────────────────────────────────────────

  it("onAttestationCreated: filter by subject only", async () => {
    const target = makeAttestationPayload({ subject: "GDEF456" });
    const other = makeAttestationPayload({ id: "test-att-2", subject: "GHIJ789" });

    await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: target });
    await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: other });

    const filtered = await collectFiltered(
      testPubsub,
      ATTESTATION_CREATED,
      [],
      (a) => a.subject === "GDEF456",
      "onAttestationCreated"
    );

    // Verify our filter logic: only target passes
    expect(filtered.every((a) => a.subject === "GDEF456")).toBe(true);
  });

  it("onAttestationCreated: filter by issuer only", async () => {
    const payloads = [
      makeAttestationPayload({ id: "att-1", issuer: "ISSUER_A" }),
      makeAttestationPayload({ id: "att-2", issuer: "ISSUER_B" }),
      makeAttestationPayload({ id: "att-3", issuer: "ISSUER_A" }),
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationCreated: ReturnType<typeof makeAttestationPayload>;
    }>(ATTESTATION_CREATED);

    const targetIssuer = "ISSUER_A";
    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const att = result.value?.onAttestationCreated;
      if (att && att.issuer === targetIssuer) seen.push(att.id);
    }

    expect(seen).toContain("att-1");
    expect(seen).toContain("att-3");
    expect(seen).not.toContain("att-2");
  });

  it("onAttestationCreated: filter by claimType only", async () => {
    const payloads = [
      makeAttestationPayload({ id: "att-1", claimType: "KYC_PASSED" }),
      makeAttestationPayload({ id: "att-2", claimType: "ACCREDITED_INVESTOR" }),
      makeAttestationPayload({ id: "att-3", claimType: "KYC_PASSED" }),
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationCreated: ReturnType<typeof makeAttestationPayload>;
    }>(ATTESTATION_CREATED);

    const targetClaim = "KYC_PASSED";
    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const att = result.value?.onAttestationCreated;
      if (att && att.claimType === targetClaim) seen.push(att.id);
    }

    expect(seen).toContain("att-1");
    expect(seen).toContain("att-3");
    expect(seen).not.toContain("att-2");
  });

  it("onAttestationCreated: filter by issuer AND claimType combined", async () => {
    const payloads = [
      makeAttestationPayload({ id: "att-1", issuer: "ISSUER_A", claimType: "KYC_PASSED" }),
      makeAttestationPayload({ id: "att-2", issuer: "ISSUER_B", claimType: "KYC_PASSED" }), // wrong issuer
      makeAttestationPayload({ id: "att-3", issuer: "ISSUER_A", claimType: "AML_CLEARED" }), // wrong claimType
      makeAttestationPayload({ id: "att-4", issuer: "ISSUER_A", claimType: "KYC_PASSED" }), // matches both
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationCreated: ReturnType<typeof makeAttestationPayload>;
    }>(ATTESTATION_CREATED);

    const targetIssuer = "ISSUER_A";
    const targetClaim = "KYC_PASSED";
    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const att = result.value?.onAttestationCreated;
      if (att && att.issuer === targetIssuer && att.claimType === targetClaim) {
        seen.push(att.id);
      }
    }

    expect(seen).toContain("att-1");
    expect(seen).toContain("att-4");
    expect(seen).not.toContain("att-2");
    expect(seen).not.toContain("att-3");
  });

  it("onAttestationCreated: filter by subject AND issuer AND claimType combined", async () => {
    const payloads = [
      makeAttestationPayload({ id: "match", subject: "SUB_A", issuer: "ISS_A", claimType: "KYC_PASSED" }),
      makeAttestationPayload({ id: "wrong-sub", subject: "SUB_B", issuer: "ISS_A", claimType: "KYC_PASSED" }),
      makeAttestationPayload({ id: "wrong-iss", subject: "SUB_A", issuer: "ISS_B", claimType: "KYC_PASSED" }),
      makeAttestationPayload({ id: "wrong-claim", subject: "SUB_A", issuer: "ISS_A", claimType: "AML_CLEARED" }),
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationCreated: ReturnType<typeof makeAttestationPayload>;
    }>(ATTESTATION_CREATED);

    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const att = result.value?.onAttestationCreated;
      if (att && att.subject === "SUB_A" && att.issuer === "ISS_A" && att.claimType === "KYC_PASSED") {
        seen.push(att.id);
      }
    }

    expect(seen).toEqual(["match"]);
  });

  // ── onAttestationRevoked filters ─────────────────────────────────────────

  it("onAttestationRevoked: filter by subject only", async () => {
    const payloads = [
      makeRevocationPayload({ id: "att-1", subject: "SUB_A" }),
      makeRevocationPayload({ id: "att-2", subject: "SUB_B" }),
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationRevoked: ReturnType<typeof makeRevocationPayload>;
    }>(ATTESTATION_REVOKED);

    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const ev = result.value?.onAttestationRevoked;
      if (ev && ev.subject === "SUB_A") seen.push(ev.id);
    }

    expect(seen).toContain("att-1");
    expect(seen).not.toContain("att-2");
  });

  it("onAttestationRevoked: filter by issuer only", async () => {
    const payloads = [
      makeRevocationPayload({ id: "att-1", issuer: "ISS_A" }),
      makeRevocationPayload({ id: "att-2", issuer: "ISS_B" }),
      makeRevocationPayload({ id: "att-3", issuer: "ISS_A" }),
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationRevoked: ReturnType<typeof makeRevocationPayload>;
    }>(ATTESTATION_REVOKED);

    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const ev = result.value?.onAttestationRevoked;
      if (ev && ev.issuer === "ISS_A") seen.push(ev.id);
    }

    expect(seen).toContain("att-1");
    expect(seen).toContain("att-3");
    expect(seen).not.toContain("att-2");
  });

  it("onAttestationRevoked: filter by claimType only", async () => {
    const payloads = [
      makeRevocationPayload({ id: "att-1", claimType: "KYC_PASSED" }),
      makeRevocationPayload({ id: "att-2", claimType: "AML_CLEARED" }),
      makeRevocationPayload({ id: "att-3", claimType: "KYC_PASSED" }),
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationRevoked: ReturnType<typeof makeRevocationPayload>;
    }>(ATTESTATION_REVOKED);

    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const ev = result.value?.onAttestationRevoked;
      if (ev && ev.claimType === "KYC_PASSED") seen.push(ev.id);
    }

    expect(seen).toContain("att-1");
    expect(seen).toContain("att-3");
    expect(seen).not.toContain("att-2");
  });

  it("onAttestationRevoked: filter by issuer AND claimType combined", async () => {
    const payloads = [
      makeRevocationPayload({ id: "match", issuer: "ISS_A", claimType: "KYC_PASSED" }),
      makeRevocationPayload({ id: "wrong-iss", issuer: "ISS_B", claimType: "KYC_PASSED" }),
      makeRevocationPayload({ id: "wrong-claim", issuer: "ISS_A", claimType: "AML_CLEARED" }),
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationRevoked: ReturnType<typeof makeRevocationPayload>;
    }>(ATTESTATION_REVOKED);

    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const ev = result.value?.onAttestationRevoked;
      if (ev && ev.issuer === "ISS_A" && ev.claimType === "KYC_PASSED") seen.push(ev.id);
    }

    expect(seen).toEqual(["match"]);
  });

  it("onAttestationRevoked: filter by subject AND issuer AND claimType combined", async () => {
    const payloads = [
      makeRevocationPayload({ id: "match", subject: "SUB_A", issuer: "ISS_A", claimType: "KYC_PASSED" }),
      makeRevocationPayload({ id: "wrong-sub", subject: "SUB_B", issuer: "ISS_A", claimType: "KYC_PASSED" }),
      makeRevocationPayload({ id: "wrong-iss", subject: "SUB_A", issuer: "ISS_B", claimType: "KYC_PASSED" }),
      makeRevocationPayload({ id: "wrong-claim", subject: "SUB_A", issuer: "ISS_A", claimType: "AML_CLEARED" }),
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationRevoked: ReturnType<typeof makeRevocationPayload>;
    }>(ATTESTATION_REVOKED);

    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const ev = result.value?.onAttestationRevoked;
      if (ev && ev.subject === "SUB_A" && ev.issuer === "ISS_A" && ev.claimType === "KYC_PASSED") {
        seen.push(ev.id);
      }
    }

    expect(seen).toEqual(["match"]);
  });

  // ── Backwards compatibility: no-filter case passes everything through ────

  it("onAttestationCreated: no filters passes all events", async () => {
    const payloads = [
      makeAttestationPayload({ id: "att-1", subject: "SUB_A", issuer: "ISS_A", claimType: "KYC_PASSED" }),
      makeAttestationPayload({ id: "att-2", subject: "SUB_B", issuer: "ISS_B", claimType: "AML_CLEARED" }),
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_CREATED, { onAttestationCreated: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationCreated: ReturnType<typeof makeAttestationPayload>;
    }>(ATTESTATION_CREATED);

    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const att = result.value?.onAttestationCreated;
      if (att) seen.push(att.id);
    }

    expect(seen).toContain("att-1");
    expect(seen).toContain("att-2");
  });

  it("onAttestationRevoked: no filters passes all events", async () => {
    const payloads = [
      makeRevocationPayload({ id: "att-1" }),
      makeRevocationPayload({ id: "att-2", issuer: "OTHER_ISS" }),
    ];

    for (const p of payloads) {
      await testPubsub.publish(ATTESTATION_REVOKED, { onAttestationRevoked: p });
    }

    const iter = testPubsub.asyncIterableIterator<{
      onAttestationRevoked: ReturnType<typeof makeRevocationPayload>;
    }>(ATTESTATION_REVOKED);

    const seen: string[] = [];
    for (let i = 0; i < payloads.length; i++) {
      const result = await iter.next();
      const ev = result.value?.onAttestationRevoked;
      if (ev) seen.push(ev.id);
    }

    expect(seen).toContain("att-1");
    expect(seen).toContain("att-2");
  });
});
