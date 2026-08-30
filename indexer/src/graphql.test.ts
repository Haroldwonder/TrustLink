/**
 * #1137 – Resolver-level tests for indexer/src/graphql.ts
 *
 * The GraphQL surface is actively evolving (ADR-011 federation / schema
 * versioning), but the resolver implementations built by `buildResolvers`
 * were only covered indirectly via schema-validation tests. These tests
 * drive the resolvers directly against a mocked Prisma client and a mocked
 * Redis cache.
 *
 * ioredis is mocked so importing the module never opens a real connection.
 */

jest.mock("ioredis", () => {
  return jest.fn().mockImplementation(() => ({
    on: jest.fn(),
    get: jest.fn(),
    set: jest.fn(),
    del: jest.fn(),
    keys: jest.fn(),
    quit: jest.fn().mockResolvedValue("OK"),
    disconnect: jest.fn(),
  }));
});

import {
  buildResolvers,
  cacheInvalidate,
  SCHEMA_VERSION,
  ATTESTATION_CREATED,
  ATTESTATION_REVOKED,
  ISSUER_REGISTERED,
} from "./graphql";

type AnyResolvers = ReturnType<typeof buildResolvers>;

function makeDb() {
  return {
    issuer: {
      findUnique: jest.fn(),
      findMany: jest.fn(),
      count: jest.fn(),
    },
    attestation: {
      findMany: jest.fn(),
    },
    multisigProposal: {
      findUnique: jest.fn(),
      findMany: jest.fn(),
    },
    auditEntry: {
      findMany: jest.fn(),
    },
    $queryRaw: jest.fn(),
  };
}

const D = new Date("2026-01-02T03:04:05.000Z");
const ISO = D.toISOString();

describe("module constants", () => {
  it("exports a semver-shaped SCHEMA_VERSION", () => {
    expect(SCHEMA_VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it("exports the pubsub channel names", () => {
    expect(ATTESTATION_CREATED).toBe("ATTESTATION_CREATED");
    expect(ATTESTATION_REVOKED).toBe("ATTESTATION_REVOKED");
    expect(ISSUER_REGISTERED).toBe("ISSUER_REGISTERED");
  });
});

describe("buildResolvers shape", () => {
  it("returns Query and Subscription resolver maps", () => {
    const r = buildResolvers(makeDb() as never);
    expect(typeof r.Query.issuer).toBe("function");
    expect(typeof r.Query.issuers).toBe("function");
    expect(typeof r.Query.issuerStats).toBe("function");
    expect(typeof r.Query.proposal).toBe("function");
    expect(typeof r.Query.proposals).toBe("function");
    expect(typeof r.Query.auditLog).toBe("function");
    expect(typeof r.Subscription.onAttestationCreated.subscribe).toBe("function");
    expect(typeof r.Subscription.onAttestationRevoked.subscribe).toBe("function");
    expect(typeof r.Subscription.onIssuerRegistered.subscribe).toBe("function");
  });
});

describe("Query.issuer", () => {
  it("returns the issuer with ISO-serialised timestamps", async () => {
    const db = makeDb();
    db.issuer.findUnique.mockResolvedValue({
      address: "GISSUER",
      rateLimit: 10,
      registeredAt: D,
      updatedAt: D,
    });
    const r = buildResolvers(db as never);
    const out = await r.Query.issuer({}, { address: "GISSUER" });
    expect(db.issuer.findUnique).toHaveBeenCalledWith({ where: { address: "GISSUER" } });
    expect(out).toMatchObject({ address: "GISSUER", registeredAt: ISO, updatedAt: ISO });
  });

  it("returns null when the issuer does not exist", async () => {
    const db = makeDb();
    db.issuer.findUnique.mockResolvedValue(null);
    const r = buildResolvers(db as never);
    expect(await r.Query.issuer({}, { address: "nope" })).toBeNull();
  });
});

describe("Query.issuers", () => {
  it("applies default pagination and serialises rows", async () => {
    const db = makeDb();
    db.issuer.findMany.mockResolvedValue([
      { address: "G1", registeredAt: D, updatedAt: D },
    ]);
    db.issuer.count.mockResolvedValue(1);
    const r = buildResolvers(db as never);
    const out = await r.Query.issuers({}, {});
    expect(db.issuer.findMany).toHaveBeenCalledWith({
      skip: 0,
      take: 50,
      orderBy: { registeredAt: "desc" },
    });
    expect(out).toEqual({
      items: [{ address: "G1", registeredAt: ISO, updatedAt: ISO }],
      total: 1,
    });
  });

  it("honours explicit start/limit", async () => {
    const db = makeDb();
    db.issuer.findMany.mockResolvedValue([]);
    db.issuer.count.mockResolvedValue(0);
    const r = buildResolvers(db as never);
    await r.Query.issuers({}, { start: 20, limit: 5 });
    expect(db.issuer.findMany).toHaveBeenCalledWith(
      expect.objectContaining({ skip: 20, take: 5 }),
    );
  });
});

describe("Query.issuerStats", () => {
  it("aggregates attestation rows when there is no cache", async () => {
    const db = makeDb();
    db.attestation.findMany.mockResolvedValue([
      { isRevoked: false, claimType: "KYC" },
      { isRevoked: true, claimType: "KYC" },
      { isRevoked: false, claimType: "AML" },
    ]);
    db.issuer.findUnique.mockResolvedValue({ address: "GX", rateLimit: 100 });
    const r = buildResolvers(db as never, null);
    const out = await r.Query.issuerStats({}, { issuer: "GX" });
    expect(out).toEqual({
      issuer: "GX",
      total: 3,
      active: 2,
      revoked: 1,
      claimTypes: ["KYC", "AML"],
      rateLimit: 100,
    });
  });

  it("returns null rateLimit when the issuer is unknown", async () => {
    const db = makeDb();
    db.attestation.findMany.mockResolvedValue([]);
    db.issuer.findUnique.mockResolvedValue(null);
    const r = buildResolvers(db as never, null);
    const out = await r.Query.issuerStats({}, { issuer: "GX" });
    expect(out).toMatchObject({ total: 0, active: 0, revoked: 0, claimTypes: [], rateLimit: null });
  });

  it("serves a cached value without hitting the database", async () => {
    const db = makeDb();
    const cached = { issuer: "GX", total: 7, active: 7, revoked: 0, claimTypes: [], rateLimit: null };
    const redis = {
      get: jest.fn().mockResolvedValue(JSON.stringify(cached)),
      set: jest.fn(),
    };
    const r = buildResolvers(db as never, redis as never);
    const out = await r.Query.issuerStats({}, { issuer: "GX" });
    expect(out).toEqual(cached);
    expect(redis.get).toHaveBeenCalledWith("issuerStats:GX");
    expect(db.attestation.findMany).not.toHaveBeenCalled();
  });

  it("recomputes and writes to cache on a miss", async () => {
    const db = makeDb();
    db.attestation.findMany.mockResolvedValue([{ isRevoked: false, claimType: "KYC" }]);
    db.issuer.findUnique.mockResolvedValue({ rateLimit: 5 });
    const redis = {
      get: jest.fn().mockResolvedValue(null),
      set: jest.fn().mockResolvedValue("OK"),
    };
    const r = buildResolvers(db as never, redis as never);
    await r.Query.issuerStats({}, { issuer: "GX" });
    expect(redis.set).toHaveBeenCalledWith(
      "issuerStats:GX",
      expect.any(String),
      "EX",
      expect.any(Number),
    );
  });
});

describe("Query.proposal / proposals", () => {
  const proposalRow = {
    id: "p1",
    subject: "S",
    expiresAt: 1893456000n,
    createdAt: D,
    updatedAt: D,
  };

  it("maps a single proposal", async () => {
    const db = makeDb();
    db.multisigProposal.findUnique.mockResolvedValue(proposalRow);
    const r = buildResolvers(db as never);
    const out = await r.Query.proposal({}, { id: "p1" });
    expect(out).toMatchObject({ id: "p1", expiresAt: "1893456000", createdAt: ISO, updatedAt: ISO });
  });

  it("returns null for a missing proposal", async () => {
    const db = makeDb();
    db.multisigProposal.findUnique.mockResolvedValue(null);
    const r = buildResolvers(db as never);
    expect(await r.Query.proposal({}, { id: "x" })).toBeNull();
  });

  it("filters proposals by subject and finalized flag", async () => {
    const db = makeDb();
    db.multisigProposal.findMany.mockResolvedValue([proposalRow]);
    const r = buildResolvers(db as never);
    const out = await r.Query.proposals({}, { subject: "S", finalized: false });
    expect(db.multisigProposal.findMany).toHaveBeenCalledWith({
      where: { subject: "S", finalized: false },
      orderBy: { createdAt: "desc" },
    });
    expect(out).toHaveLength(1);
    expect(out[0].expiresAt).toBe("1893456000");
  });
});

describe("Query.auditLog", () => {
  it("returns audit entries ordered by timestamp with serialised fields", async () => {
    const db = makeDb();
    db.auditEntry.findMany.mockResolvedValue([
      { id: 1, attestationId: "a1", action: "created", timestamp: 1700000000n, createdAt: D },
    ]);
    const r = buildResolvers(db as never);
    const out = await r.Query.auditLog({}, { attestationId: "a1" });
    expect(db.auditEntry.findMany).toHaveBeenCalledWith({
      where: { attestationId: "a1" },
      orderBy: { timestamp: "asc" },
    });
    expect(out[0]).toMatchObject({ timestamp: "1700000000", createdAt: ISO });
  });
});

describe("Subscription resolvers", () => {
  const r: AnyResolvers = buildResolvers(makeDb() as never);

  it("onAttestationCreated.resolve unwraps the payload", () => {
    const payload = { onAttestationCreated: { id: "a1" } } as never;
    expect(r.Subscription.onAttestationCreated.resolve(payload)).toEqual({ id: "a1" });
  });

  it("onAttestationRevoked.resolve unwraps the payload", () => {
    const payload = { onAttestationRevoked: { id: "a1", issuer: "i", subject: "s", claimType: "c", revokedAt: ISO } } as never;
    expect(r.Subscription.onAttestationRevoked.resolve(payload)).toMatchObject({ id: "a1" });
  });

  it("onIssuerRegistered.resolve unwraps the payload", () => {
    const payload = { onIssuerRegistered: { issuer: "i", registeredAt: ISO } } as never;
    expect(r.Subscription.onIssuerRegistered.resolve(payload)).toMatchObject({ issuer: "i" });
  });

  it("onAttestationCreated.subscribe yields nothing when 'created' is not in the topics allowlist", async () => {
    const iter: AsyncIterator<unknown> = r.Subscription.onAttestationCreated.subscribe({}, {
      topics: ["revoked"],
    }) as never;
    await expect(iter.next()).resolves.toEqual({ done: true, value: undefined });
  });

  it("onAttestationRevoked.subscribe yields nothing when 'revoked' is not in the topics allowlist", async () => {
    const iter: AsyncIterator<unknown> = r.Subscription.onAttestationRevoked.subscribe({}, {
      topics: ["created"],
    }) as never;
    await expect(iter.next()).resolves.toEqual({ done: true, value: undefined });
  });

  it("onIssuerRegistered.subscribe yields nothing when 'iss_reg' is not in the topics allowlist", async () => {
    const iter: AsyncIterator<unknown> = r.Subscription.onIssuerRegistered.subscribe({}, {
      topics: ["created"],
    }) as never;
    await expect(iter.next()).resolves.toEqual({ done: true, value: undefined });
  });
});

describe("cacheInvalidate", () => {
  it("is a no-op when redis is null", async () => {
    await expect(cacheInvalidate(null, "issuerStats:*")).resolves.toBeUndefined();
  });

  it("deletes every key matching the pattern", async () => {
    const redis = {
      keys: jest.fn().mockResolvedValue(["issuerStats:a", "issuerStats:b"]),
      del: jest.fn().mockResolvedValue(2),
    };
    await cacheInvalidate(redis as never, "issuerStats:*");
    expect(redis.keys).toHaveBeenCalledWith("issuerStats:*");
    expect(redis.del).toHaveBeenCalledWith("issuerStats:a", "issuerStats:b");
  });

  it("does not delete when nothing matches", async () => {
    const redis = { keys: jest.fn().mockResolvedValue([]), del: jest.fn() };
    await cacheInvalidate(redis as never, "issuerStats:*");
    expect(redis.del).not.toHaveBeenCalled();
  });

  it("swallows redis errors", async () => {
    const redis = { keys: jest.fn().mockRejectedValue(new Error("down")), del: jest.fn() };
    await expect(cacheInvalidate(redis as never, "x")).resolves.toBeUndefined();
  });
});
