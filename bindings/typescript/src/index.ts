/**
 * TrustLink Contract — TypeScript bindings
 *
 * Auto-generated from src/lib.rs, src/types.rs, and src/errors.rs.
 * Do NOT edit by hand — run `make bindings` to regenerate.
 *
 * @example
 * ```ts
 * import { TrustLinkClient, Networks } from "@trustlink/contract";
 *
 * const client = new TrustLinkClient({
 *   contractId: "C...",
 *   rpcUrl: "https://soroban-testnet.stellar.org",
 *   networkPassphrase: Networks.TESTNET,
 * });
 *
 * const hasKyc = await client.hasValidClaim(userAddress, "KYC_PASSED");
 * ```
 */

export * from "./types";
export * from "./client";
