/**
 * TrustLink Contract — TypeScript type definitions
 *
 * Auto-generated from src/types.rs and src/errors.rs.
 * Do NOT edit by hand — run `make bindings` to regenerate.
 */

import { xdr } from "@stellar/stellar-sdk";

// ─── Enums ────────────────────────────────────────────────────────────────────

export enum AttestationStatus {
  Valid = "Valid",
  Expired = "Expired",
  Revoked = "Revoked",
  Pending = "Pending",
}

export enum AuditAction {
  Created = "Created",
  Revoked = "Revoked",
  Renewed = "Renewed",
  Updated = "Updated",
}

/** Trust tier assigned to a registered issuer. */
export enum IssuerTier {
  Basic = 0,
  Verified = 1,
  Premium = 2,
}

// ─── Structs ──────────────────────────────────────────────────────────────────

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
  imported: boolean;
  bridged: boolean;
  source_chain: string | null;
  source_tx: string | null;
  tags: string[] | null;
  revocation_reason: string | null;
  /** True when the subject has requested GDPR deletion. */
  deleted: boolean;
}

export interface AuditEntry {
  action: AuditAction;
  actor: string;
  timestamp: bigint;
  details: string | null;
}

export interface ClaimTypeInfo {
  claim_type: string;
  description: string;
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

export interface Endorsement {
  attestation_id: string;
  endorser: string;
  timestamp: bigint;
}

export interface ExpirationHook {
  callback_contract: string;
  notify_days_before: number;
}

export interface FeeConfig {
  attestation_fee: bigint;
  fee_collector: string;
  fee_token: string | null;
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

export interface IssuerMetadata {
  name: string;
  url: string;
  description: string;
}

export interface IssuerStats {
  total_issued: bigint;
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

export interface TtlConfig {
  ttl_days: number;
}

// ─── Errors ───────────────────────────────────────────────────────────────────

export enum ContractErrorCode {
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
}

export const CONTRACT_ERRORS: Record<number, string> = {
  1: "AlreadyInitialized",
  2: "NotInitialized",
  3: "Unauthorized",
  4: "NotFound",
  5: "DuplicateAttestation",
  6: "AlreadyRevoked",
  7: "Expired",
  8: "InvalidValidFrom",
  9: "InvalidExpiration",
  10: "MetadataTooLong",
  11: "InvalidTimestamp",
  12: "InvalidFee",
  13: "FeeTokenRequired",
  14: "TooManyTags",
  15: "TagTooLong",
  16: "InvalidThreshold",
  17: "NotRequiredSigner",
  18: "AlreadySigned",
  19: "ProposalFinalized",
  20: "ProposalExpired",
  21: "ReasonTooLong",
  22: "CannotEndorseOwn",
  23: "AlreadyEndorsed",
  24: "ContractPaused",
};

// ─── XDR helpers ──────────────────────────────────────────────────────────────

/** Decode a contract error code from an XDR ScVal. */
export function decodeContractError(scVal: xdr.ScVal): ContractErrorCode | undefined {
  if (scVal.switch() === xdr.ScValType.scvError()) {
    const err = scVal.error();
    if (err.switch() === xdr.ScErrorType.sceContract()) {
      return err.code() as ContractErrorCode;
    }
  }
  return undefined;
}
