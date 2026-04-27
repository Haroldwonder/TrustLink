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

import type {
  Attestation,
  AttestationRequest,
  AttestationStatus,
  AuditEntry,
  ClaimTypeInfo,
  ContractConfig,
  ContractMetadata,
  Endorsement,
  FeeConfig,
  GlobalStats,
  HealthStatus,
  IssuerMetadata,
  IssuerStats,
  IssuerTier,
  MultiSigProposal,
  Network,
  TrustLinkClientOptions,
} from "./types";

import {
  CircuitBreaker,
  withRetry,
  type RetryOptions,
  type CircuitBreakerOptions,
} from "./resilience";

const RPC_URLS: Record<string, string> = {
  testnet: "https://soroban-testnet.stellar.org",
  mainnet: "https://mainnet.stellar.validationcloud.io/v1/XCSmR1nSS3we7PCXV4oMiA",
  local: "http://localhost:8000/soroban/rpc",
};

const NETWORK_PASSPHRASES: Record<string, string> = {
  testnet: Networks.TESTNET,
  mainnet: Networks.PUBLIC,
  local: Networks.STANDALONE,
};

/**
 * TrustLinkClient — TypeScript wrapper for the TrustLink Soroban smart contract.
 *
 * All read-only methods use `simulateTransaction` under the hood and return
 * decoded native values. Write methods return the raw simulation result so
 * callers can sign and submit with their own keypair / wallet.
 */
export class TrustLinkClient {
  private readonly server: SorobanRpc.Server;
  private readonly contract: Contract;
  private readonly networkPassphrase: string;
  private readonly rpcUrl: string;
  private readonly retryOptions: RetryOptions;
  private readonly breaker: CircuitBreaker;

  constructor(options: TrustLinkClientOptions) {
    const { contractId, network, rpcUrl } = options;

    this.rpcUrl =
      rpcUrl ??
      (RPC_URLS[network as string] ?? (network as string));

    this.networkPassphrase =
      NETWORK_PASSPHRASES[network as string] ?? Networks.TESTNET;

    this.server = new SorobanRpc.Server(this.rpcUrl, { allowHttp: true });
    this.contract = new Contract(contractId);
    this.retryOptions = options.retry ?? {};
    this.breaker = new CircuitBreaker(options.circuitBreaker ?? {});
  }

  // ── Helpers ────────────────────────────────────────────────────────────────

  /**
   * Simulate a read-only contract call and return the decoded result.
   * Uses a throwaway source account (the zero address) since no auth is needed.
   * Retries with exponential backoff and respects the circuit breaker.
   */
  private async simulate<T>(method: string, ...args: xdr.ScVal[]): Promise<T> {
    return withRetry(async () => {
      const dummySource = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
      const account = new Account(dummySource, "0");

      const tx = new TransactionBuilder(account, {
        fee: BASE_FEE,
        networkPassphrase: this.networkPassphrase,
      })
        .addOperation(this.contract.call(method, ...args))
        .setTimeout(30)
        .build();

      const result = await this.server.simulateTransaction(tx);

      if (SorobanRpc.Api.isSimulationError(result)) {
        throw new Error(`Contract simulation failed: ${result.error}`);
      }

      const simSuccess = result as SorobanRpc.Api.SimulateTransactionSuccessResponse;
      if (!simSuccess.result) {
        throw new Error(`No result returned from simulation of ${method}`);
      }

      return scValToNative(simSuccess.result.retval) as T;
    }, this.retryOptions, this.breaker);
  }

  private addr(address: string): xdr.ScVal {
    return Address.fromString(address).toScVal();
  }

  private str(value: string): xdr.ScVal {
    return nativeToScVal(value, { type: "string" });
  }

  private u32(value: number): xdr.ScVal {
    return nativeToScVal(value, { type: "u32" });
  }

  private u64(value: bigint): xdr.ScVal {
    return nativeToScVal(value, { type: "u64" });
  }

  private optU64(value: bigint | null | undefined): xdr.ScVal {
    if (value == null) return nativeToScVal(null);
    return nativeToScVal(value, { type: "u64" });
  }

  private optStr(value: string | null | undefined): xdr.ScVal {
    if (value == null) return nativeToScVal(null);
    return nativeToScVal(value, { type: "string" });
  }

  private strVec(values: string[]): xdr.ScVal {
    return nativeToScVal(values, { type: "array" });
  }

  // ── Admin / Initialization ─────────────────────────────────────────────────

  async getAdmin(): Promise<string> {
    return this.simulate("get_admin");
  }

  async getVersion(): Promise<string> {
    return this.simulate("get_version");
  }

  async isPaused(): Promise<boolean> {
    return this.simulate("is_paused");
  }

  async healthCheck(): Promise<HealthStatus> {
    return this.simulate("health_check");
  }

  async getGlobalStats(): Promise<GlobalStats> {
    return this.simulate("get_global_stats");
  }

  async getContractMetadata(): Promise<ContractMetadata> {
    return this.simulate("get_contract_metadata");
  }

  async getConfig(): Promise<ContractConfig> {
    return this.simulate("get_config");
  }

  async getFeeConfig(): Promise<FeeConfig> {
    return this.simulate("get_fee_config");
  }

  // ── Issuer Registry ────────────────────────────────────────────────────────

  async isIssuer(address: string): Promise<boolean> {
    return this.simulate("is_issuer", this.addr(address));
  }

  async getIssuerStats(issuer: string): Promise<IssuerStats> {
    return this.simulate("get_issuer_stats", this.addr(issuer));
  }

  async getIssuerTier(issuer: string): Promise<IssuerTier | null> {
    return this.simulate("get_issuer_tier", this.addr(issuer));
  }

  async getIssuerMetadata(issuer: string): Promise<IssuerMetadata | null> {
    return this.simulate("get_issuer_metadata", this.addr(issuer));
  }

  // ── Bridge Registry ────────────────────────────────────────────────────────

  async isBridge(address: string): Promise<boolean> {
    return this.simulate("is_bridge", this.addr(address));
  }

  // ── Claim Type Registry ────────────────────────────────────────────────────

  async getClaimTypeDescription(claimType: string): Promise<string | null> {
    return this.simulate("get_claim_type_description", this.str(claimType));
  }

  async listClaimTypes(start: number, limit: number): Promise<string[]> {
    return this.simulate("list_claim_types", this.u32(start), this.u32(limit));
  }

  // ── Attestation Queries ────────────────────────────────────────────────────

  async getAttestation(attestationId: string): Promise<Attestation> {
    return this.simulate("get_attestation", this.str(attestationId));
  }

  async getAttestationStatus(attestationId: string): Promise<AttestationStatus> {
    return this.simulate("get_attestation_status", this.str(attestationId));
  }

  async getAttestationByType(
    subject: string,
    claimType: string
  ): Promise<Attestation> {
    return this.simulate(
      "get_attestation_by_type",
      this.addr(subject),
      this.str(claimType)
    );
  }

  async getSubjectAttestations(
    subject: string,
    start: number,
    limit: number
  ): Promise<Attestation[]> {
    return this.simulate(
      "get_subject_attestations",
      this.addr(subject),
      this.u32(start),
      this.u32(limit)
    );
  }

  async getIssuerAttestations(
    issuer: string,
    start: number,
    limit: number
  ): Promise<Attestation[]> {
    return this.simulate(
      "get_issuer_attestations",
      this.addr(issuer),
      this.u32(start),
      this.u32(limit)
    );
  }

  async getAttestationsByTag(subject: string, tag: string): Promise<string[]> {
    return this.simulate(
      "get_attestations_by_tag",
      this.addr(subject),
      this.str(tag)
    );
  }

  async getValidClaims(subject: string): Promise<string[]> {
    return this.simulate("get_valid_claims", this.addr(subject));
  }

  async getAuditLog(attestationId: string): Promise<AuditEntry[]> {
    return this.simulate("get_audit_log", this.str(attestationId));
  }

  // ── Claim Verification ─────────────────────────────────────────────────────

  async hasValidClaim(subject: string, claimType: string): Promise<boolean> {
    return this.simulate(
      "has_valid_claim",
      this.addr(subject),
      this.str(claimType)
    );
  }

  async hasValidClaimFromIssuer(
    subject: string,
    claimType: string,
    issuer: string
  ): Promise<boolean> {
    return this.simulate(
      "has_valid_claim_from_issuer",
      this.addr(subject),
      this.str(claimType),
      this.addr(issuer)
    );
  }

  async hasAnyClaim(subject: string, claimTypes: string[]): Promise<boolean> {
    return this.simulate(
      "has_any_claim",
      this.addr(subject),
      this.strVec(claimTypes)
    );
  }

  async hasAllClaims(subject: string, claimTypes: string[]): Promise<boolean> {
    return this.simulate(
      "has_all_claims",
      this.addr(subject),
      this.strVec(claimTypes)
    );
  }

  async hasValidClaimFromTier(
    subject: string,
    claimType: string,
    minTier: IssuerTier
  ): Promise<boolean> {
    const tierMap: Record<IssuerTier, number> = { Basic: 0, Verified: 1, Premium: 2 };
    return this.simulate(
      "has_valid_claim_from_tier",
      this.addr(subject),
      this.str(claimType),
      nativeToScVal(tierMap[minTier], { type: "u32" })
    );
  }

  // ── Count Queries ──────────────────────────────────────────────────────────

  async getSubjectAttestationCount(subject: string): Promise<bigint> {
    return this.simulate("get_subject_attestation_count", this.addr(subject));
  }

  async getIssuerAttestationCount(issuer: string): Promise<bigint> {
    return this.simulate("get_issuer_attestation_count", this.addr(issuer));
  }

  async getValidClaimCount(subject: string): Promise<bigint> {
    return this.simulate("get_valid_claim_count", this.addr(subject));
  }

  // ── Multi-Sig Proposals ────────────────────────────────────────────────────

  async getMultisigProposal(proposalId: string): Promise<MultiSigProposal> {
    return this.simulate("get_multisig_proposal", this.str(proposalId));
  }

  async proposeAttestation(
    proposer: string,
    subject: string,
    claimType: string,
    requiredSigners: string[],
    threshold: number
  ): Promise<SorobanRpc.Api.SimulateTransactionResponse> {
    const account = new Account(proposer, "0");
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(
        this.contract.call(
          "propose_attestation",
          this.addr(proposer),
          this.addr(subject),
          this.str(claimType),
          nativeToScVal(requiredSigners.map((s) => Address.fromString(s).toScVal()), { type: "array" }),
          this.u32(threshold)
        )
      )
      .setTimeout(30)
      .build();
    return this.server.simulateTransaction(tx);
  }

  async cosignAttestation(
    issuer: string,
    proposalId: string
  ): Promise<SorobanRpc.Api.SimulateTransactionResponse> {
    const account = new Account(issuer, "0");
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(
        this.contract.call("cosign_attestation", this.addr(issuer), this.str(proposalId))
      )
      .setTimeout(30)
      .build();
    return this.server.simulateTransaction(tx);
  }

  // ── Attestation Requests ───────────────────────────────────────────────────

  async requestAttestation(
    subject: string,
    issuer: string,
    claimType: string
  ): Promise<SorobanRpc.Api.SimulateTransactionResponse> {
    const account = new Account(subject, "0");
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(
        this.contract.call(
          "request_attestation",
          this.addr(subject),
          this.addr(issuer),
          this.str(claimType)
        )
      )
      .setTimeout(30)
      .build();
    return this.server.simulateTransaction(tx);
  }

  async fulfillRequest(
    issuer: string,
    requestId: string,
    expiration?: bigint
  ): Promise<SorobanRpc.Api.SimulateTransactionResponse> {
    const account = new Account(issuer, "0");
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(
        this.contract.call(
          "fulfill_request",
          this.addr(issuer),
          this.str(requestId),
          this.optU64(expiration ?? null)
        )
      )
      .setTimeout(30)
      .build();
    return this.server.simulateTransaction(tx);
  }

  async rejectRequest(
    issuer: string,
    requestId: string,
    reason?: string
  ): Promise<SorobanRpc.Api.SimulateTransactionResponse> {
    const account = new Account(issuer, "0");
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(
        this.contract.call(
          "reject_request",
          this.addr(issuer),
          this.str(requestId),
          this.optStr(reason ?? null)
        )
      )
      .setTimeout(30)
      .build();
    return this.server.simulateTransaction(tx);
  }

  async getAttestationRequest(requestId: string): Promise<AttestationRequest> {
    return this.simulate("get_attestation_request", this.str(requestId));
  }

  // ── Endorsements ──────────────────────────────────────────────────────────

  async getEndorsements(attestationId: string): Promise<Endorsement[]> {
    return this.simulate("get_endorsements", this.str(attestationId));
  }

  async getEndorsementCount(attestationId: string): Promise<number> {
    return this.simulate("get_endorsement_count", this.str(attestationId));
  }

  // ── Pagination Helpers ─────────────────────────────────────────────────────

  async *iterateSubjectAttestations(
    subject: string,
    pageSize = 20
  ): AsyncGenerator<Attestation> {
    let start = 0;
    while (true) {
      const page = await this.getSubjectAttestations(subject, start, pageSize);
      yield* page;
      if (page.length < pageSize) break;
      start += page.length;
    }
  }

  async *iterateIssuerAttestations(
    issuer: string,
    pageSize = 20
  ): AsyncGenerator<Attestation> {
    let start = 0;
    while (true) {
      const page = await this.getIssuerAttestations(issuer, start, pageSize);
      yield* page;
      if (page.length < pageSize) break;
      start += page.length;
    }
  }
}
