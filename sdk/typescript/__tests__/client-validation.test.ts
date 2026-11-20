import { TrustLinkClient } from "../src/client";

describe("TrustLinkClient constructor validation", () => {
  const CONTRACT_ID = "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";

  test("accepts valid network names", () => {
    expect(() => new TrustLinkClient({ contractId: CONTRACT_ID, network: "testnet" })).not.toThrow();
    expect(() => new TrustLinkClient({ contractId: CONTRACT_ID, network: "mainnet" })).not.toThrow();
    expect(() => new TrustLinkClient({ contractId: CONTRACT_ID, network: "local" })).not.toThrow();
  });

  test("throws clear error for invalid network name", () => {
    const error = "Invalid network: \"testnest\". Valid networks are: testnet, mainnet, local. For custom RPC URLs, use the 'rpcUrl' option instead.";
    expect(() => new TrustLinkClient({ contractId: CONTRACT_ID, network: "testnest" as any })).toThrow(error);
  });

  test("throws clear error for misspelled network name", () => {
    const error = "Invalid network: \"mainet\". Valid networks are: testnet, mainnet, local. For custom RPC URLs, use the 'rpcUrl' option instead.";
    expect(() => new TrustLinkClient({ contractId: CONTRACT_ID, network: "mainet" as any })).toThrow(error);
  });

  test("throws clear error for arbitrary string", () => {
    const error = "Invalid network: \"https://custom-rpc.com\". Valid networks are: testnet, mainnet, local. For custom RPC URLs, use the 'rpcUrl' option instead.";
    expect(() => new TrustLinkClient({ contractId: CONTRACT_ID, network: "https://custom-rpc.com" as any })).toThrow(error);
  });

  test("accepts valid rpcUrl", () => {
    expect(() => new TrustLinkClient({
      contractId: CONTRACT_ID,
      network: "testnet",
      rpcUrl: "https://custom-rpc.com"
    })).not.toThrow();
  });

  test("throws error for invalid rpcUrl", () => {
    expect(() => new TrustLinkClient({
      contractId: CONTRACT_ID,
      network: "testnet",
      rpcUrl: "not-a-valid-url"
    })).toThrow("Invalid rpcUrl: \"not-a-valid-url\" is not a valid URL.");
  });
});

describe("TrustLinkClient.hasValidClaimBatch validation", () => {
  const CONTRACT_ID = "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
  const SUBJECT_A = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
  const SUBJECT_B = "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";

  const makeClient = () => new TrustLinkClient({ contractId: CONTRACT_ID, network: "testnet" });

  test("rejects a non-array subjects list", async () => {
    const client = makeClient();
    await expect(client.hasValidClaimBatch(SUBJECT_A as any, "kyc")).rejects.toThrow(
      "subjects must be an array of addresses."
    );
  });

  test("rejects an empty subjects list", async () => {
    const client = makeClient();
    await expect(client.hasValidClaimBatch([], "kyc")).rejects.toThrow(
      "subjects must not be empty."
    );
  });

  test("rejects an invalid address in the subjects list", async () => {
    const client = makeClient();
    await expect(client.hasValidClaimBatch([SUBJECT_A, "not-an-address"], "kyc")).rejects.toThrow(
      "Invalid subject address at index 1: \"not-an-address\"."
    );
  });

  test("rejects an invalid claimType", async () => {
    const client = makeClient();
    await expect(client.hasValidClaimBatch([SUBJECT_A], "" as any)).rejects.toThrow(
      "claimType must be a non-empty string."
    );
  });

  test("returns a boolean array aligned with the input subjects", async () => {
    const client = makeClient();
    const simulate = jest
      .spyOn(client as any, "simulate")
      .mockResolvedValue([true, false]);

    const result = await client.hasValidClaimBatch([SUBJECT_A, SUBJECT_B], "kyc");

    expect(result).toEqual([true, false]);
    expect(Array.isArray(result)).toBe(true);
    expect(result).toHaveLength(2);
    expect(simulate).toHaveBeenCalledTimes(1);

    simulate.mockRestore();
  });
});
