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
});import {
  Address,
  BASE_FEE,
  Contract,
  Keypair,
  nativeToScVal,
  Networks,
  rpc as SorobanRpc,
  TransactionBuilder,
} from "@stellar/stellar-sdk";
import { existsSync, readFileSync } from "fs";
import { resolve } from "path";
import type { ConformanceContext } from "./types";

const RPC_URL = process.env.RPC_URL ?? "http://localhost:8000/soroban/rpc";
const NETWORK_PASSPHRASE = process.env.NETWORK_PASSPHRASE ?? Networks.STANDALONE;

export function resolveContractId(): string {
  if (process.env.CONTRACT_ID) {
    return process.env.CONTRACT_ID;
  }
  const idFile = resolve(__dirname, "../../.local.contract-id");
  if (existsSync(idFile)) {
    return readFileSync(idFile, "utf8").trim();
  }
  throw new Error(
    "CONTRACT_ID not set and .local.contract-id not found. Run scripts/setup_local.sh first.",
  );
}

export async function isRpcAvailable(): Promise<boolean> {
  try {
    const server = new SorobanRpc.Server(RPC_URL, { allowHttp: true });
    await server.getLatestLedger();
    return true;
  } catch {
    return false;
  }
}

async function fundAccount(keypair: Keypair): Promise<void> {
  const response = await fetch(`http://localhost:8000/friendbot?addr=${keypair.publicKey()}`);
  if (!response.ok) {
    throw new Error(`Friendbot failed: ${response.status}`);
  }
}

async function invoke(
  contractId: string,
  method: string,
  args: ReturnType<typeof nativeToScVal>[],
  signer: Keypair,
): Promise<void> {
  const server = new SorobanRpc.Server(RPC_URL, { allowHttp: true });
  const contract = new Contract(contractId);
  const account = await server.getAccount(signer.publicKey());

  const tx = new TransactionBuilder(account, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  const simResult = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(simResult)) {
    throw new Error(`Simulation failed for ${method}: ${simResult.error}`);
  }

  const prepared = SorobanRpc.assembleTransaction(
    tx,
    simResult as SorobanRpc.Api.SimulateTransactionSuccessResponse,
  ).build();
  prepared.sign(signer);

  const sendResult = await server.sendTransaction(prepared);
  if (sendResult.status === "ERROR") {
    throw new Error(`sendTransaction failed for ${method}`);
  }

  for (let attempt = 0; attempt < 30; attempt += 1) {
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 1_000));
    const status = await server.getTransaction(sendResult.hash);
    if (status.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
      return;
    }
    if (status.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
      throw new Error(`Transaction failed for ${method}`);
    }
  }
  throw new Error(`Transaction timed out for ${method}`);
}

export async function prepareConformanceContext(): Promise<ConformanceContext> {
  const contractId = resolveContractId();
  const adminKeypair = Keypair.random();
  const issuerKeypair = Keypair.random();
  const subjectKeypair = Keypair.random();

  await Promise.all([
    fundAccount(adminKeypair),
    fundAccount(issuerKeypair),
    fundAccount(subjectKeypair),
  ]);

  await invoke(
    contractId,
    "register_issuer",
    [Address.fromString(adminKeypair.publicKey()).toScVal(), Address.fromString(issuerKeypair.publicKey()).toScVal()],
    adminKeypair,
  );

  await invoke(
    contractId,
    "create_attestation",
    [
      Address.fromString(issuerKeypair.publicKey()).toScVal(),
      Address.fromString(subjectKeypair.publicKey()).toScVal(),
      nativeToScVal("KYC_PASSED", { type: "string" }),
      nativeToScVal(null),
      nativeToScVal(null),
    ],
    issuerKeypair,
  );

  return {
    contractId,
    rpcUrl: RPC_URL,
    networkPassphrase: NETWORK_PASSPHRASE,
    subject: subjectKeypair.publicKey(),
    issuer: issuerKeypair.publicKey(),
  };
}
import type { ConformanceObservation } from "./types";

const ERROR_NAME_TO_CODE: Record<string, number> = {
  AlreadyInitialized: 1,
  NotInitialized: 2,
  Unauthorized: 3,
  NotFound: 4,
  DuplicateAttestation: 5,
  AlreadyRevoked: 6,
  Expired: 7,
};

export function normalizeAttestationIds(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.map((entry) => {
    if (typeof entry === "string") {
      return entry;
    }
    if (entry && typeof entry === "object" && "id" in entry) {
      return String((entry as { id: unknown }).id);
    }
    return String(entry);
  });
}

export function parseErrorObservation(step: string, error: unknown): ConformanceObservation {
  if (error && typeof error === "object") {
    if ("code" in error && typeof (error as { code: unknown }).code === "number") {
      const code = (error as { code: number }).code;
      const rawName =
        "name" in error && typeof (error as { name: unknown }).name === "string"
          ? (error as { name: string }).name
          : `ContractError(${code})`;
      const name = rawName.replace(/Error$/, "");
      return { step, kind: "error", code, name };
    }
  }

  const message = error instanceof Error ? error.message : String(error);
  const contractMatch = message.match(/Error\(Contract,\s*#(\d+)\)/);
  if (contractMatch) {
    const code = Number.parseInt(contractMatch[1], 10);
    return { step, kind: "error", code, name: `ContractError(${code})` };
  }

  const namedCode = ERROR_NAME_TO_CODE[message];
  if (namedCode !== undefined) {
    return { step, kind: "error", code: namedCode, name: message };
  }

  throw new Error(`Unable to normalize error for step ${step}: ${message}`);
}

export function assertObservationsMatch(
  baseline: ConformanceObservation[],
  actual: ConformanceObservation[],
  clientName: string,
): void {
  const baselineJson = JSON.stringify(baseline, null, 2);
  const actualJson = JSON.stringify(actual, null, 2);
  if (baselineJson !== actualJson) {
    throw new Error(
      `Conformance drift detected for ${clientName}.\nExpected:\n${baselineJson}\nActual:\n${actualJson}`,
    );
  }
}
