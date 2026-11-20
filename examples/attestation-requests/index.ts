/**
 * Example: Attestation request flow
 *
 * This script demonstrates the full lifecycle of attestation requests:
 *
 *   1. A subject requests an attestation from an issuer.
 *   2. The issuer lists its pending requests.
 *   3. The issuer fulfils one request and rejects another.
 *   4. The subject verifies the fulfilled claim.
 *
 * Run with:
 *   npx ts-node examples/attestation-requests/index.ts
 *
 * See README.md in this directory for details on expiry and cleanup.
 */

import { TrustLinkClient } from "@trustlink/sdk";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

// In a real deployment these would come from your environment / config.
const RPC_URL = process.env.RPC_URL ?? "http://localhost:8545";
const ISSUER_KEY = process.env.ISSUER_KEY ?? "0xissuer-private-key";
const SUBJECT_KEY = process.env.SUBJECT_KEY ?? "0xsubject-private-key";

// The claim type the subject is asking the issuer to attest.
const CLAIM_TYPE = "KYC_VERIFIED";

async function main(): Promise<void> {
  // -------------------------------------------------------------------------
  // Set up clients
  // -------------------------------------------------------------------------
  const issuer = new TrustLinkClient({ rpcUrl: RPC_URL, privateKey: ISSUER_KEY });
  const subject = new TrustLinkClient({ rpcUrl: RPC_URL, privateKey: SUBJECT_KEY });

  const issuerAddress = await issuer.getAddress();
  const subjectAddress = await subject.getAddress();

  console.log(`Issuer:  ${issuerAddress}`);
  console.log(`Subject: ${subjectAddress}`);

  // -------------------------------------------------------------------------
  // Step 1: Subject requests an attestation
  // -------------------------------------------------------------------------
  console.log("\n[1] Subject requests an attestation...");

  const requestId = await subject.requestAttestation({
    issuer: issuerAddress,
    claimType: CLAIM_TYPE,
    // Optional metadata the issuer can inspect when deciding.
    data: { note: "Please verify my KYC documents." },
  });

  // requestId: e.g. "0x3f2a...c1" (bytes32 identifier for this request)
  // status:    "pending"
  console.log(`    requestId: ${requestId}`);
  console.log(`    status:    pending`);

  // -------------------------------------------------------------------------
  // Step 2: Issuer lists pending requests
  // -------------------------------------------------------------------------
  console.log("\n[2] Issuer lists pending requests...");

  const pending = await issuer.getPendingRequests();

  // pending: array of { requestId, subject, claimType, createdAt, expiresAt, status }
  // Example:
  //   [
  //     {
  //       requestId: "0x3f2a...c1",
  //       subject:   "0xsubject...",
  //       claimType: "KYC_VERIFIED",
  //       createdAt: 1712345678,
  //       expiresAt: 1712950478,
  //       status:    "pending",
  //     },
  //   ]
  console.log(`    Found ${pending.length} pending request(s):`);
  for (const req of pending) {
    console.log(
      `      - ${req.requestId} | subject=${req.subject} | type=${req.claimType} | status=${req.status}`,
    );
  }

  // -------------------------------------------------------------------------
  // Step 3a: Issuer fulfils the first request
  // -------------------------------------------------------------------------
  console.log("\n[3a] Issuer fulfils the first request...");

  const toFulfil = pending[0];
  if (!toFulfil) {
    throw new Error("No pending requests to fulfil.");
  }

  const fulfilTx = await issuer.fulfillRequest({
    requestId: toFulfil.requestId,
    // Optional: attach an expiry to the issued claim (unix seconds).
    // If omitted, the SDK applies the default claim TTL.
    expiresAt: Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 365, // 1 year
  });

  // fulfilTx: transaction hash, e.g. "0xabc123..."
  // request status after fulfil: "fulfilled"
  // a new claim is now minted for the subject
  console.log(`    tx:      ${fulfilTx}`);
  console.log(`    status:  fulfilled`);

  // -------------------------------------------------------------------------
  // Step 3b: Issuer rejects a second request
  // -------------------------------------------------------------------------
  console.log("\n[3b] Issuer rejects a second request...");

  // For the demo we create a second request from the subject so we have one
  // to reject. In practice this would be another subject's request.
  const secondRequestId = await subject.requestAttestation({
    issuer: issuerAddress,
    claimType: "ACCREDITED_INVESTOR",
    data: { note: "Second request, will be rejected." },
  });

  // secondRequestId: e.g. "0x9b7d...e4"
  // status:          "pending"
  console.log(`    requestId: ${secondRequestId}`);
  console.log(`    status:    pending`);

  const rejectTx = await issuer.rejectRequest({
    requestId: secondRequestId,
    reason: "Insufficient documentation provided.",
  });

  // rejectTx: transaction hash
  // request status after reject: "rejected"
  console.log(`    tx:      ${rejectTx}`);
  console.log(`    status:  rejected`);

  // -------------------------------------------------------------------------
  // Step 4: Subject verifies the fulfilled claim
  // -------------------------------------------------------------------------
  console.log("\n[4] Subject verifies the fulfilled claim...");

  const hasClaim = await subject.hasValidClaim({
    claimType: CLAIM_TYPE,
    issuer: issuerAddress,
  });

  // hasClaim: true (the claim was minted in step 3a and has not expired)
  console.log(`    hasValidClaim(${CLAIM_TYPE}): ${hasClaim}`);

  if (hasClaim) {
    const claim = await subject.getAttestation({
      claimType: CLAIM_TYPE,
      issuer: issuerAddress,
    });

    // claim: { claimType, issuer, subject, issuedAt, expiresAt, status, ... }
    // Example:
    //   {
    //     claimType: "KYC_VERIFIED",
    //     issuer:    "0xissuer...",
    //     subject:   "0xsubject...",
    //     issuedAt:  1712345700,
    //     expiresAt: 1743881700,
    //     status:    "valid",
    //   }
    console.log("    Claim details:");
    console.log(`      claimType: ${claim.claimType}`);
    console.log(`      issuer:    ${claim.issuer}`);
    console.log(`      subject:   ${claim.subject}`);
    console.log(`      issuedAt:  ${claim.issuedAt}`);
    console.log(`      expiresAt: ${claim.expiresAt}`);
    console.log(`      status:    ${claim.status}`);
  }

  // The rejected request never produced a claim, so this should be false.
  const hasRejectedClaim = await subject.hasValidClaim({
    claimType: "ACCREDITED_INVESTOR",
    issuer: issuerAddress,
  });
  console.log(`    hasValidClaim(ACCREDITED_INVESTOR): ${hasRejectedClaim}`);

  console.log("\nDone.");
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});