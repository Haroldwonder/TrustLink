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
  Template,
  TrustLinkClientOptions,
} from "./types";
import { parseTrustLinkError } from "./types";

import {
  CircuitBreaker,
  withRetry,
  type RetryOptions,
} from "./resilience";

import {
  validateAddress,
  validateClaimType,
  validateAttestationId,
  validateProposalId,
  validateRequestId,
  validateTemplateId,
  validatePositive,
  validateNonNegative,
} from "./validation";

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

    // Validate network is a known network name
    if (!RPC_URLS[network]) {
      throw new Error(
        `Invalid network: "${network}". Valid networks are: ${Object.keys(RPC_URLS).join(", ")}. ` +
        `For custom RPC URLs, use the 'rpcUrl' option instead.`
      );
    }

    // Validate rpcUrl if provided
    if (rpcUrl) {
      if (typeof rpcUrl !== "string" || !rpcUrl.trim()) {
        throw new Error(`Invalid rpcUrl: "${rpcUrl}" is not a valid URL.`);
      }
      const trimmed = rpcUrl.trim();
      if (!trimmed.startsWith("http://") && !trimmed.startsWith("https://")) {
        throw new Error(`Invalid rpcUrl: "${rpcUrl}" must start with http:// or https://`);
      }
    }

    this.rpcUrl = rpcUrl ?? RPC_URLS[network];
    this.networkPassphrase = NETWORK_PASSPHRASES[network];

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
   * Uses a throwaway source account since no auth is needed.
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

  /**
   * Returns the admin address of the contract.
   * @returns The admin Stellar address.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getAdmin(): Promise<string> {
    return this.simulate("get_admin");
  }

  /**
   * Returns the admin council members.
   * @returns Array of council member addresses.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getAdminCouncil(): Promise<string[]> {
    return this.simulate("get_admin_council");
  }

  /**
   * Returns the contract version.
   * @returns Version string.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getVersion(): Promise<string> {
    return this.simulate("get_version");
  }

  /**
   * Returns whether the contract is paused.
   * @returns True if paused, false otherwise.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async isPaused(): Promise<boolean> {
    return this.simulate("is_paused");
  }

  /**
   * Returns the health status of the contract.
   * @returns Health status object.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async healthCheck(): Promise<HealthStatus> {
    return this.simulate("health_check");
  }

  /**
   * Returns global statistics for the contract.
   * @returns Global stats object.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getGlobalStats(): Promise<GlobalStats> {
    return this.simulate("get_global_stats");
  }

  /**
   * Returns the contract metadata.
   * @returns Contract metadata object.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getContractMetadata(): Promise<ContractMetadata> {
    return this.simulate("get_contract_metadata");
  }

  /**
   * Returns the contract configuration.
   * @returns Contract config object.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getConfig(): Promise<ContractConfig> {
    return this.simulate("get_config");
  }

  /**
   * Returns the fee configuration.
   * @returns Fee config object.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getFeeConfig(): Promise<FeeConfig> {
    return this.simulate("get_fee_config");
  }

  // ── Issuer Registry ────────────────────────────────────────────────────────

  /**
   * Checks if an address is registered as an issuer.
   * @param address - Stellar address to check.
   * @returns True if the address is an issuer, false otherwise.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async isIssuer(address: string): Promise<boolean> {
    validateAddress(address);
    return this.simulate("is_issuer", this.addr(address));
  }

  /**
   * Returns statistics for an issuer.
   * @param issuer - Stellar address of the issuer.
   * @returns Issuer statistics.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the issuer is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getIssuerStats(issuer: string): Promise<IssuerStats> {
    validateAddress(issuer);
    return this.simulate("get_issuer_stats", this.addr(issuer));
  }

  /**
   * Returns the tier of an issuer.
   * @param issuer - Stellar address of the issuer.
   * @returns Issuer tier or null if not found.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getIssuerTier(issuer: string): Promise<IssuerTier | null> {
    validateAddress(issuer);
    return this.simulate("get_issuer_tier", this.addr(issuer));
  }

  /**
   * Returns the metadata for an issuer.
   * @param issuer - Stellar address of the issuer.
   * @returns Issuer metadata or null if not found.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getIssuerMetadata(issuer: string): Promise<IssuerMetadata | null> {
    validateAddress(issuer);
    return this.simulate("get_issuer_metadata", this.addr(issuer));
  }

  /**
   * Returns a paginated list of issuer addresses.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of addresses to return.
   * @returns Array of issuer addresses.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getIssuerList(start: number, limit: number): Promise<string[]> {
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate("get_issuer_list", this.u32(start), this.u32(limit));
  }

  // ── Bridge Registry ────────────────────────────────────────────────────────

  /**
   * Checks if an address is registered as a bridge.
   * @param address - Stellar address to check.
   * @returns True if the address is a bridge, false otherwise.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async isBridge(address: string): Promise<boolean> {
    validateAddress(address);
    return this.simulate("is_bridge", this.addr(address));
  }

  /**
   * Returns a paginated list of bridge addresses.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of addresses to return.
   * @returns Array of bridge addresses.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getBridgeList(start: number, limit: number): Promise<string[]> {
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate("get_bridge_list", this.u32(start), this.u32(limit));
  }

  /**
   * Returns the pending admin transfer if one exists.
   * @returns Pending transfer details or null.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getPendingAdminTransfer(): Promise<{ proposed_by: string; new_admin: string } | null> {
    return this.simulate("get_pending_admin_transfer");
  }

  // ── Claim Type Registry ────────────────────────────────────────────────────

  /**
   * Returns the description for a claim type.
   * @param claimType - The claim type to look up.
   * @returns Description string or null if not found.
   * @throws {InvalidClaimTypeError} If the claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getClaimTypeDescription(claimType: string): Promise<string | null> {
    validateClaimType(claimType);
    return this.simulate("get_claim_type_description", this.str(claimType));
  }

  /**
   * Returns a paginated list of registered claim types.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of claim types to return.
   * @returns Array of claim type strings.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async listClaimTypes(start: number, limit: number): Promise<string[]> {
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate("list_claim_types", this.u32(start), this.u32(limit));
  }

  /**
   * Returns whether the given claim type is registered in the contract registry.
   * @param claimType - The claim type to check.
   * @returns True if registered, false otherwise.
   * @throws {InvalidClaimTypeError} If the claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getRegisteredClaimType(claimType: string): Promise<boolean> {
    validateClaimType(claimType);
    return this.simulate("get_registered_claim_type", this.str(claimType));
  }

  /**
   * Returns whether the contract requires claim types to be pre-registered.
   * When true, free-text claim types are rejected on attestation creation.
   * @returns True if registration required, false otherwise.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getRequireRegisteredClaimType(): Promise<boolean> {
    return this.simulate("get_require_registered_claim_type");
  }

  // ── Rate Limiting ──────────────────────────────────────────────────────────

  /**
   * Returns the per-claim-type rate limit configuration.
   * Distinct from getRateLimit(), which operates at the per-issuer level.
   * @param claimType - The claim type to look up.
   * @returns Rate limit as a bigint.
   * @throws {InvalidClaimTypeError} If the claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getRateLimitForClaimType(claimType: string): Promise<bigint> {
    validateClaimType(claimType);
    return this.simulate("get_rate_limit_for_claim_type", this.str(claimType));
  }

  // ── Delegation Queries ─────────────────────────────────────────────────────

  /**
   * Returns a delegation if one exists.
   * @param delegator - Stellar address of the delegator.
   * @param delegate - Stellar address of the delegate.
   * @param claimType - The claim type for the delegation.
   * @returns Delegation object or null if not found.
   * @throws {InvalidAddressError} If any address format is invalid.
   * @throws {InvalidClaimTypeError} If the claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getDelegation(
    delegator: string,
    delegate: string,
    claimType: string
  ): Promise<Delegation | null> {
    validateAddress(delegator);
    validateAddress(delegate);
    validateClaimType(claimType);
    return this.simulate(
      "get_delegation",
      this.addr(delegator),
      this.addr(delegate),
      this.str(claimType)
    );
  }

  /**
   * Returns a paginated list of delegations for a delegator.
   * @param delegator - Stellar address of the delegator.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of delegations to return.
   * @returns Array of delegation objects.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async listDelegationsByDelegator(delegator: string, start: number, limit: number): Promise<Delegation[]> {
    validateAddress(delegator);
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate("list_delegations_by_delegator", this.addr(delegator), this.u32(start), this.u32(limit));
  }

  // ── Attestation Queries ────────────────────────────────────────────────────

  /**
   * Returns an attestation by ID.
   * @param attestationId - The attestation ID.
   * @returns Attestation object.
   * @throws {TrustLinkValidationError} If the attestation ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the attestation is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getAttestation(attestationId: string): Promise<Attestation> {
    validateAttestationId(attestationId);
    return this.simulate("get_attestation", this.str(attestationId));
  }

  /**
   * Returns the status of an attestation.
   * @param attestationId - The attestation ID.
   * @returns Attestation status.
   * @throws {TrustLinkValidationError} If the attestation ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the attestation is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getAttestationStatus(attestationId: string): Promise<AttestationStatus> {
    validateAttestationId(attestationId);
    return this.simulate("get_attestation_status", this.str(attestationId));
  }

  /**
   * Returns an attestation for a subject by claim type.
   * @param subject - Stellar address of the subject.
   * @param claimType - The claim type.
   * @returns Attestation object.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidClaimTypeError} If the claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the attestation is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getAttestationByType(
    subject: string,
    claimType: string
  ): Promise<Attestation> {
    validateAddress(subject);
    validateClaimType(claimType);
    return this.simulate(
      "get_attestation_by_type",
      this.addr(subject),
      this.str(claimType)
    );
  }

  /**
   * Returns a paginated list of attestations for a subject.
   * @param subject - Stellar address of the subject.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of attestations to return.
   * @returns Array of attestation objects.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getSubjectAttestations(
    subject: string,
    start: number,
    limit: number
  ): Promise<Attestation[]> {
    validateAddress(subject);
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate(
      "get_subject_attestations",
      this.addr(subject),
      this.u32(start),
      this.u32(limit)
    );
  }

  /**
   * Returns a paginated list of attestations for an issuer.
   * @param issuer - Stellar address of the issuer.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of attestations to return.
   * @returns Array of attestation objects.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getIssuerAttestations(
    issuer: string,
    start: number,
    limit: number
  ): Promise<Attestation[]> {
    validateAddress(issuer);
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate(
      "get_issuer_attestations",
      this.addr(issuer),
      this.u32(start),
      this.u32(limit)
    );
  }

  /**
   * Returns attestation IDs for a subject filtered by tag.
   * @param subject - Stellar address of the subject.
   * @param tag - Tag to filter by.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of IDs to return.
   * @returns Array of attestation IDs.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getAttestationsByTag(subject: string, tag: string, start = 0, limit = 20): Promise<string[]> {
    validateAddress(subject);
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    const all = await this.simulate<string[]>(
      "get_attestations_by_tag",
      this.addr(subject),
      this.str(tag)
    );
    return all.slice(start, start + limit);
  }

  /**
   * Returns a paginated list of attestation IDs for a subject filtered by jurisdiction.
   * @param subject - Stellar address of the subject.
   * @param jurisdiction - Jurisdiction code to filter by (e.g. "US", "EU").
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of IDs to return.
   * @returns Array of attestation IDs.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getAttestationsByJurisdiction(
    subject: string,
    jurisdiction: string,
    start: number,
    limit: number
  ): Promise<string[]> {
    validateAddress(subject);
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate(
      "get_attestations_by_jurisdiction",
      this.addr(subject),
      this.str(jurisdiction),
      this.u32(start),
      this.u32(limit)
    );
  }

  /**
   * Returns valid claim types for a subject.
   * @param subject - Stellar address of the subject.
   * @returns Array of valid claim type strings.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getValidClaims(subject: string): Promise<string[]> {
    validateAddress(subject);
    return this.simulate("get_valid_claims", this.addr(subject));
  }

  /**
   * Returns the audit log for an attestation.
   * @param attestationId - The attestation ID.
   * @returns Array of audit entries.
   * @throws {TrustLinkValidationError} If the attestation ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the attestation is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getAuditLog(attestationId: string): Promise<AuditEntry[]> {
    validateAttestationId(attestationId);
    return this.simulate("get_audit_log", this.str(attestationId));
  }

  // ── Claim Verification ─────────────────────────────────────────────────────

  /**
   * Checks if a subject has a valid claim of a specific type.
   * @param subject - Stellar address of the subject.
   * @param claimType - The claim type to check.
   * @returns True if the subject has a valid claim, false otherwise.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidClaimTypeError} If the claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async hasValidClaim(subject: string, claimType: string): Promise<boolean> {
    validateAddress(subject);
    validateClaimType(claimType);
    return this.simulate(
      "has_valid_claim",
      this.addr(subject),
      this.str(claimType)
    );
  }

  /**
   * Checks if a subject has a valid claim from a specific issuer.
   * @param subject - Stellar address of the subject.
   * @param claimType - The claim type to check.
   * @param issuer - Stellar address of the issuer.
   * @returns True if the subject has a valid claim from the issuer, false otherwise.
   * @throws {InvalidAddressError} If any address format is invalid.
   * @throws {InvalidClaimTypeError} If the claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async hasValidClaimFromIssuer(
    subject: string,
    claimType: string,
    issuer: string
  ): Promise<boolean> {
    validateAddress(subject);
    validateClaimType(claimType);
    validateAddress(issuer);
    return this.simulate(
      "has_valid_claim_from_issuer",
      this.addr(subject),
      this.str(claimType),
      this.addr(issuer)
    );
  }

  /**
   * Checks if a subject has any of the specified claim types.
   * @param subject - Stellar address of the subject.
   * @param claimTypes - Array of claim types to check.
   * @returns True if the subject has any of the claims, false otherwise.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidClaimTypeError} If any claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async hasAnyClaim(subject: string, claimTypes: string[]): Promise<boolean> {
    validateAddress(subject);
    claimTypes.forEach(ct => validateClaimType(ct));
    return this.simulate(
      "has_any_claim",
      this.addr(subject),
      this.strVec(claimTypes)
    );
  }

  /**
   * Checks if a subject has all of the specified claim types.
   * @param subject - Stellar address of the subject.
   * @param claimTypes - Array of claim types to check.
   * @returns True if the subject has all of the claims, false otherwise.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidClaimTypeError} If any claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async hasAllClaims(subject: string, claimTypes: string[]): Promise<boolean> {
    validateAddress(subject);
    claimTypes.forEach(ct => validateClaimType(ct));
    return this.simulate(
      "has_all_claims",
      this.addr(subject),
      this.strVec(claimTypes)
    );
  }

  /**
   * Checks if a subject has a valid claim from issuers of at least a specified tier.
   * @param subject - Stellar address of the subject.
   * @param claimType - The claim type to check.
   * @param minTier - Minimum issuer tier required.
   * @returns True if the subject has a valid claim from the required tier, false otherwise.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidClaimTypeError} If the claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async hasValidClaimFromTier(
    subject: string,
    claimType: string,
    minTier: IssuerTier
  ): Promise<boolean> {
    validateAddress(subject);
    validateClaimType(claimType);
    // IssuerTier is a Soroban #[contracttype] enum — encode as ScVec([ScSymbol(variant)])
    return this.simulate(
      "has_valid_claim_from_tier",
      this.addr(subject),
      this.str(claimType),
      xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(minTier)])
    );
  }

  /**
   * Returns the total count of attestations for a claim type.
   * @param claimType - The claim type to count.
   * @returns Total count as a bigint.
   * @throws {InvalidClaimTypeError} If the claim type format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getClaimTypeCount(claimType: string): Promise<bigint> {
    validateClaimType(claimType);
    return this.simulate("get_claim_type_count", this.str(claimType));
  }

  // ── Count Queries ──────────────────────────────────────────────────────────

  /**
   * Returns the total number of attestations for a subject.
   * @param subject - Stellar address of the subject.
   * @returns Total count as a bigint.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getSubjectAttestationCount(subject: string): Promise<bigint> {
    validateAddress(subject);
    return this.simulate("get_subject_attestation_count", this.addr(subject));
  }

  /**
   * Returns the total number of attestations for an issuer.
   * @param issuer - Stellar address of the issuer.
   * @returns Total count as a bigint.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getIssuerAttestationCount(issuer: string): Promise<bigint> {
    validateAddress(issuer);
    return this.simulate("get_issuer_attestation_count", this.addr(issuer));
  }

  /**
   * Returns the total number of valid claims for a subject.
   * @param subject - Stellar address of the subject.
   * @returns Total count as a bigint.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getValidClaimCount(subject: string): Promise<bigint> {
    validateAddress(subject);
    return this.simulate("get_valid_claim_count", this.addr(subject));
  }

  // ── Expiring Attestations (Issue #604) ───────────────────────────────────────

  /**
   * Returns attestations expiring within `withinDays` days, sorted by expiration ascending.
   * @param subject - Stellar address of the subject.
   * @param withinDays - Number of days to look ahead for expiring attestations.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of results to return.
   * @returns Array of attestation objects.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If any numeric parameter is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getExpiringAttestations(
    subject: string,
    withinDays: number,
    start: number,
    limit: number
  ): Promise<Attestation[]> {
    validateAddress(subject);
    validateNonNegative(withinDays, "withinDays");
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate(
      "get_expiring_attestations",
      this.addr(subject),
      this.u32(withinDays),
      this.u32(start),
      this.u32(limit)
    );
  }

  /**
   * Returns issuer's attestations expiring within `daysWindow` days, sorted by expiration ascending.
   * @param issuer - Stellar address of the issuer.
   * @param daysWindow - Number of days to look ahead for expiring attestations.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of results to return.
   * @returns Array of attestation objects.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If any numeric parameter is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getIssuerExpiringAttestations(
    issuer: string,
    daysWindow: number,
    start: number,
    limit: number
  ): Promise<Attestation[]> {
    validateAddress(issuer);
    validateNonNegative(daysWindow, "daysWindow");
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate(
      "get_issuer_expiring_attestations",
      this.addr(issuer),
      this.u32(daysWindow),
      this.u32(start),
      this.u32(limit)
    );
  }

  // ── Multi-Sig Proposals ────────────────────────────────────────────────────

  /**
   * Returns a multi-sig proposal by ID.
   * @param proposalId - The proposal ID.
   * @returns Multi-sig proposal object.
   * @throws {TrustLinkValidationError} If the proposal ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the proposal is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getMultisigProposal(proposalId: string): Promise<MultiSigProposal> {
    validateProposalId(proposalId);
    return this.simulate("get_multisig_proposal", this.str(proposalId));
  }

  /**
   * Returns the multi-sig proposal TTL in days.
   * @returns TTL in days.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getMultisigTtl(): Promise<number> {
    return this.simulate("get_multisig_ttl");
  }

  /**
   * Returns a paginated list of open multi-sig proposals for a subject.
   * @param subject - Stellar address of the subject.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of proposals to return.
   * @returns Array of multi-sig proposal objects.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async listOpenProposals(
    subject: string,
    start: number,
    limit: number
  ): Promise<MultiSigProposal[]> {
    validateAddress(subject);
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate(
      "list_open_proposals",
      this.addr(subject),
      this.u32(start),
      this.u32(limit)
    );
  }

  /**
   * Cancels a multi-sig proposal.
   * @param proposer - Stellar address of the proposer.
   * @param proposalId - The proposal ID.
   * @returns Simulation result for the transaction.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {TrustLinkValidationError} If the proposal ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {UnauthorizedError} If the caller is not the proposer.
   * @throws {ProposalFinalizedError} If the proposal is already finalized.
   * @throws {ProposalExpiredError} If the proposal has expired.
   * @throws {TrustLinkError} For other contract errors.
   */
  async cancelMultisigProposal(
    proposer: string,
    proposalId: string
  ): Promise<SorobanRpc.Api.SimulateTransactionResponse> {
    validateAddress(proposer);
    validateProposalId(proposalId);
    const dummySource = proposer;
    const account = new Account(dummySource, "0");
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(
        this.contract.call(
          "cancel_multisig_proposal",
          this.addr(proposer),
          this.str(proposalId)
        )
      )
      .setTimeout(30)
      .build();
    return this.server.simulateTransaction(tx);
  }

  /**
   * Fulfills an attestation request.
   * @param issuer - Stellar address of the issuer.
   * @param requestId - The request ID.
   * @param expiration - Optional expiration timestamp.
   * @returns Simulation result for the transaction.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {TrustLinkValidationError} If the request ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {UnauthorizedError} If the caller is not the issuer.
   * @throws {NotFoundError} If the request is not found.
   * @throws {InvalidExpirationError} If the expiration is invalid.
   * @throws {TrustLinkError} For other contract errors.
   */
  async fulfillRequest(
    issuer: string,
    requestId: string,
    expiration?: bigint
  ): Promise<SorobanRpc.Api.SimulateTransactionResponse> {
    validateAddress(issuer);
    validateRequestId(requestId);
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

  /**
   * Rejects an attestation request.
   * @param issuer - Stellar address of the issuer.
   * @param requestId - The request ID.
   * @param reason - Optional rejection reason.
   * @returns Simulation result for the transaction.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {TrustLinkValidationError} If the request ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {UnauthorizedError} If the caller is not the issuer.
   * @throws {NotFoundError} If the request is not found.
   * @throws {ReasonTooLongError} If the reason is too long.
   * @throws {TrustLinkError} For other contract errors.
   */
  async rejectRequest(
    issuer: string,
    requestId: string,
    reason?: string
  ): Promise<SorobanRpc.Api.SimulateTransactionResponse> {
    validateAddress(issuer);
    validateRequestId(requestId);
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

  /**
   * Returns an attestation request by ID.
   * @param requestId - The request ID.
   * @returns Attestation request object.
   * @throws {TrustLinkValidationError} If the request ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the request is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getAttestationRequest(requestId: string): Promise<AttestationRequest> {
    validateRequestId(requestId);
    return this.simulate("get_attestation_request", this.str(requestId));
  }

  /**
   * Returns the raw low-level request state for a given request ID.
   * Distinct from getAttestationRequest(), which returns the high-level processed object.
   * @param requestId - The request ID.
   * @returns Attestation request object.
   * @throws {TrustLinkValidationError} If the request ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the request is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getRequest(requestId: string): Promise<AttestationRequest> {
    validateRequestId(requestId);
    return this.simulate("get_request", this.str(requestId));
  }

  // ── Endorsements ──────────────────────────────────────────────────────────

  /**
   * Returns endorsements for an attestation.
   * @param attestationId - The attestation ID.
   * @returns Array of endorsement objects.
   * @throws {TrustLinkValidationError} If the attestation ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the attestation is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getEndorsements(attestationId: string): Promise<Endorsement[]> {
    validateAttestationId(attestationId);
    return this.simulate("get_endorsements", this.str(attestationId));
  }

  /**
   * Returns the number of endorsements for an attestation.
   * @param attestationId - The attestation ID.
   * @returns Endorsement count.
   * @throws {TrustLinkValidationError} If the attestation ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the attestation is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getEndorsementCount(attestationId: string): Promise<number> {
    validateAttestationId(attestationId);
    return this.simulate("get_endorsement_count", this.str(attestationId));
  }

  /**
   * Returns a paginated list of endorsements by an endorser.
   * @param endorser - Stellar address of the endorser.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of endorsements to return.
   * @returns Array of endorsement objects.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async listEndorsementsByEndorser(endorser: string, start: number, limit: number): Promise<Endorsement[]> {
    validateAddress(endorser);
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate("list_endorsements_by_endorser", this.addr(endorser), this.u32(start), this.u32(limit));
  }

  // ── Templates ─────────────────────────────────────────────────────────────

  /**
   * Returns a template by ID.
   * @param issuer - Stellar address of the issuer.
   * @param templateId - The template ID.
   * @returns Attestation template object.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {TrustLinkValidationError} If the template ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {NotFoundError} If the template is not found.
   * @throws {TrustLinkError} For other contract errors.
   */
  async getTemplate(issuer: string, templateId: string): Promise<import("./types").AttestationTemplate> {
    validateAddress(issuer);
    validateTemplateId(templateId);
    return this.simulate("get_template", this.addr(issuer), this.str(templateId));
  }

  /**
   * Returns a paginated list of templates for an issuer.
   * @param issuer - Stellar address of the issuer.
   * @param start - Zero-based page offset.
   * @param limit - Maximum number of templates to return.
   * @returns Array of template objects.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If start or limit are invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async listTemplates(issuer: string, start: number, limit: number): Promise<Template[]> {
    validateAddress(issuer);
    validateNonNegative(start, "start");
    validatePositive(limit, "limit");
    return this.simulate("list_templates", this.addr(issuer), this.u32(start), this.u32(limit));
  }

  // ── Whitelist ──────────────────────────────────────────────────────────────

  /**
   * Bulk adds subjects to an issuer's whitelist.
   * @param issuer - Stellar address of the issuer.
   * @param subjects - Array of subject addresses to add.
   * @returns Simulation result for the transaction.
   * @throws {InvalidAddressError} If any address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {UnauthorizedError} If the caller is not the issuer.
   * @throws {LimitExceededError} If the whitelist limit would be exceeded.
   * @throws {TrustLinkError} For other contract errors.
   */
  async bulkAddToWhitelist(issuer: string, subjects: string[]): Promise<void> {
    validateAddress(issuer);
    subjects.forEach(s => validateAddress(s));
    const subjectsVal = xdr.ScVal.scvVec(subjects.map(s => this.addr(s)));
    return this.simulate("bulk_add_to_whitelist", this.addr(issuer), subjectsVal);
  }

  /**
   * Checks if a subject is whitelisted by an issuer.
   * @param issuer - Stellar address of the issuer.
   * @param subject - Stellar address of the subject.
   * @returns True if whitelisted, false otherwise.
   * @throws {InvalidAddressError} If any address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async isWhitelisted(issuer: string, subject: string): Promise<boolean> {
    validateAddress(issuer);
    validateAddress(subject);
    return this.simulate("is_whitelisted", this.addr(issuer), this.addr(subject));
  }

  /**
   * Checks if an issuer has whitelist enabled.
   * @param issuer - Stellar address of the issuer.
   * @returns True if whitelist is enabled, false otherwise.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async isWhitelistEnabled(issuer: string): Promise<boolean> {
    validateAddress(issuer);
    return this.simulate("is_whitelist_enabled", this.addr(issuer));
  }

  // ── Pagination Helpers ─────────────────────────────────────────────────────

  /**
   * Iterates through all attestations for a subject.
   * @param subject - Stellar address of the subject.
   * @param pageSize - Number of attestations to fetch per page.
   * @yields Attestation objects.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If pageSize is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async *iterateSubjectAttestations(
    subject: string,
    pageSize = 20
  ): AsyncGenerator<Attestation> {
    validateAddress(subject);
    validatePositive(pageSize, "pageSize");
    let start = 0;
    while (true) {
      const page = await this.getSubjectAttestations(subject, start, pageSize);
      yield* page;
      if (page.length < pageSize) break;
      start += page.length;
    }
  }

  /**
   * Iterates through all attestations for an issuer.
   * @param issuer - Stellar address of the issuer.
   * @param pageSize - Number of attestations to fetch per page.
   * @yields Attestation objects.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {InvalidNumericValueError} If pageSize is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async *iterateIssuerAttestations(
    issuer: string,
    pageSize = 20
  ): AsyncGenerator<Attestation> {
    validateAddress(issuer);
    validatePositive(pageSize, "pageSize");
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
   * Aggregates attestations, audit logs, endorsements, and optional request
   * history (pass known request IDs in options.requestIds — the contract does
   * not expose a per-subject request index).
   * @param subject - Stellar address of the subject.
   * @param options - Optional parameters including request IDs to include.
   * @returns Subject data export object.
   * @throws {InvalidAddressError} If the address format is invalid.
   * @throws {TrustLinkValidationError} If any request ID is invalid.
   * @throws {NotInitializedError} If the contract has not been initialized.
   * @throws {TrustLinkError} For other contract errors.
   */
  async exportSubjectData(
    subject: string,
    options?: { requestIds?: string[] }
  ): Promise<SubjectDataExport> {
    validateAddress(subject);
    if (options?.requestIds) {
      options.requestIds.forEach(id => validateRequestId(id));
    }
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
