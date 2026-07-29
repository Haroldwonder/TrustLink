#!/usr/bin/env node

/**
 * Freelance Marketplace Reputation Example
 *
 * Demonstrates composing TrustLink's endorsement system with IssuerTier to
 * produce a weighted reputation score for a freelancer.
 *
 * Scenario:
 *   Three marketplace clients at different trust tiers each complete a project
 *   with the freelancer and issue a JOB_COMPLETED attestation. They then
 *   cross-endorse one another's attestations. The platform computes a
 *   reputation score that weights each endorsement by the endorser's tier:
 *
 *     Basic   = 1 point per endorsement
 *     Verified = 2 points per endorsement
 *     Premium  = 3 points per endorsement
 *
 *   A freelancer whose work is endorsed primarily by Premium-tier clients
 *   therefore ranks higher than one with the same endorsement count from
 *   Basic-tier clients.
 *
 * Prerequisites:
 *   - All three client addresses must be registered as TrustLink issuers.
 *   - Their tiers must be set by the contract admin:
 *       set_issuer_tier(admin, clientA, Basic)
 *       set_issuer_tier(admin, clientB, Verified)
 *       set_issuer_tier(admin, clientC, Premium)
 *   - Copy .env.example to .env and fill in all values.
 *
 * Run:
 *   npm install && node index.mjs
 */

import {
  Contract,
  Networks,
  SorobanRpc,
  TransactionBuilder,
  Keypair,
  nativeToScVal,
  scValToNative,
  Address,
} from "@stellar/stellar-sdk";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const cfg = {
  rpcUrl: process.env.RPC_URL || "https://soroban-testnet.stellar.org",
  networkPassphrase: process.env.NETWORK_PASSPHRASE || Networks.TESTNET,
  contractId: process.env.TRUSTLINK_CONTRACT_ID || "",
  freelancerAddress: process.env.FREELANCER_ADDRESS || "",
  clientASecret: process.env.CLIENT_A_SECRET || "", // Basic tier
  clientBSecret: process.env.CLIENT_B_SECRET || "", // Verified tier
  clientCSecret: process.env.CLIENT_C_SECRET || "", // Premium tier
};

const TIER_WEIGHTS = { Basic: 1, Verified: 2, Premium: 3 };
const JOB_CLAIM_TYPE = "JOB_COMPLETED";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function required(value, name) {
  if (!value) {
    console.error(`Error: Missing ${name}. Copy .env.example to .env and fill in all values.`);
    process.exit(1);
  }
}

/**
 * Normalize IssuerTier values returned by scValToNative.
 * Soroban contracttype enums may be decoded as ["Basic"], {Basic:null}, or "Basic"
 * depending on the SDK version, so we handle all forms.
 */
function parseTierName(raw) {
  if (typeof raw === "string") return raw;
  if (Array.isArray(raw) && raw.length > 0) return String(raw[0]);
  if (raw && typeof raw === "object") return Object.keys(raw)[0] ?? "Basic";
  return "Basic";
}

async function simulateRead(server, sourceAddress, operation) {
  const account = await server.getAccount(sourceAddress);
  const tx = new TransactionBuilder(account, {
    fee: "100",
    networkPassphrase: cfg.networkPassphrase,
  })
    .addOperation(operation)
    .setTimeout(30)
    .build();

  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) {
    throw new Error(`Simulation failed: ${sim.error}`);
  }
  return sim.result?.retval;
}

async function submitWrite(server, sourceKeypair, operation) {
  const account = await server.getAccount(sourceKeypair.publicKey());
  let tx = new TransactionBuilder(account, {
    fee: "1000000",
    networkPassphrase: cfg.networkPassphrase,
  })
    .addOperation(operation)
    .setTimeout(60)
    .build();

  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) {
    throw new Error(`Write simulation failed: ${sim.error}`);
  }

  tx = SorobanRpc.assembleTransaction(tx, sim, cfg.networkPassphrase);
  tx.sign(sourceKeypair);

  const sent = await server.sendTransaction(tx);
  if (sent.status === "ERROR") {
    throw new Error(`Transaction failed: ${sent.errorResultXdr || "unknown"}`);
  }

  const hash = sent.hash;
  while (true) {
    const res = await server.getTransaction(hash);
    if (res.status === "SUCCESS") return res;
    if (res.status === "FAILED") throw new Error("Transaction status FAILED");
    await new Promise((r) => setTimeout(r, 1200));
  }
}

// ---------------------------------------------------------------------------
// Domain operations
// ---------------------------------------------------------------------------

/**
 * Issue a JOB_COMPLETED attestation from a client to the freelancer.
 * Returns the newly created attestation ID.
 */
async function issueJobCompletion(server, contract, clientKp, jobRef) {
  const expiration = Math.floor(Date.now() / 1000) + 2 * 365 * 24 * 60 * 60;
  const metadata = JSON.stringify({
    job_ref: jobRef,
    completed_at: new Date().toISOString(),
  });

  const op = contract.call(
    "create_attestation",
    nativeToScVal(Address.fromString(clientKp.publicKey()), { type: "address" }),
    nativeToScVal(Address.fromString(cfg.freelancerAddress), { type: "address" }),
    nativeToScVal(JOB_CLAIM_TYPE, { type: "string" }),
    nativeToScVal(BigInt(expiration), { type: "u64" }),
    nativeToScVal(metadata, { type: "string" })
  );

  const res = await submitWrite(server, clientKp, op);
  return res.returnValue ? scValToNative(res.returnValue) : null;
}

/**
 * Endorse an existing JOB_COMPLETED attestation.
 * The endorser must be a registered issuer and cannot endorse their own attestation.
 */
async function endorseAttestation(server, contract, endorserKp, attestationId) {
  const op = contract.call(
    "endorse_attestation",
    nativeToScVal(Address.fromString(endorserKp.publicKey()), { type: "address" }),
    nativeToScVal(attestationId, { type: "string" })
  );
  await submitWrite(server, endorserKp, op);
}

/**
 * Fetch the IssuerTier for a given issuer address.
 * Returns "Basic" if no tier is set.
 */
async function getIssuerTier(server, contract, issuerAddress, sourceAddress) {
  const op = contract.call(
    "get_issuer_tier",
    nativeToScVal(Address.fromString(issuerAddress), { type: "address" })
  );
  const retval = await simulateRead(server, sourceAddress, op);
  const raw = retval ? scValToNative(retval) : null;
  return raw ? parseTierName(raw) : "Basic";
}

/**
 * Fetch all endorsements for an attestation.
 */
async function getEndorsements(server, contract, attestationId, sourceAddress) {
  const op = contract.call(
    "get_endorsements",
    nativeToScVal(attestationId, { type: "string" })
  );
  const retval = await simulateRead(server, sourceAddress, op);
  return retval ? scValToNative(retval) : [];
}

// ---------------------------------------------------------------------------
// Reputation scoring
// ---------------------------------------------------------------------------

/**
 * Compute a weighted reputation score for the freelancer across all their
 * JOB_COMPLETED attestations.
 *
 * Score formula:
 *   For each attestation:
 *     For each endorsement of that attestation:
 *       score += TIER_WEIGHTS[endorser_tier]
 *
 * Returns the total score and a per-attestation breakdown.
 */
async function computeReputationScore(server, contract, attestationIds, sourceAddress) {
  let totalScore = 0;
  const breakdown = [];

  for (const attestationId of attestationIds) {
    const endorsements = await getEndorsements(server, contract, attestationId, sourceAddress);
    let attestationScore = 0;
    const endorsementDetails = [];

    for (const endorsement of endorsements) {
      const endorserAddress =
        typeof endorsement.endorser === "string"
          ? endorsement.endorser
          : endorsement.endorser?.toString?.() ?? String(endorsement.endorser);

      const tier = await getIssuerTier(server, contract, endorserAddress, sourceAddress);
      const weight = TIER_WEIGHTS[tier] ?? 1;
      attestationScore += weight;
      endorsementDetails.push({ endorser: endorserAddress, tier, weight });
    }

    totalScore += attestationScore;
    breakdown.push({
      attestationId,
      score: attestationScore,
      endorsements: endorsementDetails,
    });
  }

  return { totalScore, breakdown };
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  required(cfg.contractId, "TRUSTLINK_CONTRACT_ID");
  required(cfg.freelancerAddress, "FREELANCER_ADDRESS");
  required(cfg.clientASecret, "CLIENT_A_SECRET");
  required(cfg.clientBSecret, "CLIENT_B_SECRET");
  required(cfg.clientCSecret, "CLIENT_C_SECRET");

  const server = new SorobanRpc.Server(cfg.rpcUrl);
  const contract = new Contract(cfg.contractId);

  const clientA = Keypair.fromSecret(cfg.clientASecret); // Basic tier
  const clientB = Keypair.fromSecret(cfg.clientBSecret); // Verified tier
  const clientC = Keypair.fromSecret(cfg.clientCSecret); // Premium tier

  console.log("\n=== FREELANCE MARKETPLACE REPUTATION EXAMPLE ===");
  console.log(`Freelancer:          ${cfg.freelancerAddress}`);
  console.log(`Client A (Basic):    ${clientA.publicKey()}`);
  console.log(`Client B (Verified): ${clientB.publicKey()}`);
  console.log(`Client C (Premium):  ${clientC.publicKey()}`);

  // ── Step 1: Each client issues a JOB_COMPLETED attestation ───────────────
  //
  // In a real marketplace, this happens automatically when the client marks a
  // project as complete and the platform submits the transaction on their behalf.

  console.log("\n--- Step 1: Issue job-completion attestations ---");

  console.log("Client A issuing JOB_COMPLETED for job-freelance-001...");
  let idA;
  try {
    idA = await issueJobCompletion(server, contract, clientA, "job-freelance-001");
    console.log(`  ✓ Attestation ID: ${idA}`);
  } catch (err) {
    if (err.message.includes("DuplicateAttestation")) {
      console.log("  ⚠ Attestation already exists — continuing with existing record.");
    } else {
      throw err;
    }
  }

  console.log("Client B issuing JOB_COMPLETED for job-freelance-002...");
  let idB;
  try {
    idB = await issueJobCompletion(server, contract, clientB, "job-freelance-002");
    console.log(`  ✓ Attestation ID: ${idB}`);
  } catch (err) {
    if (err.message.includes("DuplicateAttestation")) {
      console.log("  ⚠ Attestation already exists — continuing with existing record.");
    } else {
      throw err;
    }
  }

  console.log("Client C issuing JOB_COMPLETED for job-freelance-003...");
  let idC;
  try {
    idC = await issueJobCompletion(server, contract, clientC, "job-freelance-003");
    console.log(`  ✓ Attestation ID: ${idC}`);
  } catch (err) {
    if (err.message.includes("DuplicateAttestation")) {
      console.log("  ⚠ Attestation already exists — continuing with existing record.");
    } else {
      throw err;
    }
  }

  const attestationIds = [idA, idB, idC].filter(Boolean);
  if (attestationIds.length === 0) {
    console.error("No attestation IDs available. Re-run after resolving duplicate issues.");
    process.exit(1);
  }

  // ── Step 2: Clients cross-endorse attestations ────────────────────────────
  //
  // Endorsements add social proof: a Verified-tier client vouching for work
  // attested by a Basic-tier client signals that the work quality is confirmed
  // by a more trusted party.

  console.log("\n--- Step 2: Cross-endorse attestations ---");

  if (idA) {
    console.log("Client B (Verified) endorses Client A's attestation...");
    try {
      await endorseAttestation(server, contract, clientB, idA);
      console.log("  ✓ Endorsed");
    } catch (err) {
      if (err.message.includes("AlreadyEndorsed")) {
        console.log("  ⚠ Already endorsed — skipping.");
      } else {
        throw err;
      }
    }

    console.log("Client C (Premium) endorses Client A's attestation...");
    try {
      await endorseAttestation(server, contract, clientC, idA);
      console.log("  ✓ Endorsed");
    } catch (err) {
      if (err.message.includes("AlreadyEndorsed")) {
        console.log("  ⚠ Already endorsed — skipping.");
      } else {
        throw err;
      }
    }
  }

  if (idB) {
    console.log("Client C (Premium) endorses Client B's attestation...");
    try {
      await endorseAttestation(server, contract, clientC, idB);
      console.log("  ✓ Endorsed");
    } catch (err) {
      if (err.message.includes("AlreadyEndorsed")) {
        console.log("  ⚠ Already endorsed — skipping.");
      } else {
        throw err;
      }
    }
  }

  // ── Step 3: Compute tier-weighted reputation score ────────────────────────
  //
  // The platform queries endorsements for each attestation and looks up each
  // endorser's IssuerTier to assign a weight. The total score reflects both
  // the number of endorsements and the trust level of the endorsers.

  console.log("\n--- Step 3: Compute tier-weighted reputation score ---");
  console.log(`Tier weights: Basic=${TIER_WEIGHTS.Basic}, Verified=${TIER_WEIGHTS.Verified}, Premium=${TIER_WEIGHTS.Premium}`);

  const { totalScore, breakdown } = await computeReputationScore(
    server,
    contract,
    attestationIds,
    clientA.publicKey()
  );

  console.log("\nPer-attestation breakdown:");
  for (const entry of breakdown) {
    console.log(`\n  Attestation: ${entry.attestationId}`);
    console.log(`  Sub-score:   ${entry.score} point(s)`);
    if (entry.endorsements.length === 0) {
      console.log("  Endorsements: (none)");
    } else {
      for (const e of entry.endorsements) {
        const shortAddr = e.endorser.slice(0, 8) + "...";
        console.log(`    ${shortAddr} (${e.tier}) → +${e.weight}`);
      }
    }
  }

  console.log(`\n✓ Total reputation score: ${totalScore}`);
  console.log("\nInterpretation:");
  console.log("  Score = sum of endorser tier weights across all JOB_COMPLETED attestations.");
  console.log("  A Verified endorsement (2pts) outweighs two Basic endorsements (1pt each).");
  console.log("  Platforms can set minimum score thresholds for project eligibility.");

  console.log("\n=== FLOW COMPLETE ===");
}

main().catch((err) => {
  console.error("Unhandled error:", err.message);
  process.exit(1);
});
