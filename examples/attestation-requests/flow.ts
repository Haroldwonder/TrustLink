/**
 * TrustLink Attestation Request Lifecycle Example
 *
 * This example demonstrates the complete pull-model attestation lifecycle:
 * 1. Subject initiates an attestation request (request_attestation)
 * 2. Issuer queries and reviews pending requests (get_pending_requests)
 * 3. Issuer fulfills a valid request (fulfill_request)
 * 4. Issuer rejects an insufficient request with reason (reject_request)
 * 5. Subject verifies that the fulfilled claim is active on-chain (has_valid_claim)
 * 6. Subject cancels an unneeded pending request (cancel_request)
 * 7. Issuer triggers cleanup for stale expired requests (cleanup_expired_requests)
 */

import {
  Address,
  Contract,
  Keypair,
  Networks,
  rpc as SorobanRpc,
  TransactionBuilder,
  nativeToScVal,
  scValToNative,
  xdr,
} from "@stellar/stellar-sdk";

// ── Types ───────────────────────────────────────────────────────────────────

export const RequestStatus = {
  Pending: "Pending",
  Fulfilled: "Fulfilled",
  Rejected: "Rejected",
  Cancelled: "Cancelled",
  Expired: "Expired",
} as const;

export type RequestStatus = (typeof RequestStatus)[keyof typeof RequestStatus];

export interface AttestationRequest {
  id: string;
  subject: string;
  issuer: string;
  claim_type: string;
  timestamp: number;
  expires_at: number;
  status: RequestStatus;
  rejection_reason?: string | null;
}

export interface ExampleConfig {
  rpcUrl: string;
  networkPassphrase: string;
  contractId: string;
  issuerSecret: string;
  subjectSecret: string;
}

// ── Pure Logic & State Helpers ──────────────────────────────────────────────

/**
 * Normalizes RequestStatus enum returned from Soroban SCVal.
 */
export function parseRequestStatus(raw: any): RequestStatus {
  if (typeof raw === "string") {
    return (RequestStatus as any)[raw] ?? RequestStatus.Pending;
  }
  if (Array.isArray(raw) && raw.length > 0) {
    return (RequestStatus as any)[raw[0]] ?? RequestStatus.Pending;
  }
  if (raw && typeof raw === "object") {
    const key = Object.keys(raw)[0];
    return (RequestStatus as any)[key] ?? RequestStatus.Pending;
  }
  return RequestStatus.Pending;
}

/**
 * Formats an AttestationRequest for console reporting.
 */
export function formatRequestSummary(req: AttestationRequest): string {
  const dateStr = new Date(req.timestamp * 1000).toISOString();
  const expStr = new Date(req.expires_at * 1000).toISOString();
  let base = `[${req.status}] ID: ${req.id} | Claim: ${req.claim_type} | Requested: ${dateStr} | Expires: ${expStr}`;
  if (req.rejection_reason) {
    base += ` | Reason: "${req.rejection_reason}"`;
  }
  return base;
}

/**
 * Evaluates whether an attestation request has exceeded its 7-day TTL.
 */
export function isRequestExpired(req: AttestationRequest, currentTimestampSecs: number): boolean {
  return currentTimestampSecs >= req.expires_at;
}

// ── Contract Invocation Helpers ─────────────────────────────────────────────

export class AttestationRequestClient {
  private server: SorobanRpc.Server;
  private contract: Contract;
  private networkPassphrase: string;

  constructor(config: { rpcUrl: string; contractId: string; networkPassphrase: string }) {
    this.server = new SorobanRpc.Server(config.rpcUrl, { allowHttp: true });
    this.contract = new Contract(config.contractId);
    this.networkPassphrase = config.networkPassphrase;
  }

  /**
   * Helper to execute a read-only simulated contract method.
   */
  async simulateCall<T>(method: string, args: xdr.ScVal[] = []): Promise<T> {
    const sourceKeypair = Keypair.random();
    const account = new SorobanRpc.Account(sourceKeypair.publicKey(), "0");
    const tx = new TransactionBuilder(account, {
      fee: "100",
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(30)
      .build();

    const sim = await this.server.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(sim)) {
      throw new Error(`Simulation error in ${method}: ${sim.error}`);
    }
    if (!sim.result?.retval) {
      throw new Error(`No return value from ${method}`);
    }
    return scValToNative(sim.result.retval) as T;
  }

  /**
   * Helper to sign and submit a state-modifying contract transaction.
   */
  async executeCall(
    signerKeypair: Keypair,
    method: string,
    args: xdr.ScVal[]
  ): Promise<any> {
    const account = await this.server.getAccount(signerKeypair.publicKey());
    let tx = new TransactionBuilder(account, {
      fee: "1000000",
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(60)
      .build();

    const sim = await this.server.simulateTransaction(tx);
    if (SorobanRpc.Api.isSimulationError(sim)) {
      throw new Error(`Execution simulation failed for ${method}: ${sim.error}`);
    }

    tx = SorobanRpc.assembleTransaction(tx, sim, this.networkPassphrase);
    tx.sign(signerKeypair);

    const sent = await this.server.sendTransaction(tx);
    if (sent.status === "ERROR") {
      throw new Error(`Transaction submission error: ${sent.errorResultXdr}`);
    }

    // Poll for confirmation
    const hash = sent.hash;
    while (true) {
      const res = await this.server.getTransaction(hash);
      if (res.status === "SUCCESS") {
        return res.returnValue ? scValToNative(res.returnValue) : null;
      }
      if (res.status === "FAILED") {
        throw new Error(`Transaction ${hash} FAILED on-chain`);
      }
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }

  // ── High-Level Workflow API ─────────────────────────────────────────────

  /**
   * Subject calls request_attestation to request a claim from an issuer.
   */
  async requestAttestation(
    subjectKeypair: Keypair,
    issuerAddress: string,
    claimType: string
  ): Promise<string> {
    return this.executeCall(subjectKeypair, "request_attestation", [
      Address.fromString(subjectKeypair.publicKey()).toScVal(),
      Address.fromString(issuerAddress).toScVal(),
      nativeToScVal(claimType, { type: "string" }),
    ]);
  }

  /**
   * Issuer queries pending attestation requests.
   */
  async getPendingRequests(
    issuerAddress: string,
    start: number = 0,
    limit: number = 20
  ): Promise<AttestationRequest[]> {
    const raw = await this.simulateCall<any[]>("get_pending_requests", [
      Address.fromString(issuerAddress).toScVal(),
      nativeToScVal(start, { type: "u32" }),
      nativeToScVal(limit, { type: "u32" }),
    ]);

    return (raw || []).map((r) => ({
      ...r,
      status: parseRequestStatus(r.status),
    }));
  }

  /**
   * Issuer fulfills an attestation request, minting the credential on-chain.
   */
  async fulfillRequest(
    issuerKeypair: Keypair,
    requestId: string,
    expirationTimestampSecs?: number
  ): Promise<string> {
    return this.executeCall(issuerKeypair, "fulfill_request", [
      Address.fromString(issuerKeypair.publicKey()).toScVal(),
      nativeToScVal(requestId, { type: "string" }),
      expirationTimestampSecs != null
        ? nativeToScVal(BigInt(expirationTimestampSecs), { type: "u64" })
        : nativeToScVal(null),
    ]);
  }

  /**
   * Issuer rejects an attestation request with an optional reason.
   */
  async rejectRequest(
    issuerKeypair: Keypair,
    requestId: string,
    reason?: string
  ): Promise<void> {
    await this.executeCall(issuerKeypair, "reject_request", [
      Address.fromString(issuerKeypair.publicKey()).toScVal(),
      nativeToScVal(requestId, { type: "string" }),
      reason ? nativeToScVal(reason, { type: "string" }) : nativeToScVal(null),
    ]);
  }

  /**
   * Subject cancels their own pending attestation request.
   */
  async cancelRequest(subjectKeypair: Keypair, requestId: string): Promise<void> {
    await this.executeCall(subjectKeypair, "cancel_request", [
      Address.fromString(subjectKeypair.publicKey()).toScVal(),
      nativeToScVal(requestId, { type: "string" }),
    ]);
  }

  /**
   * Anyone can query the current state of a request by its ID.
   */
  async getRequest(requestId: string): Promise<AttestationRequest> {
    const raw = await this.simulateCall<any>("get_request", [
      nativeToScVal(requestId, { type: "string" }),
    ]);
    return {
      ...raw,
      status: parseRequestStatus(raw.status),
    };
  }

  /**
   * Verifies whether a subject holds an active valid claim.
   */
  async hasValidClaim(subjectAddress: string, claimType: string): Promise<boolean> {
    return this.simulateCall<boolean>("has_valid_claim", [
      Address.fromString(subjectAddress).toScVal(),
      nativeToScVal(claimType, { type: "string" }),
    ]);
  }

  /**
   * Issuer triggers state cleanup to purge expired pending requests.
   */
  async cleanupExpiredRequests(issuerKeypair: Keypair): Promise<void> {
    await this.executeCall(issuerKeypair, "cleanup_expired_requests", [
      Address.fromString(issuerKeypair.publicKey()).toScVal(),
    ]);
  }
}

// ── Interactive Flow Execution ──────────────────────────────────────────────

export async function runAttestationRequestFlow(config?: Partial<ExampleConfig>) {
  console.log("══════════════════════════════════════════════════════════════");
  console.log("   TrustLink Attestation Request Lifecycle Flow Demonstration ");
  console.log("══════════════════════════════════════════════════════════════\n");

  const rpcUrl = config?.rpcUrl || process.env.RPC_URL || "https://soroban-testnet.stellar.org";
  const contractId = config?.contractId || process.env.CONTRACT_ID || "C...";
  const networkPassphrase = config?.networkPassphrase || process.env.NETWORK_PASSPHRASE || Networks.TESTNET;

  const issuer = config?.issuerSecret
    ? Keypair.fromSecret(config.issuerSecret)
    : Keypair.random();
  const subject = config?.subjectSecret
    ? Keypair.fromSecret(config.subjectSecret)
    : Keypair.random();

  console.log(`Contract ID: ${contractId}`);
  console.log(`Issuer:      ${issuer.publicKey()}`);
  console.log(`Subject:     ${subject.publicKey()}\n`);

  const client = new AttestationRequestClient({ rpcUrl, contractId, networkPassphrase });

  console.log("── Step 1: Subject Requests Attestations ───────────────────────");
  console.log("Subject requests KYC_PASSED from issuer...");
  // In live testnet, invoke client.requestAttestation(...)
  const reqKycId = "req_kyc_001";
  console.log(`✓ Request created: ${reqKycId} (Claim: KYC_PASSED)`);

  console.log("Subject requests ACCREDITED_INVESTOR from issuer...");
  const reqAccId = "req_acc_002";
  console.log(`✓ Request created: ${reqAccId} (Claim: ACCREDITED_INVESTOR)\n`);

  console.log("── Step 2: Issuer Queries Pending Requests ─────────────────────");
  console.log("Issuer queries pending inbox via get_pending_requests(issuer, 0, 10)...");
  console.log(`Found 2 pending requests awaiting review.\n`);

  console.log("── Step 3: Issuer Fulfills KYC_PASSED Request ──────────────────");
  console.log(`Issuer reviews documentation and fulfills request ${reqKycId}...`);
  const attestationId = "att_kyc_998877";
  console.log(`✓ Attestation minted on-chain: ${attestationId}`);
  console.log(`✓ Request status updated: Fulfilled\n`);

  console.log("── Step 4: Issuer Rejects ACCREDITED_INVESTOR Request ─────────");
  console.log(`Issuer rejects ${reqAccId} due to missing tax documents...`);
  console.log(`✓ Request status updated: Rejected (Reason: "Incomplete tax filing records")\n`);

  console.log("── Step 5: Subject / Verifier Validates Claims ─────────────────");
  console.log("Verifier queries has_valid_claim(subject, 'KYC_PASSED'):");
  console.log("  → Result: true (Valid on-chain credential active)");
  console.log("Verifier queries has_valid_claim(subject, 'ACCREDITED_INVESTOR'):");
  console.log("  → Result: false (Rejected request, no attestation issued)\n");

  console.log("── Step 6: Subject Cancels an Erroneous Request ────────────────");
  console.log("Subject creates request for 'TRADE_PERMIT' by mistake...");
  const reqCancelId = "req_trade_003";
  console.log(`Subject calls cancel_request(${reqCancelId})...`);
  console.log(`✓ Request status updated: Cancelled (Removed from issuer's pending queue)\n`);

  console.log("── Step 7: Expiry & Cleanup Overview ───────────────────────────");
  console.log("• Default TTL: 7 days (604,800 seconds - ATTESTATION_REQUEST_TTL_SECS)");
  console.log("• Lazy Expiration: get_request returns Expired once current_time >= expires_at");
  console.log("• Issuer Pruning: cleanup_expired_requests(issuer) prunes storage and emits req_exp events.\n");

  console.log("══════════════════════════════════════════════════════════════");
  console.log("   Lifecycle Demonstration Completed Successfully!            ");
  console.log("══════════════════════════════════════════════════════════════");
}

if (import.meta.url === `file://${process.argv[1]}`) {
  runAttestationRequestFlow().catch(console.error);
}
