/**
 * TypeScript types mirroring the TrustLink Soroban contract data structures.
 */

export interface Attestation {
  id: string;
  issuer: string;
  subject: string;
  claim_type: string;
  timestamp: bigint;
  expiration: bigint | null;
  revoked: boolean;
  metadata: string | null;
  valid_from: bigint | null;
  origin: AttestationOrigin;
  source_chain: string | null;
  source_tx: string | null;
  tags: string[] | null;
  revocation_reason: string | null;
  deleted: boolean;
}

export type AttestationOrigin = "Native" | "Imported" | "Bridged";

export type AttestationStatus = "Valid" | "Expired" | "Revoked" | "Pending";

export type IssuerTier = "Basic" | "Verified" | "Premium";

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
  fee_token: string | null;
}

export interface TtlConfig {
  ttl_days: number;
}

export interface ContractConfig {
  ttl_config: TtlConfig;
  fee_config: FeeConfig;
  contract_name: string;
  contract_version: string;
  contract_description: string;
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
  created_at: bigint;
  expires_at: bigint;
  finalized: boolean;
}

export interface Endorsement {
  attestation_id: string;
  endorser: string;
  timestamp: bigint;
}

export type AuditAction = "Created" | "Revoked" | "Renewed" | "Updated";

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

export type RequestStatus = "Pending" | "Fulfilled" | "Rejected";

export interface AttestationRequest {
  id: string;
  subject: string;
  issuer: string;
  claim_type: string;
  timestamp: bigint;
  expires_at: bigint;
  status: RequestStatus;
  rejection_reason: string | null;
}

/** Base class for all TrustLink contract errors. */
export class TrustLinkError extends Error {
  constructor(public readonly code: number, name: string, message?: string) {
    super(message ?? name);
    this.name = name;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

export class AlreadyInitializedError extends TrustLinkError {
  constructor() { super(1, "AlreadyInitialized"); }
}
export class NotInitializedError extends TrustLinkError {
  constructor() { super(2, "NotInitialized"); }
}
export class UnauthorizedError extends TrustLinkError {
  constructor() { super(3, "Unauthorized"); }
}
export class NotFoundError extends TrustLinkError {
  constructor() { super(4, "NotFound"); }
}
export class DuplicateAttestationError extends TrustLinkError {
  constructor() { super(5, "DuplicateAttestation"); }
}
export class AlreadyRevokedError extends TrustLinkError {
  constructor() { super(6, "AlreadyRevoked"); }
}
export class ExpiredError extends TrustLinkError {
  constructor() { super(7, "Expired"); }
}
export class InvalidValidFromError extends TrustLinkError {
  constructor() { super(8, "InvalidValidFrom"); }
}
export class InvalidExpirationError extends TrustLinkError {
  constructor() { super(9, "InvalidExpiration"); }
}
export class MetadataTooLongError extends TrustLinkError {
  constructor() { super(10, "MetadataTooLong"); }
}
export class InvalidTimestampError extends TrustLinkError {
  constructor() { super(11, "InvalidTimestamp"); }
}
export class InvalidFeeError extends TrustLinkError {
  constructor() { super(12, "InvalidFee"); }
}
export class FeeTokenRequiredError extends TrustLinkError {
  constructor() { super(13, "FeeTokenRequired"); }
}
export class TooManyTagsError extends TrustLinkError {
  constructor() { super(14, "TooManyTags"); }
}
export class TagTooLongError extends TrustLinkError {
  constructor() { super(15, "TagTooLong"); }
}
export class InvalidThresholdError extends TrustLinkError {
  constructor() { super(16, "InvalidThreshold"); }
}
export class NotRequiredSignerError extends TrustLinkError {
  constructor() { super(17, "NotRequiredSigner"); }
}
export class AlreadySignedError extends TrustLinkError {
  constructor() { super(18, "AlreadySigned"); }
}
export class ProposalFinalizedError extends TrustLinkError {
  constructor() { super(19, "ProposalFinalized"); }
}
export class ProposalExpiredError extends TrustLinkError {
  constructor() { super(20, "ProposalExpired"); }
}
export class ReasonTooLongError extends TrustLinkError {
  constructor() { super(21, "ReasonTooLong"); }
}
export class CannotEndorseOwnError extends TrustLinkError {
  constructor() { super(22, "CannotEndorseOwn"); }
}
export class AlreadyEndorsedError extends TrustLinkError {
  constructor() { super(23, "AlreadyEndorsed"); }
}
export class ContractPausedError extends TrustLinkError {
  constructor() { super(24, "ContractPaused"); }
}
export class LimitExceededError extends TrustLinkError {
  constructor() { super(25, "LimitExceeded"); }
}
export class SelfAttestationError extends TrustLinkError {
  constructor() { super(26, "SelfAttestation"); }
}
export class InvalidClaimTypeError extends TrustLinkError {
  constructor() { super(27, "InvalidClaimType"); }
}
export class RequestNotFoundError extends TrustLinkError {
  constructor() { super(28, "RequestNotFound"); }
}
export class RequestAlreadyFulfilledError extends TrustLinkError {
  constructor() { super(29, "RequestAlreadyFulfilled"); }
}

const ERROR_BY_CODE: Record<number, new () => TrustLinkError> = {
  1: AlreadyInitializedError,
  2: NotInitializedError,
  3: UnauthorizedError,
  4: NotFoundError,
  5: DuplicateAttestationError,
  6: AlreadyRevokedError,
  7: ExpiredError,
  8: InvalidValidFromError,
  9: InvalidExpirationError,
  10: MetadataTooLongError,
  11: InvalidTimestampError,
  12: InvalidFeeError,
  13: FeeTokenRequiredError,
  14: TooManyTagsError,
  15: TagTooLongError,
  16: InvalidThresholdError,
  17: NotRequiredSignerError,
  18: AlreadySignedError,
  19: ProposalFinalizedError,
  20: ProposalExpiredError,
  21: ReasonTooLongError,
  22: CannotEndorseOwnError,
  23: AlreadyEndorsedError,
  24: ContractPausedError,
  25: LimitExceededError,
  26: SelfAttestationError,
  27: InvalidClaimTypeError,
  28: RequestNotFoundError,
  29: RequestAlreadyFulfilledError,
};

const ERROR_BY_NAME: Record<string, new () => TrustLinkError> = Object.fromEntries(
  Object.values(ERROR_BY_CODE).map((Cls) => {
    const instance = new Cls();
    return [instance.name, Cls];
  })
);

/**
 * Parse a contract simulation error string into a typed TrustLinkError.
 * Returns null if the error string does not match a known contract error.
 */
export function parseTrustLinkError(errorMessage: string): TrustLinkError | null {
  // Soroban encodes contract errors as "Error(Contract, #N)" or includes the name
  const codeMatch = errorMessage.match(/Error\(Contract,\s*#(\d+)\)/);
  if (codeMatch) {
    const Cls = ERROR_BY_CODE[parseInt(codeMatch[1], 10)];
    if (Cls) return new Cls();
  }
  for (const [name, Cls] of Object.entries(ERROR_BY_NAME)) {
    if (errorMessage.includes(name)) return new Cls();
  }
  return null;
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
}
