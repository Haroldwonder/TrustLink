/**
 * Unit tests for TrustLinkClient.hasValidClaimFromTier (Issue #531).
 *
 * These tests verify that the method:
 * 1. Correctly encodes the IssuerTier enum as a Soroban ScVec([ScSymbol]) value.
 * 2. Calls the contract method with the right arguments.
 * 3. Returns the decoded boolean result.
 */

import { xdr, nativeToScVal, scValToNative } from "@stellar/stellar-sdk";
import { TrustLinkClient } from "../src/client";
import type { IssuerTier } from "../src/types";

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Build the correct Soroban encoding for an IssuerTier enum variant. */
function encodeTier(tier: IssuerTier): xdr.ScVal {
  return xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(tier)]);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("TrustLinkClient.hasValidClaimFromTier", () => {
  const CONTRACT_ID = "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
  const SUBJECT = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
  const CLAIM_TYPE = "KYC_PASSED";

  let client: TrustLinkClient;
  let simulateSpy: jest.SpyInstance;

  beforeEach(() => {
    client = new TrustLinkClient({ contractId: CONTRACT_ID, network: "testnet" });
    // Spy on the private simulate method
    simulateSpy = jest
      .spyOn(client as any, "simulate")
      .mockResolvedValue(true);
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  test("calls contract with correct method name", async () => {
    await client.hasValidClaimFromTier(SUBJECT, CLAIM_TYPE, "Basic");
    expect(simulateSpy).toHaveBeenCalledWith(
      "has_valid_claim_from_tier",
      expect.anything(),
      expect.anything(),
      expect.anything()
    );
  });

  test.each<IssuerTier>(["Basic", "Verified", "Premium"])(
    "encodes %s tier as ScVec([ScSymbol])",
    async (tier) => {
      await client.hasValidClaimFromTier(SUBJECT, CLAIM_TYPE, tier);
      const tierArg = simulateSpy.mock.calls[0][3] as xdr.ScVal;

      // Should be a ScVec containing a single ScSymbol
      expect(tierArg.switch().name).toBe("scvVec");
      const vec = tierArg.vec();
      expect(vec).toHaveLength(1);
      expect(vec![0].switch().name).toBe("scvSymbol");
      expect(vec![0].sym().toString()).toBe(tier);
    }
  );

  test("returns true when simulate resolves true", async () => {
    simulateSpy.mockResolvedValue(true);
    const result = await client.hasValidClaimFromTier(SUBJECT, CLAIM_TYPE, "Verified");
    expect(result).toBe(true);
  });

  test("returns false when simulate resolves false", async () => {
    simulateSpy.mockResolvedValue(false);
    const result = await client.hasValidClaimFromTier(SUBJECT, CLAIM_TYPE, "Premium");
    expect(result).toBe(false);
  });

  test("encodes subject address correctly", async () => {
    await client.hasValidClaimFromTier(SUBJECT, CLAIM_TYPE, "Basic");
    const subjectArg = simulateSpy.mock.calls[0][1] as xdr.ScVal;
    expect(subjectArg.switch().name).toBe("scvAddress");
  });

  test("encodes claim type as string", async () => {
    await client.hasValidClaimFromTier(SUBJECT, CLAIM_TYPE, "Basic");
    const claimTypeArg = simulateSpy.mock.calls[0][2] as xdr.ScVal;
    expect(claimTypeArg.switch().name).toBe("scvString");
    expect(scValToNative(claimTypeArg)).toBe(CLAIM_TYPE);
  });
});

describe("IssuerTier encoding", () => {
  test("Basic encodes as ScVec([ScSymbol('Basic')])", () => {
    const encoded = encodeTier("Basic");
    expect(encoded.switch().name).toBe("scvVec");
    const vec = encoded.vec()!;
    expect(vec[0].sym().toString()).toBe("Basic");
  });

  test("Verified encodes as ScVec([ScSymbol('Verified')])", () => {
    const encoded = encodeTier("Verified");
    const vec = encoded.vec()!;
    expect(vec[0].sym().toString()).toBe("Verified");
  });

  test("Premium encodes as ScVec([ScSymbol('Premium')])", () => {
    const encoded = encodeTier("Premium");
    const vec = encoded.vec()!;
    expect(vec[0].sym().toString()).toBe("Premium");
  });

  test("nativeToScVal u32 encoding differs from correct ScVec encoding", () => {
    // Regression: the old implementation used nativeToScVal(n, {type:'u32'})
    // which produces ScvU32, not ScvVec. This test documents the difference.
    const wrongEncoding = nativeToScVal(0, { type: "u32" });
    const correctEncoding = encodeTier("Basic");

    expect(wrongEncoding.switch().name).toBe("scvU32");
    expect(correctEncoding.switch().name).toBe("scvVec");
    expect(wrongEncoding.switch().name).not.toBe(correctEncoding.switch().name);
  });
});
