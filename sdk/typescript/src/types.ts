/** Attestation recorded on-chain. */
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

export interface IssuerMetadata {
  name: string;
  url: string;
  description: string;
}

export interface FeeConfig {
  attestation_fee: bigint;
  fee_collector: string;
  fee_token?: string;
}

export interface ContractMetadata {
  name: string;
  version: string;
  description: string;
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
