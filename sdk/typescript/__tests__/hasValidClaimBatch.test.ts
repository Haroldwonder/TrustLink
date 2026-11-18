import { jest } from "@jest/globals";
import { scValToNative } from "@stellar/stellar-sdk";
import { TrustLinkClient } from "../src/client";
import { InvalidAddressError, InvalidClaimTypeError } from "../src/validation";

describe("TrustLinkClient.hasValidClaimBatch", () => {
  const CONTRACT_ID = "CAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQC526";
  const SUBJECT_1 = "GDD4AYCIMALJPW2DPPM45Z2EEIS5X4PPH5ZPWQZY7EISHUZVDOFKZ2UH";
  const SUBJECT_2 = "GAG6NQTDDDCG7GZP5MWFOS4N44ZPJFDWGEFOWIV66DRPM5C6PMRHML4Z";
  const CLAIM_TYPE = "KYC_PASSED";

  let client: TrustLinkClient;
  let simulateSpy: ReturnType<typeof jest.spyOn>;

  beforeEach(() => {
    client = new TrustLinkClient({ contractId: CONTRACT_ID, network: "testnet" });
    simulateSpy = jest.spyOn(client as any, "simulate");
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  test("calls the contract with a vector of addresses and string argument", async () => {
    simulateSpy.mockResolvedValue([true, false]);

    const result = await client.hasValidClaimBatch([SUBJECT_1, SUBJECT_2], CLAIM_TYPE);

    expect(simulateSpy).toHaveBeenCalledWith(
      "has_valid_claim_batch",
      expect.anything(),
      expect.anything()
    );

    const [method, subjectsVal, claimTypeVal] = simulateSpy.mock.calls[0];
    expect(method).toBe("has_valid_claim_batch");
    expect(subjectsVal.switch().name).toBe("scvVec");
    expect(scValToNative(claimTypeVal)).toBe(CLAIM_TYPE);
    expect(result).toEqual([true, false]);
  });

  test("handles empty subjects array", async () => {
    simulateSpy.mockResolvedValue([]);

    const result = await client.hasValidClaimBatch([], CLAIM_TYPE);

    expect(simulateSpy).toHaveBeenCalledWith(
      "has_valid_claim_batch",
      expect.anything(),
      expect.anything()
    );
    expect(result).toEqual([]);
  });

  test("validates each subject address", async () => {
    await expect(
      client.hasValidClaimBatch([SUBJECT_1, "invalid-address"], CLAIM_TYPE)
    ).rejects.toThrow(InvalidAddressError);

    expect(simulateSpy).not.toHaveBeenCalled();
  });

  test("validates claim type", async () => {
    await expect(
      client.hasValidClaimBatch([SUBJECT_1], "invalid claim type with spaces")
    ).rejects.toThrow(InvalidClaimTypeError);

    expect(simulateSpy).not.toHaveBeenCalled();
  });

  test("throws TypeError if subjects is not an array", async () => {
    await expect(
      client.hasValidClaimBatch("not-an-array" as any, CLAIM_TYPE)
    ).rejects.toThrow(TypeError);

    expect(simulateSpy).not.toHaveBeenCalled();
  });
});
