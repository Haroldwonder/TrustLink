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
