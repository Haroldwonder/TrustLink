/** Attestation recorded on-chain. */
/**
 * TypeScript types mirroring the TrustLink Soroban contract data structures.
 */

export interface Attestation {
  id: string;
  issuer: string;
  subject: string;
  claim_type: string;
  timestamp: number;
  expiration?: number;
  revoked: boolean;
  metadata?: string;
  valid_from?: number;
  imported: boolean;
  bridged: boolean;
  source_chain?: string;
  source_tx?: string;
  tags?: string[];
}

export type AttestationStatus = "Valid" | "Expired" | "Revoked" | "Pending";

  timestamp: bigint;
  expiration: bigint | null;
  revoked: boolean;
  metadata: string | null;
  valid_from: bigint | null;
  origin: AttestationOrigin;
  source_chain: string | null;
  source_tx: string | null;
  tags: string[] | null;
  jurisdiction: string | null;
  revocation_reason: string | null;
  deleted: boolean;
}

export type AttestationOrigin = "Native" | "Imported" | "Bridged";

export type AttestationStatus = "Valid" | "Expired" | "Revoked" | "Pending";

export type IssuerTier = "Basic" | "Verified" | "Premium";

export interface Delegation {
  delegator: string;
  delegate: string;
  claim_type: string;
  expiration: bigint | null;
}

export interface IssuerStats {
  total_issued: bigint;
}

export interface IssuerMetadata {
  name: string;
  url: string;
  description: string;
}

export interface FeeConfig {
  attestation_fee: bigint;
  fee_collector: string;
  fee_token?: string;
  fee_token: string | null;
}

export interface TtlConfig {
  ttl_days: number;
}

export interface StorageLimits {
  max_attestations_per_issuer: number;
  max_attestations_per_subject: number;
}

export interface ContractConfig {
  ttl_config: TtlConfig;
  limits: StorageLimits;
  fee_config: FeeConfig;
  contract_name: string;
  contract_version: string;
  contract_description: string;
  multisig_ttl_days: number;
}

export interface ContractMetadata {
  name: string;
  version: string;
  description: string;
}

export interface ClaimTypeInfo {
  claim_type: string;
  description: string;
}

export interface GlobalStats {
  total_attestations: bigint;
  total_revocations: bigint;
  total_issuers: bigint;
}

export interface HealthStatus {
  initialized: boolean;
  admin_set: boolean;
  issuer_count: bigint;
  total_attestations: bigint;
}

export interface MultiSigProposal {
  id: string;
  proposer: string;
  subject: string;
  claim_type: string;
  required_signers: string[];
  threshold: number;
  signers: string[];
  created_at: number;
  expires_at: number;
  finalized: boolean;
}

/** Admin-council workflow types (issues #742). */
export interface Council {
  members: string[];
  threshold: number;
  created_at: number;
}

export interface CouncilProposal {
  id: string;
  proposer: string;
  action: string;
  payload: string;
  approvals: string[];
  threshold: number;
  created_at: number;
  expires_at: number;
  executed: boolean;
}

/** Storage limits for the contract (issue #743). */
export interface StorageLimits {
  max_attestations_per_subject: number;
  max_attestations_per_issuer: number;
  max_tags_per_attestation: number;
  max_tag_length: number;
  max_metadata_length: number;
}

export type TrustLinkError =
  | "AlreadyInitialized"
  | "NotInitialized"
  | "Unauthorized"
  | "NotFound"
  | "DuplicateAttestation"
  | "AlreadyRevoked"
  | "Expired"
  | "InvalidExpiration"
  | "InvalidTimestamp"
  | "InvalidFee"
  | "FeeTokenRequired"
  | "TooManyTags"
  | "TagTooLong"
  | "MetadataTooLong"
  | "InvalidThreshold"
  | "NotRequiredSigner"
  | "AlreadySigned"
  | "ProposalFinalized"
  | "ProposalExpired";
  created_at: bigint;
  expires_at: bigint;
  finalized: boolean;
  cancelled: boolean;
}

export interface Endorsement {
  attestation_id: string;
  endorser: string;
  timestamp: bigint;
}

export interface Delegation {
  delegator: string;
  delegate: string;
  claim_types: string[] | null;
  expires_at: bigint | null;
}

export interface Template {
  id: string;
  issuer: string;
  name: string;
  claim_type: string;
  description: string | null;
}

export type AuditAction = "Created" | "Revoked" | "Renewed" | "Updated" | "Transferred";

export interface AuditEntry {
  action: AuditAction;
  actor: string;
  timestamp: bigint;
  details: string | null;
}

export interface ExpirationHook {
  callback_contract: string;
  notify_days_before: number;
}

/** Error codes returned by the TrustLink contract. */
export enum TrustLinkError {
  AlreadyInitialized = 1,
  NotInitialized = 2,
  Unauthorized = 3,
  NotFound = 4,
  DuplicateAttestation = 5,
  AlreadyRevoked = 6,
  Expired = 7,
  InvalidValidFrom = 8,
  InvalidExpiration = 9,
  MetadataTooLong = 10,
  InvalidTimestamp = 11,
  InvalidFee = 12,
  FeeTokenRequired = 13,
  TooManyTags = 14,
  TagTooLong = 15,
  InvalidThreshold = 16,
  NotRequiredSigner = 17,
  AlreadySigned = 18,
  ProposalFinalized = 19,
  ProposalExpired = 20,
  ReasonTooLong = 21,
  CannotEndorseOwn = 22,
  AlreadyEndorsed = 23,
  ContractPaused = 24,
  SubjectNotWhitelisted = 25,
  InvalidClaimType = 26,
  InvalidJurisdiction = 27,
  RateLimited = 28,
  LimitExceeded = 29,
  ProposalCancelled = 30,
}

/** Network presets supported by TrustLinkClient. */
export type Network = "testnet" | "mainnet" | "local";

export interface TrustLinkClientOptions {
  /** Deployed TrustLink contract address (C...). */
  contractId: string;
  /** Network to connect to, or a custom RPC URL string. */
  network: Network | string;
  /** Optional: override the default RPC URL for the chosen network. */
  rpcUrl?: string;
  /** Optional: retry configuration for RPC calls. */
  retry?: import("./resilience").RetryOptions;
  /** Optional: circuit breaker configuration. */
  circuitBreaker?: import("./resilience").CircuitBreakerOptions;
  /** Optional: simplified resilience config (maxRetries, backoffMs, circuitBreakerThreshold). */
  resilience?: import("./resilience").ResilienceConfig;
}
