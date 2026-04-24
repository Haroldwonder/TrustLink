/**
 * TrustLink TypeScript SDK — usage examples.
 *
 * Run with:
 *   npx ts-node examples/usage.ts
 */

import { TrustLinkClient, TrustLinkError } from "../src";

const CONTRACT_ID =
  process.env.CONTRACT_ID ??
  "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";

const USER_ADDRESS =
  process.env.USER_ADDRESS ??
  "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";

async function main() {
  const client = new TrustLinkClient({
    contractId: CONTRACT_ID,
    network: "testnet",
  });

  // ── Contract health ──────────────────────────────────────────────────────
  console.log("=== Contract Info ===");
  const health = await client.healthCheck();
  console.log("Health:", health);

  const stats = await client.getGlobalStats();
  console.log("Global stats:", stats);

  // ── Claim verification ───────────────────────────────────────────────────
  console.log("\n=== Claim Verification ===");

  const hasKyc = await client.hasValidClaim(USER_ADDRESS, "KYC_PASSED");
  console.log(`Has KYC_PASSED: ${hasKyc}`);

  const hasAny = await client.hasAnyClaim(USER_ADDRESS, [
    "KYC_PASSED",
    "ACCREDITED_INVESTOR",
    "MERCHANT_VERIFIED",
  ]);
  console.log(`Has any of KYC/ACCREDITED/MERCHANT: ${hasAny}`);

  const hasAll = await client.hasAllClaims(USER_ADDRESS, [
    "KYC_PASSED",
    "AML_CLEARED",
  ]);
  console.log(`Has all of KYC + AML: ${hasAll}`);

  // ── Attestation queries ──────────────────────────────────────────────────
  console.log("\n=== Attestation Queries ===");

  const count = await client.getSubjectAttestationCount(USER_ADDRESS);
  console.log(`Total attestations for subject: ${count}`);

  const validCount = await client.getValidClaimCount(USER_ADDRESS);
  console.log(`Valid claims: ${validCount}`);

  const page = await client.getSubjectAttestations(USER_ADDRESS, 0, 5);
  console.log(`First page (up to 5):`, page);

  // ── Claim type registry ──────────────────────────────────────────────────
  console.log("\n=== Claim Types ===");
  const claimTypes = await client.listClaimTypes(0, 20);
  console.log("Registered claim types:", claimTypes);

  for (const ct of claimTypes) {
    const desc = await client.getClaimTypeDescription(ct);
    console.log(`  ${ct}: ${desc}`);
  }

  // ── Error handling ───────────────────────────────────────────────────────
  console.log("\n=== Error Handling ===");
  try {
    await client.getAttestation("nonexistent-id");
  } catch (err) {
    // Contract errors surface as thrown Error objects with the error message.
    console.log("Expected error:", (err as Error).message);
    // You can map error codes using TrustLinkError enum:
    console.log("NotFound code:", TrustLinkError.NotFound); // 4
  }
}

main().catch(console.error);
