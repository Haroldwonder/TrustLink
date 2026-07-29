/**
 * Per-Subject Attestation Limit Example
 *
 * This example demonstrates how to use TrustLink's per-subject attestation limit
 * feature to prevent unbounded growth and query performance degradation.
 *
 * Scenarios covered:
 * 1. Setting the per-subject limit
 * 2. Creating attestations up to the limit
 * 3. Handling limit-exceeded errors
 * 4. Querying the current limit
 * 5. Disabling the limit
 * 6. Creating bundles with per-subject limit enforcement
 */

import {
  TrustLinkClient,
  Address,
  Keypair,
  Networks,
} from "@trustlink/contract";

// Setup: Initialize client and accounts
async function setupExample() {
  // Contract configuration
  const contractId = "C..."; // Replace with your contract ID
  const rpcUrl = "https://soroban-testnet.stellar.org";
  const networkPassphrase = Networks.TESTNET_NETWORK_PASSPHRASE;

  // Create keypairs for participants
  const adminKeypair = Keypair.random();
  const issuerKeypair = Keypair.random();
  const subjectKeypair = Keypair.random();

  // Addresses
  const adminAddress = adminKeypair.publicKey();
  const issuerAddress = issuerKeypair.publicKey();
  const subjectAddress = subjectKeypair.publicKey();

  // Initialize client
  const client = new TrustLinkClient({
    contractId,
    rpcUrl,
    networkPassphrase,
  });

  return {
    client,
    adminAddress,
    issuerAddress,
    subjectAddress,
    adminKeypair,
    issuerKeypair,
    subjectKeypair,
  };
}

/**
 * Scenario 1: Set the per-subject attestation limit
 */
async function scenario1_SetLimit() {
  console.log("\n=== Scenario 1: Set Per-Subject Limit ===");
  const { client, adminAddress, adminKeypair } = await setupExample();

  try {
    // Set limit to 100 attestations per subject
    const result = await client.invoke({
      method: "set_max_attestations_per_subject",
      args: {
        admin: adminAddress,
        limit: 100,
      },
      signers: [adminKeypair],
    });

    console.log("✓ Set per-subject limit to 100 attestations");
    console.log(`  Transaction result: ${result.id}`);

    // Verify the limit was set
    const limit = await client.invoke({
      method: "get_max_attestations_per_subject",
    });

    console.log(`✓ Verified limit: ${limit} attestations per subject`);
  } catch (error) {
    console.error("✗ Failed to set limit:", error);
    throw error;
  }
}

/**
 * Scenario 2: Create attestations up to the limit
 */
async function scenario2_CreateAttestationsUpToLimit() {
  console.log("\n=== Scenario 2: Create Attestations Up To Limit ===");
  const {
    client,
    adminAddress,
    issuerAddress,
    subjectAddress,
    issuerKeypair,
  } = await setupExample();

  try {
    // First, set a small limit for demonstration
    const limitResult = await client.invoke({
      method: "set_max_attestations_per_subject",
      args: {
        admin: adminAddress,
        limit: 5, // Small limit for this example
      },
      signers: [adminKeypair],
    });
    console.log("✓ Set per-subject limit to 5 attestations (for demo)");

    // Create 5 attestations (up to limit)
    const attestationIds: string[] = [];
    for (let i = 1; i <= 5; i++) {
      const id = await client.invoke({
        method: "create_attestation",
        args: {
          issuer: issuerAddress,
          subject: subjectAddress,
          claimType: `CREDENTIAL_${i}`,
          expiration: null,
          metadata: `{"index":${i}}`,
          tags: [`demo-${i}`],
        },
        signers: [issuerKeypair],
      });
      attestationIds.push(id);
      console.log(`✓ Created attestation ${i}/5: ${id.substring(0, 8)}...`);
    }

    console.log(
      `\n✓ Successfully created 5 attestations (limit: 5, used: 5/5)`
    );
    return attestationIds;
  } catch (error) {
    console.error("✗ Failed to create attestations:", error);
    throw error;
  }
}

/**
 * Scenario 3: Attempt to exceed the limit
 */
async function scenario3_HandleLimitExceeded() {
  console.log("\n=== Scenario 3: Handle Limit-Exceeded Error ===");
  const {
    client,
    adminAddress,
    issuerAddress,
    subjectAddress,
    issuerKeypair,
  } = await setupExample();

  try {
    // Set limit to 2 for easy demonstration
    await client.invoke({
      method: "set_max_attestations_per_subject",
      args: {
        admin: adminAddress,
        limit: 2,
      },
      signers: [adminKeypair],
    });
    console.log("✓ Set per-subject limit to 2 attestations");

    // Create 2 attestations (at limit)
    for (let i = 1; i <= 2; i++) {
      await client.invoke({
        method: "create_attestation",
        args: {
          issuer: issuerAddress,
          subject: subjectAddress,
          claimType: `CLAIM_${i}`,
          expiration: null,
          metadata: null,
          tags: null,
        },
        signers: [issuerKeypair],
      });
      console.log(`✓ Created attestation ${i}/2`);
    }

    // Attempt to create 3rd attestation (exceeds limit)
    console.log("\n→ Attempting to exceed limit (3rd attestation)...");
    try {
      await client.invoke({
        method: "create_attestation",
        args: {
          issuer: issuerAddress,
          subject: subjectAddress,
          claimType: "CLAIM_3",
          expiration: null,
          metadata: null,
          tags: null,
        },
        signers: [issuerKeypair],
      });
      console.error(
        "✗ ERROR: Should have rejected 3rd attestation but succeeded!"
      );
    } catch (limitError) {
      if (limitError.message.includes("LimitExceeded")) {
        console.log("✓ Correctly rejected 3rd attestation (limit exceeded)");
        console.log(`  Error: ${limitError.message}`);
      } else {
        throw limitError;
      }
    }
  } catch (error) {
    console.error("✗ Scenario failed:", error);
    throw error;
  }
}

/**
 * Scenario 4: Query the current limit
 */
async function scenario4_QueryCurrentLimit() {
  console.log("\n=== Scenario 4: Query Current Limit ===");
  const { client, adminAddress, adminKeypair } = await setupExample();

  try {
    // Initially check limit (should be unlimited)
    let limit = await client.invoke({
      method: "get_max_attestations_per_subject",
    });
    console.log(`✓ Initial limit: ${limit || "unlimited"}`);

    // Set a limit
    await client.invoke({
      method: "set_max_attestations_per_subject",
      args: {
        admin: adminAddress,
        limit: 500,
      },
      signers: [adminKeypair],
    });

    // Query after setting
    limit = await client.invoke({
      method: "get_max_attestations_per_subject",
    });
    console.log(`✓ After setting: ${limit} attestations per subject`);

    // Query via contract config
    const config = await client.invoke({
      method: "get_config",
    });
    console.log(`✓ Via config: ${config.max_attestations_per_subject}`);
  } catch (error) {
    console.error("✗ Failed to query limit:", error);
    throw error;
  }
}

/**
 * Scenario 5: Disable the limit (set to unlimited)
 */
async function scenario5_DisableLimit() {
  console.log("\n=== Scenario 5: Disable Per-Subject Limit ===");
  const { client, adminAddress, adminKeypair } = await setupExample();

  try {
    // Set initial limit
    await client.invoke({
      method: "set_max_attestations_per_subject",
      args: {
        admin: adminAddress,
        limit: 100,
      },
      signers: [adminKeypair],
    });
    console.log("✓ Set limit to 100");

    // Verify it's set
    let limit = await client.invoke({
      method: "get_max_attestations_per_subject",
    });
    console.log(`✓ Current limit: ${limit}`);

    // Disable (set to null/None)
    await client.invoke({
      method: "set_max_attestations_per_subject",
      args: {
        admin: adminAddress,
        limit: null,
      },
      signers: [adminKeypair],
    });
    console.log("✓ Disabled per-subject limit (set to null)");

    // Verify it's disabled
    limit = await client.invoke({
      method: "get_max_attestations_per_subject",
    });
    console.log(`✓ Limit after disabling: ${limit || "unlimited"}`);
  } catch (error) {
    console.error("✗ Failed to disable limit:", error);
    throw error;
  }
}

/**
 * Scenario 6: Create bundle with per-subject limit enforcement
 */
async function scenario6_BundleWithLimitEnforcement() {
  console.log("\n=== Scenario 6: Bundle Creation With Limit Enforcement ===");
  const {
    client,
    adminAddress,
    issuerAddress,
    subjectAddress,
    issuerKeypair,
  } = await setupExample();

  try {
    // Set limit to 5
    await client.invoke({
      method: "set_max_attestations_per_subject",
      args: {
        admin: adminAddress,
        limit: 5,
      },
      signers: [adminKeypair],
    });
    console.log("✓ Set per-subject limit to 5");

    // Try to create bundle with 3 claims (within limit)
    const claimTypes = ["KYC_PASSED", "AGE_VERIFIED", "JURISDICTION_US"];
    const expiration =
      BigInt(Math.floor(Date.now() / 1000)) + BigInt(365 * 24 * 60 * 60);

    const bundleId = await client.invoke({
      method: "create_attestation_bundle",
      args: {
        issuer: issuerAddress,
        subject: subjectAddress,
        claim_types: claimTypes,
        expiration,
        metadata: '{"bundle_type":"kyc"}',
        tags: ["kyc_bundle"],
      },
      signers: [issuerKeypair],
    });
    console.log(`✓ Created bundle with 3 claims: ${bundleId.substring(0, 8)}...`);

    // Try to create another bundle with 3 claims (would total 6, exceeds 5)
    console.log("\n→ Attempting to create another 3-claim bundle (would exceed limit)...");
    try {
      await client.invoke({
        method: "create_attestation_bundle",
        args: {
          issuer: issuerAddress,
          subject: subjectAddress,
          claim_types: ["DOCUMENT_VERIFIED", "INCOME_VERIFIED", "CREDIT_CHECK"],
          expiration,
          metadata: '{"bundle_type":"compliance"}',
          tags: ["compliance_bundle"],
        },
        signers: [issuerKeypair],
      });
      console.error("✗ ERROR: Should have rejected second bundle!");
    } catch (limitError) {
      if (limitError.message.includes("LimitExceeded")) {
        console.log("✓ Correctly rejected second bundle (would exceed limit)");
        console.log(`  Current: 3/5, Requested: 3 more = 6/5 (exceeds!)`);
      } else {
        throw limitError;
      }
    }
  } catch (error) {
    console.error("✗ Scenario failed:", error);
    throw error;
  }
}

/**
 * Run all scenarios
 */
async function main() {
  console.log("╔════════════════════════════════════════════════════════════╗");
  console.log("║  TrustLink Per-Subject Attestation Limit Examples          ║");
  console.log("╚════════════════════════════════════════════════════════════╝");

  try {
    await scenario1_SetLimit();
    await scenario2_CreateAttestationsUpToLimit();
    await scenario3_HandleLimitExceeded();
    await scenario4_QueryCurrentLimit();
    await scenario5_DisableLimit();
    await scenario6_BundleWithLimitEnforcement();

    console.log("\n╔════════════════════════════════════════════════════════════╗");
    console.log("║  ✓ All scenarios completed successfully!                  ║");
    console.log("╚════════════════════════════════════════════════════════════╝");
  } catch (error) {
    console.error("\n✗ Example failed:", error);
    process.exit(1);
  }
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
