/**
 * Tests for the core ledger-event indexing loop (processRange/handleEvent).
 *
 * Covers:
 *  - handleEvent: ignores unwatched topics, upserts attestations for "created"
 *  - processRange: cursor advancement via checkpoint upsert, dead-letter routing on failure
 */

import { processRange, handleEvent } from "./indexer";

jest.mock("./graphql", () => ({
  pubsub: { publish: jest.fn() },
  ATTESTATION_CREATED: "ATTESTATION_CREATED",
  ATTESTATION_REVOKED: "ATTESTATION_REVOKED",
  ISSUER_REGISTERED: "ISSUER_REGISTERED",
  cacheInvalidate: jest.fn(),
}));

jest.mock("./webhooks", () => ({
  dispatchWebhooks: jest.fn().mockResolvedValue(undefined),
}));

function makeMockDb() {
  return {
    attestation: {
      upsert: jest.fn().mockResolvedValue({
        id: "att-1",
        issuer: "issuer-1",
        subject: "subject-1",
        claimType: "kyc",
        timestamp: BigInt(1000),
        expiration: null,
        createdAt: new Date("2024-01-01T00:00:00Z"),
        updatedAt: new Date("2024-01-01T00:00:00Z"),
      }),
      count: jest.fn().mockResolvedValue(0),
    },
    auditEntry: { create: jest.fn().mockResolvedValue(undefined) },
    checkpoint: { upsert: jest.fn().mockResolvedValue(undefined) },
    eventDeadLetter: { create: jest.fn().mockResolvedValue(undefined) },
    issuer: { count: jest.fn().mockResolvedValue(0) },
  };
}

function makeCreatedEvent(ledger: number) {
  return {
    ledger,
    contractId: "CTEST",
    ledgerClosedAt: "2024-01-01T00:00:00Z",
    topic: [{ __topic: "created" }, { __topic: "subject-1" }],
    value: { __data: ["att-1", "issuer-1", "kyc", 1000] },
  };
}

jest.mock("@stellar/stellar-sdk", () => ({
  scValToNative: (v: unknown) => {
    if (v && typeof v === "object" && "__topic" in (v as Record<string, unknown>)) {
      return (v as { __topic: string }).__topic;
    }
    if (v && typeof v === "object" && "__data" in (v as Record<string, unknown>)) {
      return (v as { __data: unknown }).__data;
    }
    return v;
  },
  rpc: { Server: jest.fn() },
}));

beforeEach(() => {
  jest.clearAllMocks();
});

describe("handleEvent", () => {
  it("ignores events with unwatched topics", async () => {
    const db = makeMockDb();
    const ev = {
      ledger: 1,
      topic: [{ __topic: "some_other_topic" }],
      value: { __data: [] },
    };
    await handleEvent(db as never, ev as never, null);
    expect(db.attestation.upsert).not.toHaveBeenCalled();
  });

  it("upserts an attestation for a 'created' event", async () => {
    const db = makeMockDb();
    const ev = makeCreatedEvent(100);
    await handleEvent(db as never, ev as never, null);
    expect(db.attestation.upsert).toHaveBeenCalledWith(
      expect.objectContaining({ where: { id: "att-1" } }),
    );
    expect(db.auditEntry.create).toHaveBeenCalled();
  });
});

describe("processRange", () => {
  it("advances the checkpoint past the last processed ledger", async () => {
    const db = makeMockDb();
    const rpc = {
      getEvents: jest.fn().mockResolvedValue({ events: [makeCreatedEvent(50)] }),
    };

    const nextCursor = await processRange(db as never, rpc as never, 50, 50, null);

    expect(db.checkpoint.upsert).toHaveBeenCalledWith(
      expect.objectContaining({
        update: { ledger: 50 },
        create: { id: 1, ledger: 50 },
      }),
    );
    expect(nextCursor).toBe(51);
  });

  it("routes failed events to the dead-letter table and still advances the checkpoint", async () => {
    const db = makeMockDb();
    db.attestation.upsert.mockRejectedValueOnce(new Error("db unavailable"));
    const rpc = {
      getEvents: jest.fn().mockResolvedValue({ events: [makeCreatedEvent(75)] }),
    };

    await processRange(db as never, rpc as never, 75, 75, null);

    expect(db.eventDeadLetter.create).toHaveBeenCalledWith(
      expect.objectContaining({
        data: expect.objectContaining({ ledger: 75, status: "PENDING" }),
      }),
    );
    expect(db.checkpoint.upsert).toHaveBeenCalledWith(
      expect.objectContaining({ update: { ledger: 75 } }),
    );
  });
});
