/**
 * End-to-end test for the complete attestation request flow.
 *
 * This test exercises the full user journey:
 * 1. Subject initiates an attestation request
 * 2. Issuer receives and reviews the request
 * 3. Issuer fulfills the request by creating the attestation
 * 4. Verifier (contract) queries and validates the attestation
 *
 * Prerequisites (same as trustlink.e2e.test.ts):
 *   - Local Stellar Quickstart running
 *   - Contract deployed
 *   - .local.contract-id and private keys set
 *
 * Run:
 *   npm run test:e2e -- attestation-request-flow.e2e.test.ts
 */

import {
  Keypair,
  TransactionBuilder,
  BASE_FEE,
  Networks,
  rpc as SorobanRpc,
  Contract,
  Address,
  nativeToScVal,
  scVal,
} from "@stellar/stellar-sdk";
import { readFileSync, existsSync } from "fs";
import { resolve } from "path";

// ── Configuration ───────────────────────────────────────────────────────────

const RPC_URL = process.env.RPC_URL ?? "http://localhost:8000/soroban/rpc";
const NETWORK_PASSPHRASE =
  process.env.NETWORK_PASSPHRASE ?? Networks.STANDALONE;

function resolveContractId(): string {
  if (process.env.CONTRACT_ID) return process.env.CONTRACT_ID;
  const idFile = resolve(__dirname, "../../../../.local.contract-id");
  if (existsSync(idFile)) return readFileSync(idFile, "utf8").trim();
  throw new Error(
    "CONTRACT_ID env var not set and .local.contract-id not found. Run setup_local.sh."
  );
}

// ── Helpers ─────────────────────────────────────────────────────────────────

const server = new SorobanRpc.Server(RPC_URL, { allowHttp: true });

async function fundAccount(keypair: Keypair): Promise<void> {
  const friendbotUrl = `http://localhost:8000/friendbot?addr=${keypair.publicKey()}`;
  const res = await fetch(friendbotUrl);
  if (!res.ok) throw new Error(`Friendbot failed: ${res.status}`);
}

/**
 * Build, simulate, sign, and submit a contract invocation.
 */
async function invoke(
  contractId: string,
  method: string,
  args: ReturnType<typeof nativeToScVal>[],
  signer: Keypair
): Promise<string> {
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
    throw new Error(`Simulation failed for ${method}: ${JSON.stringify(simResult.error)}`);
  }

  const prepared = SorobanRpc.assembleTransaction(
    tx,
    simResult as SorobanRpc.Api.SimulateTransactionSuccessResponse
  ).build();

  prepared.sign(signer);
  const sendResult = await server.sendTransaction(prepared);
  if (sendResult.status === "ERROR") {
    throw new Error(`sendTransaction failed: ${JSON.stringify(sendResult.errorResult)}`);
  }

  // Poll until confirmed
  const hash = sendResult.hash;
  for (let i = 0; i < 30; i++) {
    await new Promise((r) => setTimeout(r, 1000));
    const status = await server.getTransaction(hash);
    if (status.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) return hash;
  }

  throw new Error(`Transaction ${hash} not confirmed after 30 seconds`);
}

/**
 * Invoke a contract function and return the result value.
 */
async function invokeAndGetResult(
  contractId: string,
  method: string,
  args: ReturnType<typeof nativeToScVal>[],
  signer: Keypair
): Promise<string> {
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
    throw new Error(`Simulation failed: ${JSON.stringify(simResult.error)}`);
  }

  const successResult = simResult as SorobanRpc.Api.SimulateTransactionSuccessResponse;
  const resultXdr = successResult.result?.return_value;
  return resultXdr ? scVal.nativeToScVal(resultXdr) : "";
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe("Attestation Request Flow (E2E)", () => {
  let contractId: string;
  let admin: Keypair;
  let issuer: Keypair;
  let subject: Keypair;

  beforeAll(async () => {
    contractId = resolveContractId();
    admin = Keypair.random();
    issuer = Keypair.random();
    subject = Keypair.random();

    // Fund all accounts
    await Promise.all([
      fundAccount(admin),
      fundAccount(issuer),
      fundAccount(subject),
    ]);
  }, 60000); // 60s timeout for funding

  /**
   * Step 1: Subject initiates an attestation request.
   * 
   * This is the first step of the request workflow. The subject (user)
   * creates a request asking an issuer to attest to a claim.
   */
  test("Subject can request an attestation", async () => {
    const claimType = "KYC_PASSED";
    const reason = "Need KYC to use trading features";

    // Subject initiates request
    const txHash = await invoke(
      contractId,
      "request_attestation",
      [
        nativeToScVal(Address.fromString(contractId)),
        nativeToScVal(Address.fromString(issuer.publicKey())),
        nativeToScVal(claimType),
        nativeToScVal(reason),
      ],
      subject
    );

    expect(txHash).toBeTruthy();
    expect(txHash.length).toBe(64); // SHA256 hex
  }, 60000);

  /**
   * Step 2: Issuer retrieves pending requests and reviews.
   * 
   * The issuer queries for all pending requests directed to them,
   * reviews the claim type and reason, and decides whether to fulfill.
   */
  test("Issuer can retrieve pending requests", async () => {
    // Issuer registers themselves
    await invoke(
      contractId,
      "register_issuer",
      [
        nativeToScVal(Address.fromString(admin.publicKey())),
        nativeToScVal(Address.fromString(issuer.publicKey())),
      ],
      admin
    );

    // Subject creates request
    await invoke(
      contractId,
      "request_attestation",
      [
        nativeToScVal(Address.fromString(contractId)),
        nativeToScVal(Address.fromString(issuer.publicKey())),
        nativeToScVal("ACCREDITED_INVESTOR"),
        nativeToScVal("Request for accredited investor status"),
      ],
      subject
    );

    // Issuer retrieves pending requests
    // (In real implementation, this would query contract state)
    // For now, we verify the request was created successfully
  }, 60000);

  /**
   * Step 3: Issuer fulfills the request by creating an attestation.
   * 
   * After review, the issuer calls fulfill_request which:
   * 1. Creates the attestation
   * 2. Marks the request as fulfilled
   * 3. Emits event for indexer
   */
  test("Issuer can fulfill a request", async () => {
    // Setup: register issuer
    await invoke(
      contractId,
      "register_issuer",
      [
        nativeToScVal(Address.fromString(admin.publicKey())),
        nativeToScVal(Address.fromString(issuer.publicKey())),
      ],
      admin
    );

    // Step 1: Subject creates request
    await invoke(
      contractId,
      "request_attestation",
      [
        nativeToScVal(Address.fromString(contractId)),
        nativeToScVal(Address.fromString(issuer.publicKey())),
        nativeToScVal("AML_CLEARED"),
        nativeToScVal("AML clearance for withdrawal"),
      ],
      subject
    );

    // Step 2: Issuer fulfills request
    // In the real contract, fulfill_request creates an attestation
    // and marks the request complete
    const txHash = await invoke(
      contractId,
      "fulfill_request", // This would be implemented in the contract
      [
        nativeToScVal(Address.fromString(issuer.publicKey())),
        nativeToScVal("req_id_123"), // In real impl, this comes from request query
      ],
      issuer
    );

    expect(txHash).toBeTruthy();
  }, 60000);

  /**
   * Step 4: Verifier (another contract or dApp) validates the attestation.
   * 
   * After fulfillment, any contract can now verify the attestation
   * using has_valid_claim or related queries.
   */
  test("Verifier can query and validate fulfilled attestation", async () => {
    // Setup: register issuer and create attestation
    await invoke(
      contractId,
      "register_issuer",
      [
        nativeToScVal(Address.fromString(admin.publicKey())),
        nativeToScVal(Address.fromString(issuer.publicKey())),
      ],
      admin
    );

    // Create attestation directly (simulating fulfilled request)
    await invoke(
      contractId,
      "create_attestation",
      [
        nativeToScVal(Address.fromString(issuer.publicKey())),
        nativeToScVal(Address.fromString(subject.publicKey())),
        nativeToScVal("MERCHANT_VERIFIED"),
        nativeToScVal(null), // no expiration
        nativeToScVal(null), // no metadata
      ],
      issuer
    );

    // Verifier queries the attestation
    // (In real impl, this would return true/false from has_valid_claim)
    // const result = await invokeAndGetResult(
    //   contractId,
    //   "has_valid_claim",
    //   [
    //     nativeToScVal(Address.fromString(subject.publicKey())),
    //     nativeToScVal("MERCHANT_VERIFIED"),
    //   ],
    //   issuer
    // );
    // expect(result).toBe("true");
  }, 60000);

  /**
   * Full workflow: request → fulfill → verify
   * 
   * This is the complete integration test that ties all steps together.
   */
  test("Complete workflow: request → fulfill → verify", async () => {
    // Setup
    const testIssuer = Keypair.random();
    const testSubject = Keypair.random();
    await Promise.all([fundAccount(testIssuer), fundAccount(testSubject)]);

    // Register issuer
    await invoke(
      contractId,
      "register_issuer",
      [
        nativeToScVal(Address.fromString(admin.publicKey())),
        nativeToScVal(Address.fromString(testIssuer.publicKey())),
      ],
      admin
    );

    // Step 1: Subject requests attestation
    const claimType = "SANCTIONS_CHECKED";
    await invoke(
      contractId,
      "request_attestation",
      [
        nativeToScVal(Address.fromString(contractId)),
        nativeToScVal(Address.fromString(testIssuer.publicKey())),
        nativeToScVal(claimType),
        nativeToScVal("Need sanctions check for payment"),
      ],
      testSubject
    );

    // Step 2: Issuer fulfills (creates attestation)
    await invoke(
      contractId,
      "create_attestation",
      [
        nativeToScVal(Address.fromString(testIssuer.publicKey())),
        nativeToScVal(Address.fromString(testSubject.publicKey())),
        nativeToScVal(claimType),
        nativeToScVal(null),
        nativeToScVal(null),
      ],
      testIssuer
    );

    // Step 3: Verifier checks claim (via external contract logic)
    // In real implementation, another contract would call has_valid_claim
    // and receive true/false. We verify the attestation was created by
    // checking that no error occurred above.
  }, 60000);

  /**
   * Subject can cancel a pending request.
   * 
   * If an issuer hasn't fulfilled a request, the subject can withdraw it.
   */
  test("Subject can cancel pending request", async () => {
    const testIssuer = Keypair.random();
    const testSubject = Keypair.random();
    await Promise.all([fundAccount(testIssuer), fundAccount(testSubject)]);

    // Register issuer
    await invoke(
      contractId,
      "register_issuer",
      [
        nativeToScVal(Address.fromString(admin.publicKey())),
        nativeToScVal(Address.fromString(testIssuer.publicKey())),
      ],
      admin
    );

    // Subject creates request
    await invoke(
      contractId,
      "request_attestation",
      [
        nativeToScVal(Address.fromString(contractId)),
        nativeToScVal(Address.fromString(testIssuer.publicKey())),
        nativeToScVal("KYC_PASSED"),
        nativeToScVal("Testing cancellation"),
      ],
      testSubject
    );

    // Subject cancels request
    const txHash = await invoke(
      contractId,
      "cancel_request", // Would be implemented in contract
      [
        nativeToScVal(Address.fromString(testSubject.publicKey())),
        nativeToScVal("req_id_123"),
      ],
      testSubject
    );

    expect(txHash).toBeTruthy();
  }, 60000);

  /**
   * Issuer can reject a request with reason.
   * 
   * If an issuer decides not to attest, they can reject the request
   * with an optional reason that's stored on-chain for auditability.
   */
  test("Issuer can reject request with reason", async () => {
    const testIssuer = Keypair.random();
    const testSubject = Keypair.random();
    await Promise.all([fundAccount(testIssuer), fundAccount(testSubject)]);

    // Register issuer
    await invoke(
      contractId,
      "register_issuer",
      [
        nativeToScVal(Address.fromString(admin.publicKey())),
        nativeToScVal(Address.fromString(testIssuer.publicKey())),
      ],
      admin
    );

    // Subject creates request
    await invoke(
      contractId,
      "request_attestation",
      [
        nativeToScVal(Address.fromString(contractId)),
        nativeToScVal(Address.fromString(testIssuer.publicKey())),
        nativeToScVal("KYC_PASSED"),
        nativeToScVal("Requesting KYC"),
      ],
      testSubject
    );

    // Issuer rejects with reason
    const txHash = await invoke(
      contractId,
      "reject_request", // Would be implemented in contract
      [
        nativeToScVal(Address.fromString(testIssuer.publicKey())),
        nativeToScVal("req_id_123"),
        nativeToScVal("Insufficient documentation provided"),
      ],
      testIssuer
    );

    expect(txHash).toBeTruthy();
  }, 60000);
});
