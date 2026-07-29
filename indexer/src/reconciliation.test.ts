/**
 * Tests for the indexer ↔ contract reconciliation job.
 *
 * Verifies that a deliberately introduced indexing discrepancy is detected
 * and reported via the drift result (and Prometheus counters).
 */

import {
  reconcileAttestations,
  OnChainAttestation,
  scheduleReconciliationJob,
} from "./reconciliation";
import { register } from "prom-client";

function makeIndexedRow(overrides: Record<string, unknown> = {}) {
  return {
    id: "att-1",
    issuer: "GISSUER",
    subject: "GSUBJECT",
    claimType: "KYC_PASSED",
    timestamp: BigInt(1_700_000_000),
    expiration: null,
    isRevoked: false,
    revocationReason: null,
    metadata: null,
    imported: false,
    bridged: false,
    sourceChain: null,
    sourceTx: null,
    createdAt: new Date("2024-01-01T00:00:00Z"),
    updatedAt: new Date("2024-01-01T00:00:00Z"),
    ...overrides,
  };
}

function makeOnChain(
  overrides: Partial<OnChainAttestation> = {},
): OnChainAttestation {
  return {
    id: "att-1",
    issuer: "GISSUER",
    subject: "GSUBJECT",
    claim_type: "KYC_PASSED",
    timestamp: 1_700_000_000,
    expiration: null,
    revoked: false,
    metadata: null,
    ...overrides,
  };
}

function makeMockDb(rows: ReturnType<typeof makeIndexedRow>[]) {
  return {
    attestation: {
      findMany: jest.fn().mockResolvedValue(rows),
    },
  };
}

describe("reconcileAttestations", () => {
  beforeEach(() => {
    register.resetMetrics();
  });

  it("reports no drift when indexed and on-chain state match", async () => {
    const db = makeMockDb([makeIndexedRow()]);
    const fetchOnChain = jest.fn().mockResolvedValue(makeOnChain());

    const result = await reconcileAttestations(db as never, fetchOnChain, {
      sampleSize: 10,
    });

    expect(result.checked).toBe(1);
    expect(result.drifts).toHaveLength(0);
    expect(result.missingOnChain).toBe(0);
    expect(fetchOnChain).toHaveBeenCalledWith("att-1");
  });

  it("detects and reports a deliberately introduced claimType discrepancy", async () => {
    // Indexed row has the wrong claim type — simulates an indexing bug.
    const db = makeMockDb([
      makeIndexedRow({ claimType: "WRONG_CLAIM" }),
    ]);
    const fetchOnChain = jest.fn().mockResolvedValue(
      makeOnChain({ claim_type: "KYC_PASSED" }),
    );

    const result = await reconcileAttestations(db as never, fetchOnChain, {
      sampleSize: 10,
    });

    expect(result.checked).toBe(1);
    expect(result.drifts).toHaveLength(1);
    expect(result.drifts[0].attestationId).toBe("att-1");
    expect(result.drifts[0].fields).toContain("claimType");
    expect(result.drifts[0].indexed.claimType).toBe("WRONG_CLAIM");
    expect(result.drifts[0].onChain.claimType).toBe("KYC_PASSED");
  });

  it("detects isRevoked drift against on-chain revoked flag", async () => {
    const db = makeMockDb([makeIndexedRow({ isRevoked: false })]);
    const fetchOnChain = jest
      .fn()
      .mockResolvedValue(makeOnChain({ revoked: true }));

    const result = await reconcileAttestations(db as never, fetchOnChain);

    expect(result.drifts).toHaveLength(1);
    expect(result.drifts[0].fields).toContain("isRevoked");
  });

  it("reports missing_on_chain when the contract has no row for an indexed id", async () => {
    const db = makeMockDb([makeIndexedRow()]);
    const fetchOnChain = jest.fn().mockResolvedValue(null);

    const result = await reconcileAttestations(db as never, fetchOnChain);

    expect(result.missingOnChain).toBe(1);
    expect(result.drifts[0].fields).toEqual(["missing_on_chain"]);
  });

  it("samples only the requested attestation IDs when provided", async () => {
    const db = makeMockDb([makeIndexedRow({ id: "att-only" })]);
    const fetchOnChain = jest.fn().mockResolvedValue(
      makeOnChain({ id: "att-only" }),
    );

    await reconcileAttestations(db as never, fetchOnChain, {
      attestationIds: ["att-only"],
    });

    expect(db.attestation.findMany).toHaveBeenCalledWith({
      where: { id: { in: ["att-only"] } },
    });
  });
});

describe("scheduleReconciliationJob", () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("runs on the configured schedule", async () => {
    const db = makeMockDb([]);
    const timer = scheduleReconciliationJob(db as never, {
      intervalMinutes: 1,
      sampleSize: 5,
      contractId: "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
      rpcUrl: "http://localhost:8000/soroban/rpc",
      runImmediately: false,
    });

    expect(timer).toBeDefined();

    // Advance one interval — the scheduled callback should fire.
    jest.advanceTimersByTime(60_000);
    // Allow any pending promise microtasks from the interval callback.
    await Promise.resolve();

    clearInterval(timer);
    expect(db.attestation.findMany).toHaveBeenCalled();
  });
});
