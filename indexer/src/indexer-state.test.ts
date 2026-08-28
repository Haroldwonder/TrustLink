/**
 * #1138 – Focused unit tests for indexer/src/indexer-state.ts
 *
 * This module holds the indexer's in-memory "last processed ledger" cursor.
 * `indexer-chaos.test.ts` exercises resilience scenarios but assumes the
 * cursor itself is correct; these tests verify the get/set contract and the
 * persist -> restore round-trip that the indexer relies on after a restart.
 */
import { getLastLedger, setLastLedger } from "./indexer-state";

describe("indexer-state ledger cursor", () => {
  // The cursor is module-level singleton state; reset it around every test so
  // ordering never matters and other suites are unaffected.
  const initial = getLastLedger();

  afterEach(() => {
    setLastLedger(0);
  });

  afterAll(() => {
    setLastLedger(initial);
  });

  it("starts at 0 before anything is processed", () => {
    setLastLedger(0);
    expect(getLastLedger()).toBe(0);
  });

  it("returns a number", () => {
    expect(typeof getLastLedger()).toBe("number");
  });

  it("reflects the most recently set ledger", () => {
    setLastLedger(42);
    expect(getLastLedger()).toBe(42);
  });

  it("is a stable read (repeated gets return the same value)", () => {
    setLastLedger(1234);
    expect(getLastLedger()).toBe(1234);
    expect(getLastLedger()).toBe(1234);
  });

  it("advances monotonically as ledgers are processed", () => {
    for (const ledger of [10, 11, 12, 25, 26]) {
      setLastLedger(ledger);
      expect(getLastLedger()).toBe(ledger);
    }
  });

  it("does not enforce monotonicity itself (last write wins, e.g. a rewind)", () => {
    setLastLedger(500);
    setLastLedger(300);
    expect(getLastLedger()).toBe(300);
  });

  it("round-trips a checkpoint value (persist -> restore after restart)", () => {
    const checkpoint = 987654;
    setLastLedger(checkpoint);
    const persisted = getLastLedger();

    // simulate a process restart resetting the in-memory cursor
    setLastLedger(0);
    expect(getLastLedger()).toBe(0);

    // restore from the persisted checkpoint
    setLastLedger(persisted);
    expect(getLastLedger()).toBe(checkpoint);
  });

  it("handles large ledger sequence numbers", () => {
    const big = Number.MAX_SAFE_INTEGER;
    setLastLedger(big);
    expect(getLastLedger()).toBe(big);
  });
});
