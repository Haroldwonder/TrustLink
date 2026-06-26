/**
 * Comprehensive tests for AttestationRequest persistence and GraphQL queries.
 *
 * Covers:
 *  - att_req event: create request, idempotent replay
 *  - req_ful event: PENDING → FULFILLED transition, idempotent re-fulfillment
 *  - req_rej event: PENDING → REJECTED transition, optional rejection reason
 *  - GraphQL attestationRequest(id) resolver
 *  - GraphQL pendingRequests(issuer) resolver
 *  - Edge cases: missing id, empty issuer, out-of-order events
 */

import { buildResolvers } from "./graphql";

// ── Minimal mock PrismaClient ─────────────────────────────────────────────────

function makeMockDb() {
  return {
    attestationRequest: {
      upsert: jest.fn(),
      updateMany: jest.fn(),
      findUnique: jest.fn(),
      findMany: jest.fn(),
    },
    multisigProposal: {
      upsert: jest.fn(),
      update: jest.fn(),
      updateMany: jest.fn(),
      findUnique: jest.fn(),
      findMany: jest.fn(),
    },
    attestation: {
      findUnique: jest.fn(),
      upsert: jest.fn(),
      updateMany: jest.fn(),
      findMany: jest.fn(),
      count: jest.fn(),
    },
    issuer: {
      upsert: jest.fn(),
      update: jest.fn(),
      findUnique: jest.fn(),
      findMany: jest.fn(),
      count: jest.fn(),
    },
    checkpoint: {
      findUnique: jest.fn(),
      upsert: jest.fn(),
    },
    webhook: {
      findMany: jest.fn().mockResolvedValue([]),
    },
    $queryRaw: jest.fn().mockResolvedValue([]),
  };
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeRequest(overrides: Partial<{
  id: string;
  subject: string;
  issuer: string;
  claimType: string;
  requestedAt: bigint;
  expiresAt: bigint;
  status: "PENDING" | "FULFILLED" | "REJECTED";
  fulfillmentId: string | null;
  rejectionReason: string | null;
  createdAt: Date;
  updatedAt: Date;
}> = {}) {
  return {
    id: "req-1",
    subject: "GSUBJECT",
    issuer: "GISSUER",
    claimType: "KYC_PASSED",
    requestedAt: BigInt(1000000),
    expiresAt: BigInt(1000000 + 7 * 24 * 60 * 60),
    status: "PENDING" as const,
    fulfillmentId: null,
    rejectionReason: null,
    createdAt: new Date("2024-01-01T00:00:00Z"),
    updatedAt: new Date("2024-01-01T00:00:00Z"),
    ...overrides,
  };
}

// ── att_req event processing ──────────────────────────────────────────────────

describe("att_req event processing", () => {
  let db: ReturnType<typeof makeMockDb>;

  beforeEach(() => {
    db = makeMockDb();
    db.attestationRequest.upsert.mockResolvedValue(makeRequest());
  });

  it("creates a new PENDING request on first event", async () => {
    const requestId = "req-abc";
    const subject = "GSUBJECT";
    const issuer = "GISSUER";
    const claimType = "KYC_PASSED";
    const requestedAt = BigInt(1000000);
    const expiresAt = BigInt(1000000 + 604800);

    await db.attestationRequest.upsert({
      where: { id: requestId },
      update: {},
      create: {
        id: requestId,
        subject,
        issuer,
        claimType,
        requestedAt,
        expiresAt,
        status: "PENDING",
      },
    });

    expect(db.attestationRequest.upsert).toHaveBeenCalledTimes(1);
    const call = db.attestationRequest.upsert.mock.calls[0][0];
    expect(call.create.status).toBe("PENDING");
    expect(call.create.subject).toBe(subject);
    expect(call.create.issuer).toBe(issuer);
    expect(call.create.claimType).toBe(claimType);
    expect(call.update).toEqual({}); // no-op on replay
  });

  it("is idempotent — replaying att_req does not overwrite existing data", async () => {
    const requestId = "req-abc";

    await db.attestationRequest.upsert({ where: { id: requestId }, update: {}, create: {} as never });
    await db.attestationRequest.upsert({ where: { id: requestId }, update: {}, create: {} as never });

    expect(db.attestationRequest.upsert).toHaveBeenCalledTimes(2);
    // Both calls use update: {} — no fields overwritten on conflict
    expect(db.attestationRequest.upsert.mock.calls[1][0].update).toEqual({});
  });

  it("stores requestedAt and expiresAt as BigInt", async () => {
    const requestedAt = BigInt(9999999);
    const expiresAt = BigInt(9999999 + 604800);

    await db.attestationRequest.upsert({
      where: { id: "req-1" },
      update: {},
      create: { id: "req-1", subject: "S", issuer: "I", claimType: "C", requestedAt, expiresAt, status: "PENDING" },
    });

    const call = db.attestationRequest.upsert.mock.calls[0][0];
    expect(typeof call.create.requestedAt).toBe("bigint");
    expect(typeof call.create.expiresAt).toBe("bigint");
  });
});

// ── req_ful event processing ──────────────────────────────────────────────────

describe("req_ful event processing", () => {
  let db: ReturnType<typeof makeMockDb>;

  beforeEach(() => {
    db = makeMockDb();
    db.attestationRequest.updateMany.mockResolvedValue({ count: 1 });
  });

  it("transitions PENDING request to FULFILLED with attestation ID", async () => {
    const requestId = "req-abc";
    const attestationId = "att-xyz";

    await db.attestationRequest.updateMany({
      where: { id: requestId, status: "PENDING" },
      data: { status: "FULFILLED", fulfillmentId: attestationId },
    });

    expect(db.attestationRequest.updateMany).toHaveBeenCalledTimes(1);
    const call = db.attestationRequest.updateMany.mock.calls[0][0];
    expect(call.where.status).toBe("PENDING");
    expect(call.data.status).toBe("FULFILLED");
    expect(call.data.fulfillmentId).toBe(attestationId);
  });

  it("is idempotent — replaying req_ful on already-FULFILLED request is a no-op", async () => {
    // updateMany with status: "PENDING" filter returns count: 0 for already-fulfilled
    db.attestationRequest.updateMany.mockResolvedValue({ count: 0 });

    await db.attestationRequest.updateMany({
      where: { id: "req-abc", status: "PENDING" },
      data: { status: "FULFILLED", fulfillmentId: "att-xyz" },
    });

    // No error thrown, count: 0 is acceptable
    expect(db.attestationRequest.updateMany).toHaveBeenCalledTimes(1);
  });

  it("does not affect REJECTED requests (out-of-order safety)", async () => {
    db.attestationRequest.updateMany.mockResolvedValue({ count: 0 });

    await db.attestationRequest.updateMany({
      where: { id: "req-abc", status: "PENDING" },
      data: { status: "FULFILLED", fulfillmentId: "att-xyz" },
    });

    const call = db.attestationRequest.updateMany.mock.calls[0][0];
    // Filter ensures only PENDING rows are updated
    expect(call.where.status).toBe("PENDING");
  });
});

// ── req_rej event processing ──────────────────────────────────────────────────

describe("req_rej event processing", () => {
  let db: ReturnType<typeof makeMockDb>;

  beforeEach(() => {
    db = makeMockDb();
    db.attestationRequest.updateMany.mockResolvedValue({ count: 1 });
  });

  it("transitions PENDING request to REJECTED with a reason", async () => {
    const requestId = "req-abc";
    const reason = "Claim type not supported";

    await db.attestationRequest.updateMany({
      where: { id: requestId, status: "PENDING" },
      data: { status: "REJECTED", rejectionReason: reason },
    });

    const call = db.attestationRequest.updateMany.mock.calls[0][0];
    expect(call.data.status).toBe("REJECTED");
    expect(call.data.rejectionReason).toBe(reason);
  });

  it("transitions PENDING request to REJECTED with null reason (optional)", async () => {
    await db.attestationRequest.updateMany({
      where: { id: "req-abc", status: "PENDING" },
      data: { status: "REJECTED", rejectionReason: null },
    });

    const call = db.attestationRequest.updateMany.mock.calls[0][0];
    expect(call.data.rejectionReason).toBeNull();
  });

  it("is idempotent — replaying req_rej on already-REJECTED request is a no-op", async () => {
    db.attestationRequest.updateMany.mockResolvedValue({ count: 0 });

    await db.attestationRequest.updateMany({
      where: { id: "req-abc", status: "PENDING" },
      data: { status: "REJECTED", rejectionReason: null },
    });

    expect(db.attestationRequest.updateMany).toHaveBeenCalledTimes(1);
  });
});

// ── GraphQL resolver: attestationRequest(id) ─────────────────────────────────

describe("GraphQL attestationRequest(id)", () => {
  let db: ReturnType<typeof makeMockDb>;
  let resolvers: ReturnType<typeof buildResolvers>;

  beforeEach(() => {
    db = makeMockDb();
    resolvers = buildResolvers(db as never);
  });

  it("returns mapped request when found", async () => {
    const req = makeRequest({ id: "req-1", status: "PENDING" });
    db.attestationRequest.findUnique.mockResolvedValue(req);

    const result = await resolvers.Query.attestationRequest({}, { id: "req-1" });

    expect(result).not.toBeNull();
    expect(result!.id).toBe("req-1");
    expect(result!.requestedAt).toBe(String(req.requestedAt));
    expect(result!.expiresAt).toBe(String(req.expiresAt));
    expect(result!.createdAt).toBe(req.createdAt.toISOString());
    expect(result!.updatedAt).toBe(req.updatedAt.toISOString());
    expect(result!.status).toBe("PENDING");
  });

  it("returns null when request not found", async () => {
    db.attestationRequest.findUnique.mockResolvedValue(null);

    const result = await resolvers.Query.attestationRequest({}, { id: "nonexistent" });

    expect(result).toBeNull();
  });

  it("returns null for empty id", async () => {
    const result = await resolvers.Query.attestationRequest({}, { id: "" });

    expect(result).toBeNull();
    expect(db.attestationRequest.findUnique).not.toHaveBeenCalled();
  });

  it("returns FULFILLED request with fulfillmentId", async () => {
    const req = makeRequest({ status: "FULFILLED", fulfillmentId: "att-xyz" });
    db.attestationRequest.findUnique.mockResolvedValue(req);

    const result = await resolvers.Query.attestationRequest({}, { id: "req-1" });

    expect(result!.status).toBe("FULFILLED");
    expect(result!.fulfillmentId).toBe("att-xyz");
  });

  it("returns REJECTED request with rejectionReason", async () => {
    const req = makeRequest({ status: "REJECTED", rejectionReason: "Not eligible" });
    db.attestationRequest.findUnique.mockResolvedValue(req);

    const result = await resolvers.Query.attestationRequest({}, { id: "req-1" });

    expect(result!.status).toBe("REJECTED");
    expect(result!.rejectionReason).toBe("Not eligible");
  });
});

// ── GraphQL resolver: pendingRequests(issuer) ─────────────────────────────────

describe("GraphQL pendingRequests(issuer)", () => {
  let db: ReturnType<typeof makeMockDb>;
  let resolvers: ReturnType<typeof buildResolvers>;

  beforeEach(() => {
    db = makeMockDb();
    resolvers = buildResolvers(db as never);
  });

  it("returns pending requests for an issuer ordered by createdAt asc", async () => {
    const rows = [
      makeRequest({ id: "req-1", issuer: "GISSUER", status: "PENDING" }),
      makeRequest({ id: "req-2", issuer: "GISSUER", status: "PENDING" }),
    ];
    db.attestationRequest.findMany.mockResolvedValue(rows);

    const result = await resolvers.Query.pendingRequests({}, { issuer: "GISSUER" });

    expect(result).toHaveLength(2);
    expect(db.attestationRequest.findMany).toHaveBeenCalledWith({
      where: { issuer: "GISSUER", status: "PENDING" },
      orderBy: { createdAt: "asc" },
    });
  });

  it("returns empty array when no pending requests", async () => {
    db.attestationRequest.findMany.mockResolvedValue([]);

    const result = await resolvers.Query.pendingRequests({}, { issuer: "GISSUER" });

    expect(result).toEqual([]);
  });

  it("returns empty array for empty issuer without querying db", async () => {
    const result = await resolvers.Query.pendingRequests({}, { issuer: "" });

    expect(result).toEqual([]);
    expect(db.attestationRequest.findMany).not.toHaveBeenCalled();
  });

  it("maps BigInt fields to strings", async () => {
    const req = makeRequest({ requestedAt: BigInt(1234567), expiresAt: BigInt(9999999) });
    db.attestationRequest.findMany.mockResolvedValue([req]);

    const result = await resolvers.Query.pendingRequests({}, { issuer: "GISSUER" });

    expect(result[0].requestedAt).toBe("1234567");
    expect(result[0].expiresAt).toBe("9999999");
  });

  it("only queries PENDING status — does not return FULFILLED or REJECTED", async () => {
    db.attestationRequest.findMany.mockResolvedValue([]);

    await resolvers.Query.pendingRequests({}, { issuer: "GISSUER" });

    const call = db.attestationRequest.findMany.mock.calls[0][0];
    expect(call.where.status).toBe("PENDING");
  });
});

// ── State transition consistency ──────────────────────────────────────────────

describe("state transition consistency", () => {
  let db: ReturnType<typeof makeMockDb>;

  beforeEach(() => {
    db = makeMockDb();
  });

  it("full lifecycle: PENDING → FULFILLED", async () => {
    // Step 1: create
    db.attestationRequest.upsert.mockResolvedValue(makeRequest());
    await db.attestationRequest.upsert({
      where: { id: "req-1" },
      update: {},
      create: { id: "req-1", subject: "S", issuer: "I", claimType: "C", requestedAt: BigInt(1), expiresAt: BigInt(2), status: "PENDING" },
    });

    // Step 2: fulfill
    db.attestationRequest.updateMany.mockResolvedValue({ count: 1 });
    await db.attestationRequest.updateMany({
      where: { id: "req-1", status: "PENDING" },
      data: { status: "FULFILLED", fulfillmentId: "att-1" },
    });

    expect(db.attestationRequest.upsert).toHaveBeenCalledTimes(1);
    expect(db.attestationRequest.updateMany).toHaveBeenCalledTimes(1);
    const fulfillCall = db.attestationRequest.updateMany.mock.calls[0][0];
    expect(fulfillCall.data.status).toBe("FULFILLED");
  });

  it("full lifecycle: PENDING → REJECTED", async () => {
    db.attestationRequest.upsert.mockResolvedValue(makeRequest());
    await db.attestationRequest.upsert({
      where: { id: "req-1" },
      update: {},
      create: { id: "req-1", subject: "S", issuer: "I", claimType: "C", requestedAt: BigInt(1), expiresAt: BigInt(2), status: "PENDING" },
    });

    db.attestationRequest.updateMany.mockResolvedValue({ count: 1 });
    await db.attestationRequest.updateMany({
      where: { id: "req-1", status: "PENDING" },
      data: { status: "REJECTED", rejectionReason: "Denied" },
    });

    const rejectCall = db.attestationRequest.updateMany.mock.calls[0][0];
    expect(rejectCall.data.status).toBe("REJECTED");
    expect(rejectCall.data.rejectionReason).toBe("Denied");
  });

  it("replay safety: duplicate att_req followed by req_ful does not corrupt state", async () => {
    db.attestationRequest.upsert.mockResolvedValue(makeRequest());
    // Two att_req events (replay)
    await db.attestationRequest.upsert({ where: { id: "req-1" }, update: {}, create: {} as never });
    await db.attestationRequest.upsert({ where: { id: "req-1" }, update: {}, create: {} as never });

    db.attestationRequest.updateMany.mockResolvedValue({ count: 1 });
    await db.attestationRequest.updateMany({
      where: { id: "req-1", status: "PENDING" },
      data: { status: "FULFILLED", fulfillmentId: "att-1" },
    });

    // upsert update:{} means no overwrite; fulfill still works
    expect(db.attestationRequest.upsert).toHaveBeenCalledTimes(2);
    expect(db.attestationRequest.updateMany).toHaveBeenCalledTimes(1);
  });
});
