/**
 * TrustLink Attestation Bundles Example (TypeScript)
 *
 * This example demonstrates how to create and verify attestation bundles.
 * Bundles allow issuing multiple related attestations atomically with a shared bundle ID.
 */

import {
  TrustLinkClient,
  createAttestationBundle,
  getBundleWithAttestations,
  verifyBundleValidity,
  verifyAttestationsInSameBundle,
  verifyBundleClaimTypes,
  verifyBundleSize,
  getIssuerBundles,
  getSubjectBundles,
} from "@trustlink/contract";

// ─── Example 1: Create a KYC Bundle ────────────────────────────────────────

async function example1_CreateKycBundle() {
  console.log("=== Example 1: Create a KYC Bundle ===\n");

  const client = new TrustLinkClient({
    contractId: process.env.CONTRACT_ID || "C...",
    rpcUrl: "https://soroban-testnet.stellar.org",
  });

  const issuerAddress = process.env.ISSUER_ADDRESS || "GBRPYHIL2CI3...";
  const subjectAddress = process.env.SUBJECT_ADDRESS || "GBBD5YHQVW53...";

  // Create a bundle with KYC-related claims
  const bundleId = await createAttestationBundle(client, {
    issuer: issuerAddress,
    subject: subjectAddress,
    claimTypes: ["KYC_PASSED", "AGE_VERIFIED", "JURISDICTION_US"],
    expiration: BigInt(Math.floor(Date.now() / 1000)) + BigInt(365 * 24 * 60 * 60),
    metadata: JSON.stringify({
      kyc_level: "enhanced",
      checked_at: new Date().toISOString(),
      check_method: "video_call",
    }),
  });

  console.log(`✓ Bundle created: ${bundleId}`);
  console.log(`  Contains 3 attestations: KYC_PASSED, AGE_VERIFIED, JURISDICTION_US\n`);
}

// ─── Example 2: Verify Bundle Membership ──────────────────────────────────

async function example2_VerifyBundleMembership() {
  console.log("=== Example 2: Verify Bundle Membership ===\n");

  const client = new TrustLinkClient({
    contractId: process.env.CONTRACT_ID || "C...",
    rpcUrl: "https://soroban-testnet.stellar.org",
  });

  const bundleId = process.argv[3] || "bundle_id_here";

  // Get bundle and its attestations
  const { bundle, attestations } = await getBundleWithAttestations(client, bundleId);

  console.log(`Bundle: ${bundle.id}`);
  console.log(`  Issuer: ${bundle.issuer}`);
  console.log(`  Subject: ${bundle.subject}`);
  console.log(`  Created: ${new Date(Number(bundle.timestamp) * 1000).toISOString()}`);
  console.log(`  Claim Types: ${bundle.claim_types.join(", ")}`);
  console.log(`  Attestation Count: ${attestations.length}`);
  console.log(`  All Valid: ${bundle.all_valid}\n`);

  // Verify all attestations are from the same bundle
  const commonBundleId = verifyAttestationsInSameBundle(attestations);
  if (commonBundleId === bundleId) {
    console.log("✓ All attestations are from the same bundle\n");
  } else {
    console.log("✗ Attestations are not all from the same bundle\n");
  }

  // Show individual attestations
  console.log("Individual Attestations:");
  for (const att of attestations) {
    console.log(`  - ${att.claim_type}: ${att.id}`);
    console.log(`    Issuer: ${att.issuer}`);
    console.log(`    Valid: ${!att.revoked}`);
  }
  console.log();
}

// ─── Example 3: Verify Bundle Integrity ───────────────────────────────────

async function example3_VerifyBundleIntegrity() {
  console.log("=== Example 3: Verify Bundle Integrity ===\n");

  const client = new TrustLinkClient({
    contractId: process.env.CONTRACT_ID || "C...",
    rpcUrl: "https://soroban-testnet.stellar.org",
  });

  const bundleId = process.argv[3] || "bundle_id_here";

  // Check bundle validity
  const isValid = await verifyBundleValidity(client, bundleId);
  console.log(`Bundle is valid: ${isValid}`);

  // Get bundle details
  const bundle = await client.getBundle(bundleId);

  // Verify expected claim types
  const expectedClaims = ["KYC_PASSED", "AGE_VERIFIED", "JURISDICTION_US"];
  const hasExpectedClaims = verifyBundleClaimTypes(bundle, expectedClaims);
  console.log(`Has expected claim types: ${hasExpectedClaims}`);

  // Verify bundle size
  const expectedSize = 3;
  const hasExpectedSize = verifyBundleSize(bundle, expectedSize);
  console.log(`Has expected size (${expectedSize}): ${hasExpectedSize}\n`);
}

// ─── Example 4: Query Bundles ──────────────────────────────────────────────

async function example4_QueryBundles() {
  console.log("=== Example 4: Query Bundles ===\n");

  const client = new TrustLinkClient({
    contractId: process.env.CONTRACT_ID || "C...",
    rpcUrl: "https://soroban-testnet.stellar.org",
  });

  const issuerAddress = process.env.ISSUER_ADDRESS || "GBRPYHIL2CI3...";
  const subjectAddress = process.env.SUBJECT_ADDRESS || "GBBD5YHQVW53...";

  // Get all bundles created by issuer
  const issuerBundles = await getIssuerBundles(client, issuerAddress);
  console.log(`Bundles created by issuer: ${issuerBundles.length}`);
  for (const bundleId of issuerBundles.slice(0, 5)) {
    console.log(`  - ${bundleId}`);
  }
  console.log();

  // Get all bundles issued to subject
  const subjectBundles = await getSubjectBundles(client, subjectAddress);
  console.log(`Bundles issued to subject: ${subjectBundles.length}`);
  for (const bundleId of subjectBundles.slice(0, 5)) {
    console.log(`  - ${bundleId}`);
  }
  console.log();
}

// ─── Example 5: Compliance Bundle Scenario ────────────────────────────────

async function example5_ComplianceBundle() {
  console.log("=== Example 5: Compliance Bundle Scenario ===\n");

  const client = new TrustLinkClient({
    contractId: process.env.CONTRACT_ID || "C...",
    rpcUrl: "https://soroban-testnet.stellar.org",
  });

  const complianceIssuer = process.env.COMPLIANCE_ISSUER || "GBRPYHIL2CI3...";
  const organizationAddress = process.env.ORG_ADDRESS || "GBBD5YHQVW53...";

  // Create a compliance bundle with multiple certifications
  const bundleId = await createAttestationBundle(client, {
    issuer: complianceIssuer,
    subject: organizationAddress,
    claimTypes: ["SOC_2_CERTIFIED", "ISO_27001_APPROVED", "PCI_DSS_COMPLIANT", "GDPR_COMPLIANT"],
    expiration: BigInt(Math.floor(Date.now() / 1000)) + BigInt(365 * 24 * 60 * 60),
    metadata: JSON.stringify({
      audit_date: new Date().toISOString(),
      audit_firm: "Big Four Auditor",
      compliance_level: "critical_infrastructure",
    }),
  });

  console.log(`✓ Compliance bundle created: ${bundleId}`);
  console.log(`  Contains 4 compliance certifications`);
  console.log(`  All issued atomically to ensure consistency\n`);

  // Verify the compliance bundle
  const bundle = await client.getBundle(bundleId);
  console.log("Compliance Attestations:");
  for (let i = 0; i < bundle.claim_types.length; i++) {
    console.log(`  ${i + 1}. ${bundle.claim_types[i]}`);
    console.log(`     ID: ${bundle.attestation_ids[i]}`);
  }
  console.log();
}

// ─── Example 6: Error Handling ────────────────────────────────────────────

async function example6_ErrorHandling() {
  console.log("=== Example 6: Error Handling ===\n");

  const client = new TrustLinkClient({
    contractId: process.env.CONTRACT_ID || "C...",
    rpcUrl: "https://soroban-testnet.stellar.org",
  });

  const issuerAddress = process.env.ISSUER_ADDRESS || "GBRPYHIL2CI3...";
  const subjectAddress = process.env.SUBJECT_ADDRESS || "GBBD5YHQVW53...";

  try {
    // Try to create a bundle with too many claims (should fail)
    const tooManyClaims = Array.from({ length: 60 }, (_, i) => `CLAIM_${i}`);
    await createAttestationBundle(client, {
      issuer: issuerAddress,
      subject: subjectAddress,
      claimTypes: tooManyClaims,
    });
  } catch (error: any) {
    console.log(`✓ Caught expected error: ${error.message}`);
    console.log(`  Bundle size is limited to 50 attestations\n`);
  }

  try {
    // Try to create a bundle with same issuer and subject (should fail)
    await createAttestationBundle(client, {
      issuer: issuerAddress,
      subject: issuerAddress, // Same as issuer - invalid
      claimTypes: ["CLAIM_TYPE"],
    });
  } catch (error: any) {
    console.log(`✓ Caught expected error: ${error.message}`);
    console.log(`  Issuers cannot create attestations for themselves\n`);
  }
}

// ─── Main: Run examples ────────────────────────────────────────────────────

async function main() {
  console.log("TrustLink Attestation Bundles Examples\n");
  console.log("Select an example to run:\n");
  console.log("1. Create a KYC Bundle");
  console.log("2. Verify Bundle Membership");
  console.log("3. Verify Bundle Integrity");
  console.log("4. Query Bundles");
  console.log("5. Compliance Bundle Scenario");
  console.log("6. Error Handling\n");

  const example = process.argv[2] || "1";

  try {
    switch (example) {
      case "1":
        await example1_CreateKycBundle();
        break;
      case "2":
        await example2_VerifyBundleMembership();
        break;
      case "3":
        await example3_VerifyBundleIntegrity();
        break;
      case "4":
        await example4_QueryBundles();
        break;
      case "5":
        await example5_ComplianceBundle();
        break;
      case "6":
        await example6_ErrorHandling();
        break;
      default:
        console.log(`Unknown example: ${example}`);
    }
  } catch (error) {
    console.error("Error:", error);
  }
}

main();
