import {
  Contract,
  Networks,
  rpc,
  TransactionBuilder,
  BASE_FEE,
  xdr,
} from "@stellar/stellar-sdk";
import type {
  Attestation,
  AttestationStatus,
  CouncilProposal,
  Council,
  ContractMetadata,
  FeeConfig,
  IssuerMetadata,
  MultiSigProposal,
  StorageLimits,
} from "./types.js";

export interface TrustLinkClientOptions {
  /** Stellar RPC server URL. */
  rpcUrl: string;
  /** TrustLink contract ID. */
  contractId: string;
  /** Stellar network passphrase. Defaults to Testnet. */
  networkPassphrase?: string;
}

/**
 * TrustLinkClient — typed wrapper around the TrustLink Soroban contract.
 *
 * All read methods use `simulateTransaction` so they require no signing or fees.
 */
export class TrustLinkClient {
  private readonly server: rpc.Server;
  private readonly contract: Contract;
  private readonly networkPassphrase: string;
  private readonly contractId: string;

  constructor(options: TrustLinkClientOptions) {
    this.rpcUrl = options.rpcUrl;
    this.contractId = options.contractId;
    this.networkPassphrase =
      options.networkPassphrase ?? Networks.TESTNET;
    this.server = new rpc.Server(options.rpcUrl, { allowHttp: true });
    this.contract = new Contract(options.contractId);
  }

  // ---------------------------------------------------------------------------
  // Internal helpers
  // ---------------------------------------------------------------------------

  private rpcUrl: string;

  /** Build and simulate a read-only contract call, returning the decoded value. */
  private async simulate<T>(method: string, args: xdr.ScVal[]): Promise<T> {
    const account = await this.server.getAccount(
      "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN"
    );
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(30)
      .build();

    const result = await this.server.simulateTransaction(tx);
    if (rpc.Api.isSimulationError(result)) {
      throw new Error(`Contract error in ${method}: ${result.error}`);
    }
    if (!rpc.Api.isSimulationSuccess(result) || !result.result) {
      throw new Error(`Unexpected simulation result for ${method}`);
    }
    return scValToNative(result.result.retval) as T;
  }

  // ---------------------------------------------------------------------------
  // Attestation queries
  // ---------------------------------------------------------------------------

  /** Fetch a single attestation by its ID. */
  async getAttestation(attestationId: string): Promise<Attestation> {
    return this.simulate<Attestation>("get_attestation", [
      xdr.ScVal.scvString(attestationId),
    ]);
  }

  /** Fetch the live status of an attestation. */
  async getAttestationStatus(attestationId: string): Promise<AttestationStatus> {
    return this.simulate<AttestationStatus>("get_attestation_status", [
      xdr.ScVal.scvString(attestationId),
    ]);
  }

  /** All attestation IDs for a subject address. */
  async getSubjectAttestations(subject: string): Promise<string[]> {
    return this.simulate<string[]>("get_subject_attestations", [
      xdr.ScVal.scvAddress(xdr.ScAddress.scAddressTypeAccount(
        xdr.PublicKey.publicKeyTypeEd25519(Buffer.from(subject, "hex"))
      )),
    ]);
  }

  /**
   * Fetch attestations whose `timestamp` falls within [start, end] (inclusive).
   *
   * Maps to the contract's `get_attestations_in_range(start, end)` entry point.
   *
   * @param start - Lower-bound Unix timestamp (seconds).
   * @param end   - Upper-bound Unix timestamp (seconds).
   * @returns Array of matching attestations.
   */
  async getAttestationsInRange(
    start: number,
    end: number
  ): Promise<Attestation[]> {
    return this.simulate<Attestation[]>("get_attestations_in_range", [
      xdr.ScVal.scvU64(xdr.Uint64.fromString(String(start))),
      xdr.ScVal.scvU64(xdr.Uint64.fromString(String(end))),
    ]);
  }

  /**
   * Cursor-based range query — returns up to `limit` attestations created
   * **after** the attestation identified by `cursor`.
   *
   * Maps to the contract's `get_attestations_in_range_after(cursor, limit)`.
   *
   * @param cursor - ID of the last seen attestation (exclusive lower bound).
   * @param limit  - Maximum number of results to return.
   * @returns Array of attestations following the cursor.
   */
  async getAttestationsInRangeAfter(
    cursor: string,
    limit: number
  ): Promise<Attestation[]> {
    return this.simulate<Attestation[]>("get_attestations_in_range_after", [
      xdr.ScVal.scvString(cursor),
      xdr.ScVal.scvU32(limit),
    ]);
  }

  /** Attestations issued by a given issuer address. */
  async getIssuerAttestations(issuer: string): Promise<string[]> {
    return this.simulate<string[]>("get_issuer_attestations", [
      xdr.ScVal.scvAddress(xdr.ScAddress.scAddressTypeAccount(
        xdr.PublicKey.publicKeyTypeEd25519(Buffer.from(issuer, "hex"))
      )),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Admin / Council (issue #742)
  // ---------------------------------------------------------------------------

  /**
   * Fetch the current admin council configuration.
   *
   * Maps to the contract's `get_council()` entry point.
   */
  async getCouncil(): Promise<Council> {
    return this.simulate<Council>("get_council", []);
  }

  /**
   * Fetch a single admin-council proposal by its ID.
   *
   * Maps to the contract's `get_council_proposal(proposal_id)` entry point.
   *
   * @param proposalId - The unique proposal identifier.
   */
  async getCouncilProposal(proposalId: string): Promise<CouncilProposal> {
    return this.simulate<CouncilProposal>("get_council_proposal", [
      xdr.ScVal.scvString(proposalId),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Storage limits (issue #743)
  // ---------------------------------------------------------------------------

  /**
   * Fetch the contract's configured storage limits.
   *
   * Maps to the contract's `get_limits()` entry point.
   */
  async getLimits(): Promise<StorageLimits> {
    return this.simulate<StorageLimits>("get_limits", []);
  }

  // ---------------------------------------------------------------------------
  // Multi-sig proposals
  // ---------------------------------------------------------------------------

  /** Fetch an admin multi-sig proposal by ID. */
  async getMultisigProposal(proposalId: string): Promise<MultiSigProposal> {
    return this.simulate<MultiSigProposal>("get_multisig_proposal", [
      xdr.ScVal.scvString(proposalId),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Misc
  // ---------------------------------------------------------------------------

  async getAdmin(): Promise<string> {
    return this.simulate<string>("get_admin", []);
  }

  async getFeeConfig(): Promise<FeeConfig> {
    return this.simulate<FeeConfig>("get_fee_config", []);
  }

  async getIssuerMetadata(issuer: string): Promise<IssuerMetadata | undefined> {
    return this.simulate<IssuerMetadata | undefined>("get_issuer_metadata", [
      xdr.ScVal.scvAddress(xdr.ScAddress.scAddressTypeAccount(
        xdr.PublicKey.publicKeyTypeEd25519(Buffer.from(issuer, "hex"))
      )),
    ]);
  }

  async getContractMetadata(): Promise<ContractMetadata> {
    return this.simulate<ContractMetadata>("get_contract_metadata", []);
  }

  async getVersion(): Promise<string> {
    return this.simulate<string>("get_version", []);
  }
}

/** Decode a Soroban ScVal to a plain JS value (best-effort). */
function scValToNative(val: xdr.ScVal): unknown {
  switch (val.switch()) {
    case xdr.ScValType.scvString():
      return val.str().toString();
    case xdr.ScValType.scvSymbol():
      return val.sym().toString();
    case xdr.ScValType.scvBool():
      return val.b();
    case xdr.ScValType.scvU32():
      return val.u32();
    case xdr.ScValType.scvI32():
      return val.i32();
    case xdr.ScValType.scvU64():
      return Number(val.u64().toString());
    case xdr.ScValType.scvI64():
      return Number(val.i64().toString());
    case xdr.ScValType.scvVec(): {
      const vec = val.vec();
      return vec ? vec.map(scValToNative) : [];
    }
    case xdr.ScValType.scvMap(): {
      const entries = val.map() ?? [];
      const obj: Record<string, unknown> = {};
      for (const entry of entries) {
        const key = scValToNative(entry.key()) as string;
        obj[key] = scValToNative(entry.val());
      }
      return obj;
    }
    case xdr.ScValType.scvVoid():
      return undefined;
    default:
      return val;
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
  Delegation,
  Endorsement,
  FeeConfig,
  GlobalStats,
  HealthStatus,
  IssuerMetadata,
  IssuerStats,
  IssuerTier,
  MultiSigProposal,
  Network,
  SubjectDataExport,
  TrustLinkClientOptions,
} from "./types";
import { parseTrustLinkError } from "./types";

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
    const res = options.resilience ?? {};
    this.retryOptions = options.retry ?? {
      maxAttempts: res.maxRetries,
      initialDelayMs: res.backoffMs,
    };
    this.breaker = new CircuitBreaker(options.circuitBreaker ?? {
      failureThreshold: res.circuitBreakerThreshold,
    });
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
        const typed = parseTrustLinkError(result.error);
        if (typed) throw typed;
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

  async getAdminCouncil(): Promise<string[]> {
    return this.simulate("get_admin_council");
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

  async getIssuerList(start: number, limit: number): Promise<string[]> {
    return this.simulate("get_issuer_list", this.u32(start), this.u32(limit));
  }

  // ── Bridge Registry ────────────────────────────────────────────────────────

  async isBridge(address: string): Promise<boolean> {
    return this.simulate("is_bridge", this.addr(address));
  }

  async getBridgeList(start: number, limit: number): Promise<string[]> {
    return this.simulate("get_bridge_list", this.u32(start), this.u32(limit));
  }

  async getPendingAdminTransfer(): Promise<{ proposed_by: string; new_admin: string } | null> {
    return this.simulate("get_pending_admin_transfer");
  }

  // ── Claim Type Registry ────────────────────────────────────────────────────

  async getClaimTypeDescription(claimType: string): Promise<string | null> {
    return this.simulate("get_claim_type_description", this.str(claimType));
  }

  async listClaimTypes(start: number, limit: number): Promise<string[]> {
    return this.simulate("list_claim_types", this.u32(start), this.u32(limit));
  }

  /** Returns whether the given claim type is registered in the contract registry. */
  async getRegisteredClaimType(claimType: string): Promise<boolean> {
    return this.simulate("get_registered_claim_type", this.str(claimType));
  }

  /**
   * Returns whether the contract requires claim types to be pre-registered.
   * When true, free-text claim types are rejected on attestation creation.
   */
  async getRequireRegisteredClaimType(): Promise<boolean> {
    return this.simulate("get_require_registered_claim_type");
  }

  // ── Rate Limiting ──────────────────────────────────────────────────────────

  /**
   * Returns the per-claim-type rate limit configuration.
   * Distinct from getRateLimit(), which operates at the per-issuer level.
   */
  async getRateLimitForClaimType(claimType: string): Promise<bigint> {
    return this.simulate("get_rate_limit_for_claim_type", this.str(claimType));
  // ── Delegation Queries ────────────────────────────────────────────────────

  async getDelegation(
    delegator: string,
    delegate: string,
    claimType: string
  ): Promise<Delegation | null> {
    return this.simulate(
      "get_delegation",
      this.addr(delegator),
      this.addr(delegate),
      this.str(claimType)
    );
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

  async getAttestationsByTag(subject: string, tag: string, start = 0, limit = 20): Promise<string[]> {
    const all = await this.simulate<string[]>(
      "get_attestations_by_tag",
      this.addr(subject),
      this.str(tag)
    );
    return all.slice(start, start + limit);
  }

  /**
   * Returns a paginated list of attestation IDs for a subject filtered by jurisdiction.
   *
   * @param subject     - Stellar address of the subject.
   * @param jurisdiction - Jurisdiction code to filter by (e.g. "US", "EU").
   * @param start       - Zero-based page offset.
   * @param limit       - Maximum number of IDs to return.
   */
  async getAttestationsByJurisdiction(
    subject: string,
    jurisdiction: string,
    start: number,
    limit: number
  ): Promise<string[]> {
    return this.simulate(
      "get_attestations_by_jurisdiction",
      this.addr(subject),
      this.str(jurisdiction),
      this.u32(start),
      this.u32(limit)
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
    // IssuerTier is a Soroban #[contracttype] enum — encode as ScVec([ScSymbol(variant)])
    return this.simulate(
      "has_valid_claim_from_tier",
      this.addr(subject),
      this.str(claimType),
      xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(minTier)])
    );
  }

  async getClaimTypeCount(claimType: string): Promise<bigint> {
    return this.simulate("get_claim_type_count", this.str(claimType));
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

  /** Returns the configurable proposal TTL in ledgers. Use this to inform proposers of the available co-signature window. */
  async getMultisigTtl(): Promise<bigint> {
    return this.simulate("get_multisig_ttl");
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

  /**
   * Returns the raw low-level request state for a given request ID.
   * Distinct from getAttestationRequest(), which returns the high-level processed object.
   * Use this to inspect raw on-chain request state before processing.
   */
  async getRequest(requestId: string): Promise<AttestationRequest> {
    return this.simulate("get_request", this.str(requestId));
  }

  // ── Endorsements ──────────────────────────────────────────────────────────

  async getEndorsements(attestationId: string): Promise<Endorsement[]> {
    return this.simulate("get_endorsements", this.str(attestationId));
  }

  async getEndorsementCount(attestationId: string): Promise<number> {
    return this.simulate("get_endorsement_count", this.str(attestationId));
  }

  // ── Issue #530: Template management ───────────────────────────────────────

  async getTemplate(issuer: string, templateId: string): Promise<import("./types").AttestationTemplate> {
    return this.simulate("get_template", this.addr(issuer), this.str(templateId));
  }

  async listEndorsementsByEndorser(endorser: string, start: number, limit: number): Promise<Endorsement[]> {
    return this.simulate("list_endorsements_by_endorser", this.addr(endorser), this.u32(start), this.u32(limit));
  }

  async bulkAddToWhitelist(issuer: string, subjects: string[]): Promise<void> {
    const subjectsVal = xdr.ScVal.scvVec(subjects.map(s => this.addr(s)));
    return this.simulate("bulk_add_to_whitelist", this.addr(issuer), subjectsVal);
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

  // ── Data Portability ───────────────────────────────────────────────────────

  /**
   * Export all data held about a subject in a single structured JSON object,
   * suitable for a GDPR Article 20 / CCPA data-portability response.
   *
   * Aggregates:
   *  - All attestations via iterateSubjectAttestations
   *  - Audit log for each attestation via getAuditLog
   *  - Endorsements for each attestation via getEndorsements
   *  - Optional request history (pass known request IDs in options.requestIds)
   *
   * The contract does not expose a per-subject request index, so request
   * history is only included when the caller supplies known request IDs
   * (e.g. from an off-chain database or the indexer).
   */
  async exportSubjectData(
    subject: string,
    options?: { requestIds?: string[] }
  ): Promise<SubjectDataExport> {
    const attestations: Attestation[] = [];
    for await (const att of this.iterateSubjectAttestations(subject)) {
      attestations.push(att);
    }

    const attestationData = await Promise.all(
      attestations.map(async (att) => {
        const [auditLog, endorsements] = await Promise.all([
          this.getAuditLog(att.id),
          this.getEndorsements(att.id),
        ]);
        return { attestation: att, auditLog, endorsements };
      })
    );

    const requestHistory: AttestationRequest[] = [];
    if (options?.requestIds?.length) {
      const requests = await Promise.all(
        options.requestIds.map((id) => this.getAttestationRequest(id))
      );
      requestHistory.push(...requests);
    }

    const allEndorsements = attestationData.flatMap((d) => d.endorsements);
    const allAuditEntries = attestationData.flatMap((d) => d.auditLog);

    return {
      subject,
      exportedAt: new Date().toISOString(),
      attestations: attestationData,
      requestHistory,
      summary: {
        totalAttestations: attestations.length,
        activeAttestations: attestations.filter((a) => !a.revoked && !a.deleted).length,
        revokedAttestations: attestations.filter((a) => a.revoked).length,
        deletedAttestations: attestations.filter((a) => a.deleted).length,
        totalEndorsements: allEndorsements.length,
        totalAuditEntries: allAuditEntries.length,
      },
    };
  }
}
