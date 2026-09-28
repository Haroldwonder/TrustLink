import { describe, it, expect } from "vitest";
import { TrustLinkClient } from "../client";

// Valid Soroban contract strkey (32 zero bytes, correct base32 checksum)
const CONTRACT_ID = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4";

describe("TrustLinkClient", () => {
  it("constructs with default options", () => {
    const client = new TrustLinkClient({
      contractId: CONTRACT_ID,
      rpcUrl: "https://soroban-testnet.stellar.org",
    });
    expect(client).toBeInstanceOf(TrustLinkClient);
  });

  it("constructs with custom resilience options", () => {
    const client = new TrustLinkClient({
      contractId: CONTRACT_ID,
      rpcUrl: "https://soroban-testnet.stellar.org",
      resilience: { maxRetries: 5, backoffMs: 100, circuitBreakerThreshold: 2 },
    });
    expect(client).toBeInstanceOf(TrustLinkClient);
  });
});
