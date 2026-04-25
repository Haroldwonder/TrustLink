import {
  Account,
  Contract,
  rpc as SorobanRpc,
  TransactionBuilder,
  Networks,
  BASE_FEE,
  nativeToScVal,
  scValToNative,
  Address,
  xdr,
} from "@stellar/stellar-sdk";
import { sign } from "./wallet";

const CONTRACT_ID = import.meta.env.VITE_CONTRACT_ID as string;
const RPC_URL = import.meta.env.VITE_RPC_URL ?? "https://soroban-testnet.stellar.org";
const NETWORK_PASSPHRASE = Networks.TESTNET;

const server = new SorobanRpc.Server(RPC_URL, { allowHttp: true });
const contract = new Contract(CONTRACT_ID);

// ── helpers ──────────────────────────────────────────────────────────────────

function addr(a: string) { return Address.fromString(a).toScVal(); }
function str(s: string) { return nativeToScVal(s, { type: "string" }); }
function optStr(s: string | null) { return s == null ? nativeToScVal(null) : str(s); }
function optU64(n: bigint | null) { return n == null ? nativeToScVal(null) : nativeToScVal(n, { type: "u64" }); }

const DUMMY = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";

async function simulate<T>(method: string, ...args: xdr.ScVal[]): Promise<T> {
  const account = new Account(DUMMY, "0");
  const tx = new TransactionBuilder(account, { fee: BASE_FEE, networkPassphrase: NETWORK_PASSPHRASE })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();
  const result = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(result)) throw new Error(result.error);
  const ok = result as SorobanRpc.Api.SimulateTransactionSuccessResponse;
  if (!ok.result) throw new Error(`No result from ${method}`);
  return scValToNative(ok.result.retval) as T;
}

async function invoke(caller: string, method: string, ...args: xdr.ScVal[]): Promise<void> {
  const account = await server.getAccount(caller);
  const tx = new TransactionBuilder(account, { fee: BASE_FEE, networkPassphrase: NETWORK_PASSPHRASE })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  const simResult = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(simResult)) throw new Error(simResult.error);

  const prepared = await server.prepareTransaction(tx);
  const signed = await sign(prepared.toXDR(), NETWORK_PASSPHRASE);
  const submitted = await server.sendTransaction(
    TransactionBuilder.fromXDR(signed, NETWORK_PASSPHRASE)
  );

  if (submitted.status === "ERROR") throw new Error(submitted.errorResult?.toXDR() ?? "tx error");

  // Poll for confirmation
  let attempts = 0;
  while (attempts < 20) {
    await new Promise((r) => setTimeout(r, 1500));
    const status = await server.getTransaction(submitted.hash);
    if (status.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) return;
    if (status.status === SorobanRpc.Api.GetTransactionStatus.FAILED)
      throw new Error("Transaction failed");
    attempts++;
  }
  throw new Error("Transaction timed out");
}

// ── read ─────────────────────────────────────────────────────────────────────

export interface Attestation {
  id: string;
  issuer: string;
  subject: string;
  claim_type: string;
  timestamp: bigint;
  expiration: bigint | null;
  revoked: boolean;
  metadata: string | null;
}

export async function getSubjectAttestations(subject: string): Promise<Attestation[]> {
  return simulate("get_subject_attestations", addr(subject), nativeToScVal(0, { type: "u32" }), nativeToScVal(50, { type: "u32" }));
}

export async function hasValidClaim(subject: string, claimType: string): Promise<boolean> {
  return simulate("has_valid_claim", addr(subject), str(claimType));
}

export async function isIssuer(address: string): Promise<boolean> {
  return simulate("is_issuer", addr(address));
}

export async function getAdmin(): Promise<string> {
  return simulate("get_admin");
}

export async function listClaimTypes(): Promise<string[]> {
  return simulate("list_claim_types", nativeToScVal(0, { type: "u32" }), nativeToScVal(50, { type: "u32" }));
}

// ── write ─────────────────────────────────────────────────────────────────────

export async function registerIssuer(admin: string, issuer: string): Promise<void> {
  return invoke(admin, "register_issuer", addr(admin), addr(issuer));
}

export async function removeIssuer(admin: string, issuer: string): Promise<void> {
  return invoke(admin, "remove_issuer", addr(admin), addr(issuer));
}

export async function createAttestation(
  issuer: string,
  subject: string,
  claimType: string,
  expiration: bigint | null,
  metadata: string | null
): Promise<void> {
  return invoke(
    issuer,
    "create_attestation",
    addr(issuer),
    addr(subject),
    str(claimType),
    optU64(expiration),
    optStr(metadata),
    nativeToScVal(null) // tags
  );
}

export async function revokeAttestation(
  issuer: string,
  attestationId: string,
  reason: string | null
): Promise<void> {
  return invoke(issuer, "revoke_attestation", addr(issuer), str(attestationId), optStr(reason));
}

// ── attestation requests ─────────────────────────────────────────────────────

export interface AttestationRequest {
  id: string;
  subject: string;
  issuer: string;
  claim_type: string;
  status: "pending" | "fulfilled" | "rejected";
  created_at: bigint;
  fulfilled_at: bigint | null;
}

export async function submitAttestationRequest(
  subject: string,
  issuer: string,
  claimType: string
): Promise<void> {
  return invoke(
    subject,
    "submit_attestation_request",
    addr(subject),
    addr(issuer),
    str(claimType)
  );
}

export async function getSubjectRequests(subject: string): Promise<AttestationRequest[]> {
  return simulate("get_subject_requests", addr(subject), nativeToScVal(0, { type: "u32" }), nativeToScVal(50, { type: "u32" }));
}

export async function getIssuerRequests(issuer: string): Promise<AttestationRequest[]> {
  return simulate("get_issuer_requests", addr(issuer), nativeToScVal(0, { type: "u32" }), nativeToScVal(50, { type: "u32" }));
}

export async function fulfillRequest(
  issuer: string,
  requestId: string,
  expiration: bigint | null
): Promise<void> {
  return invoke(
    issuer,
    "fulfill_request",
    addr(issuer),
    str(requestId),
    optU64(expiration)
  );
}

export async function rejectRequest(
  issuer: string,
  requestId: string,
  reason: string | null
): Promise<void> {
  return invoke(
    issuer,
    "reject_request",
    addr(issuer),
    str(requestId),
    optStr(reason)
  );
}

// ── multi-sig proposals ──────────────────────────────────────────────────────

export interface MultiSigProposal {
  id: string;
  proposer: string;
  subject: string;
  claim_type: string;
  required_signers: string[];
  signers: string[];
  threshold: number;
  expires_at: bigint;
  finalized: boolean;
}

export async function proposeAttestation(
  proposer: string,
  subject: string,
  claimType: string,
  requiredSigners: string[],
  threshold: number
): Promise<string> {
  return invoke(
    proposer,
    "propose_attestation",
    addr(proposer),
    addr(subject),
    str(claimType),
    nativeToScVal(requiredSigners.map((s) => Address.fromString(s).toScVal()), { type: "vec" }),
    nativeToScVal(threshold, { type: "u32" })
  );
}

export async function cosignAttestation(
  signer: string,
  proposalId: string
): Promise<void> {
  return invoke(
    signer,
    "cosign_attestation",
    addr(signer),
    str(proposalId)
  );
}

export async function getMultiSigProposal(proposalId: string): Promise<MultiSigProposal> {
  return simulate("get_multisig_proposal", str(proposalId));
}

// ── issuer stats ─────────────────────────────────────────────────────────────

export interface IssuerStats {
  total_issued: number;
  active: number;
  revoked: number;
  expired: number;
}

export async function getIssuerStats(issuer: string): Promise<IssuerStats> {
  return simulate("get_issuer_stats", addr(issuer));
}

export async function getExpiringAttestations(
  issuer: string,
  daysWindow: number
): Promise<Attestation[]> {
  return simulate(
    "get_issuer_expiring_attestations",
    addr(issuer),
    nativeToScVal(daysWindow, { type: "u32" }),
    nativeToScVal(0, { type: "u32" }),
    nativeToScVal(50, { type: "u32" })
  );
}
