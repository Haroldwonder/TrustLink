/**
 * End-to-end tests for the TrustLink TypeScript SDK.
 *
 * Prerequisites:
 *   1. A local Stellar Quickstart node running (docker compose up -d)
 *   2. The contract deployed and initialized via scripts/setup_local.sh
 *   3. CONTRACT_ID env var set (setup_local.sh writes it to .local.contract-id)
 *
 * Run:
 *   npm run test:e2e
 *
 * Or with explicit env vars:
 *   CONTRACT_ID=C... ISSUER_SECRET=S... npm run test:e2e
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
  Operation,
  Transaction,
} from "@stellar/stellar-sdk";
import { readFileSync, existsSync } from "fs";
import { resolve } from "path";

// ── Config ──────────────────────────────────────────────────────────────────

const RPC_URL = process.env.RPC_URL ?? "http://localhost:8000/soroban/rpc";
const NETWORK_PASSPHRASE =
  process.env.NETWORK_PASSPHRASE ?? Networks.STANDALONE;

function resolveContractId(): string {
  if (process.env.CONTRACT_ID) return process.env.CONTRACT_ID;
  const idFile = resolve(__dirname, "../../../../.local.contract-id");
  if (existsSync(idFile)) return readFileSync(idFile, "utf8").trim();
  throw new Error(
    "CONTRACT_ID env var not set and .local.contract-id not found. " +
      "Run scripts/setup_local.sh first."
  );
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const server = new SorobanRpc.Server(RPC_URL, { allowHttp: true });

async function fundAccount(keypair: Keypair): Promise<void> {
  const friendbotUrl = `http://localhost:8000/friendbot?addr=${keypair.publicKey()}`;
  const res = await fetch(friendbotUrl);
  if (!res.ok) throw new Error(`Friendbot failed: ${res.status}`);
}

/**
 * Build, simulate, sign, and submit a contract invocation.
 * Returns the transaction hash on success.
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
    throw new Error(`Simulation failed for ${method}: ${simResult.error}`);
  }

  const prepared = SorobanRpc.assembleTransaction(
    tx,
    simResult as SorobanRpc.Api.SimulateTransactionSuccessResponse
  ).build();

  prepared.sign(signer);

  const sendResult = await server.sendTransaction(prepared);
  if (sendResult.status === "ERROR") {
    throw new Error(`sendTransaction failed for ${method}: ${JSON.stringify(sendResult.errorResult)}`);
  }

  // Poll until the transaction is confirmed.
  const hash = sendResult.hash;
  for (let i = 0; i < 30; i++) {
    await new Promise((r) => setTimeout(r, 1000));
    const status = await server.getTransaction(hash);
    if (status.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) return hash;
    if (status.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
      throw new Error(`Transaction ${hash} failed for ${method}`);
    }
  }
  throw new Error(`Transaction ${hash} timed out for ${method}`);
}

async function simulate<T>(
  contractId: string,
  method: string,
  args: ReturnType<typeof nativeToScVal>[]
): Promise<T> {
  const contract = new Contract(contractId);
  const dummySource = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
  const account = new SorobanRpc.Server(RPC_URL, { allowHttp: true })
    .getAccount(dummySource)
    .catch(() => ({ accountId: () => dummySource, sequenceNumber: () => "0", incrementSequenceNumber: () => {} } as any));

  const acc = await account;
  const tx = new TransactionBuilder(acc as any, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  const result = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(result)) {
    throw new Error(`Simulation failed for ${method}: ${result.error}`);
  }
  const success = result as SorobanRpc.Api.SimulateTransactionSuccessResponse;
  if (!success.result) throw new Error(`No result from ${method}`);

  const { scValToNative } = await import("@stellar/stellar-sdk");
  return scValToNative(success.result.retval) as T;
}

function addr(address: string) {
  return Address.fromString(address).toScVal();
}

function str(value: string) {
  return nativeToScVal(value, { type: "string" });
}

// ── Test Suite ───────────────────────────────────────────────────────────────

describe("TrustLink SDK — end-to-end against local node", () => {
  let contractId: string;
  let adminKeypair: Keypair;
  let issuerKeypair: Keypair;
  let subjectKeypair: Keypair;
  let attestationId: string;

  beforeAll(async () => {
    contractId = resolveContractId();

    // Generate fresh keypairs for this test run.
    adminKeypair = process.env.ADMIN_SECRET
      ? Keypair.fromSecret(process.env.ADMIN_SECRET)
      : Keypair.random();
    issuerKeypair = process.env.ISSUER_SECRET
      ? Keypair.fromSecret(process.env.ISSUER_SECRET)
      : Keypair.random();
    subjectKeypair = Keypair.random();

    // Fund all accounts via local Friendbot.
    await Promise.all([
      fundAccount(adminKeypair),
      fundAccount(issuerKeypair),
      fundAccount(subjectKeypair),
    ]);
  }, 60_000);

  // ── initialize ─────────────────────────────────────────────────────────────

  it("contract is already initialized — get_admin returns a valid address", async () => {
    const admin: string = await simulate(contractId, "get_admin", []);
    expect(typeof admin).toBe("string");
    expect(admin.length).toBeGreaterThan(0);
  }, 30_000);

  // ── register_issuer ────────────────────────────────────────────────────────

  it("admin can register a new issuer", async () => {
    await invoke(
      contractId,
      "register_issuer",
      [addr(adminKeypair.publicKey()), addr(issuerKeypair.publicKey())],
      adminKeypair
    );

    const isIssuer: boolean = await simulate(contractId, "is_issuer", [
      addr(issuerKeypair.publicKey()),
    ]);
    expect(isIssuer).toBe(true);
  }, 60_000);

  // ── create_attestation ─────────────────────────────────────────────────────

  it("registered issuer can create an attestation", async () => {
    await invoke(
      contractId,
      "create_attestation",
      [
        addr(issuerKeypair.publicKey()),
        addr(subjectKeypair.publicKey()),
        str("KYC_PASSED"),
        nativeToScVal(null),  // no expiration
        nativeToScVal(null),  // no metadata
      ],
      issuerKeypair
    );

    // Retrieve the attestation ID from the subject's list.
    const attestations: any[] = await simulate(
      contractId,
      "get_subject_attestations",
      [addr(subjectKeypair.publicKey()), nativeToScVal(0, { type: "u32" }), nativeToScVal(10, { type: "u32" })]
    );

    expect(attestations.length).toBeGreaterThan(0);
    attestationId = attestations[0].id;
    expect(typeof attestationId).toBe("string");
    expect(attestationId.length).toBeGreaterThan(0);
  }, 60_000);

  // ── has_valid_claim ────────────────────────────────────────────────────────

  it("has_valid_claim returns true for a freshly created attestation", async () => {
    const result: boolean = await simulate(contractId, "has_valid_claim", [
      addr(subjectKeypair.publicKey()),
      str("KYC_PASSED"),
    ]);
    expect(result).toBe(true);
  }, 30_000);

  it("has_valid_claim returns false for a claim type the subject does not hold", async () => {
    const result: boolean = await simulate(contractId, "has_valid_claim", [
      addr(subjectKeypair.publicKey()),
      str("ACCREDITED_INVESTOR"),
    ]);
    expect(result).toBe(false);
  }, 30_000);

  // ── revoke ─────────────────────────────────────────────────────────────────

  it("issuer can revoke the attestation", async () => {
    await invoke(
      contractId,
      "revoke_attestation",
      [addr(issuerKeypair.publicKey()), str(attestationId)],
      issuerKeypair
    );

    const attestation: any = await simulate(contractId, "get_attestation", [
      str(attestationId),
    ]);
    expect(attestation.revoked).toBe(true);
  }, 60_000);

  it("has_valid_claim returns false after revocation", async () => {
    const result: boolean = await simulate(contractId, "has_valid_claim", [
      addr(subjectKeypair.publicKey()),
      str("KYC_PASSED"),
    ]);
    expect(result).toBe(false);
  }, 30_000);
});
