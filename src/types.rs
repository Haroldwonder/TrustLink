//! Shared data types for TrustLink.
//!
//! Defines [`Attestation`], [`AttestationStatus`], and supporting structs used
//! throughout the contract. All types are annotated with `#[contracttype]` for
//! Soroban ABI compatibility. Error definitions live in [`crate::errors`].

use soroban_sdk::{contracttype, xdr::ToXdr, Address, Bytes, Env, String, Vec};

pub use crate::errors::Error;

/// Default lifetime for a multi-sig proposal: 7 days in seconds.
pub const MULTISIG_PROPOSAL_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// Default lifetime for an attestation request: 7 days in seconds.
pub const ATTESTATION_REQUEST_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// Seconds in one day.
pub const SECS_PER_DAY: u64 = 86_400;

/// Default TTL for persistent storage entries, in days.
pub const DEFAULT_TTL_DAYS: u32 = 30;

/// Number of ledgers per day on Stellar (one ledger every ~5 seconds).
pub const DAY_IN_LEDGERS: u32 = 17_280;

/// Minimum TTL threshold in ledgers before a TTL extension is triggered (7 days).
pub const MIN_TTL_THRESHOLD_LEDGERS: u32 = 7 * DAY_IN_LEDGERS;

/// Status of an attestation request.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RequestStatus {
    Pending = 0,
    Fulfilled = 1,
    Rejected = 2,
}

/// A pull-based attestation request submitted by a subject to a registered issuer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationRequest {
    /// Unique deterministic ID (hash of subject | issuer | claim_type | timestamp).
    pub id: String,
    pub subject: Address,
    pub issuer: Address,
    pub claim_type: String,
    pub timestamp: u64,
    /// Unix timestamp after which the request expires if not acted on.
    pub expires_at: u64,
    pub status: RequestStatus,
    /// Rejection reason set by the issuer, if rejected.
    pub rejection_reason: Option<String>,
}

/// Trust tier assigned to a registered issuer.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum IssuerTier {
    Basic = 0,
    Verified = 1,
    Premium = 2,
}

impl IssuerTier {
    pub fn rank(self) -> u32 {
        self as u32
    }
}

use soroban_sdk::{contracterror, contracttype, Address, Env, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimTypeInfo {
    pub claim_type: String,
    pub description: String,
}

/// The admin council configuration: member list and quorum threshold.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminCouncil {
    /// Addresses eligible to vote on council proposals.
    pub members: Vec<Address>,
    /// Minimum approvals required to execute a proposal.
    pub quorum: u32,
}

/// Operations that require council quorum approval.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CouncilOperation {
    RemoveIssuer(Address),
    PauseContract,
}

/// Describes how an attestation entered the system.
///
/// Replaces the previous `imported: bool` and `bridged: bool` fields, which
/// were mutually exclusive and left the "native" state implicit.
///
/// # Variants
/// - `Native`   — created directly by a registered issuer via `create_attestation`.
/// - `Imported` — migrated from an external verified source by the admin via `import_attestation`.
/// - `Bridged`  — mirrored from another chain by a trusted bridge contract via `bridge_attestation`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationOrigin {
    Native,
    Imported,
    Bridged,
/// Lightweight health status returned by `health_check`.
///
/// No authentication required — designed for monitoring dashboards and uptime probes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouncilProposal {
    pub id: u32,
    pub operation: CouncilOperation,
    pub proposer: Address,
    pub approvals: Vec<Address>,
    pub executed: bool,
}

/// A single attestation record stored on-chain.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationOrigin {
    Native,
    Imported,
    Bridged,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    pub id: String,
    pub issuer: Address,
    pub subject: Address,
    pub claim_type: String,
    pub timestamp: u64,
    pub expiration: Option<u64>,
    pub revoked: bool,
    /// Set to `true` by `request_deletion` (GDPR right-to-erasure soft delete).
    /// Deleted attestations are excluded from all query results.
    pub deleted: bool,
    pub metadata: Option<String>,
    pub jurisdiction: Option<String>,
    pub valid_from: Option<u64>,
    pub origin: AttestationOrigin,
    pub source_chain: Option<String>,
    pub source_tx: Option<String>,
    pub tags: Option<Vec<String>>,
    pub revocation_reason: Option<String>,
    /// True when the subject has requested GDPR deletion of this attestation.
    /// Deleted attestations are excluded from all query results.
    pub deleted: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationStatus {
    Valid,
    Expired,
    Revoked,
    Pending,
}

/// The action recorded in an audit log entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditAction {
    Created,
    Revoked,
    Renewed,
    Updated,
    Transferred,
}

/// A single immutable entry in an attestation's audit log.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntry {
    pub action: AuditAction,
    pub actor: Address,
    pub timestamp: u64,
    pub details: Option<String>,
}

/// A social-proof endorsement of an existing attestation by a registered issuer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Endorsement {
    pub attestation_id: String,
    pub endorser: Address,
    pub timestamp: u64,
}

/// A multi-signature attestation proposal requiring threshold signatures.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigProposal {
    pub id: String,
    pub proposer: Address,
    pub subject: Address,
    pub claim_type: String,
    pub required_signers: Vec<Address>,
    pub threshold: u32,
    pub signers: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    pub finalized: bool,
}

/// Configurable storage limits to prevent exhaustion attacks.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageLimits {
    /// Maximum number of attestations a single issuer may create. Default: 10,000.
    pub max_attestations_per_issuer: u32,
    /// Maximum number of attestations a single subject may hold. Default: 100.
    pub max_attestations_per_subject: u32,
}

impl Default for StorageLimits {
    fn default() -> Self {
        Self {
            max_attestations_per_issuer: 10_000,
            max_attestations_per_subject: 100,
        }
    }
}

/// Delegation from an issuer to a sub-issuer for specific claim types.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Delegation {
    pub delegator: Address,
    pub delegate: Address,
    pub claim_type: String,
    pub expiration: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigProposal {
    pub id: String,
    pub proposer: Address,
    pub subject: Address,
    pub claim_type: String,
    pub required_signers: Vec<Address>,
    pub threshold: u32,
    pub signers: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    pub finalized: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigProposal {
    pub id: String,
    pub proposer: Address,
    pub subject: Address,
    pub claim_type: String,
    pub required_signers: Vec<Address>,
    pub threshold: u32,
    pub signers: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    pub finalized: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigProposal {
    pub id: String,
    pub proposer: Address,
    pub subject: Address,
    pub claim_type: String,
    pub required_signers: Vec<Address>,
    pub threshold: u32,
    pub signers: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    pub finalized: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigProposal {
    pub id: String,
    pub proposer: Address,
    pub subject: Address,
    pub claim_type: String,
    pub required_signers: Vec<Address>,
    pub threshold: u32,
    pub signers: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    pub finalized: bool,
}

pub type AdminCouncil = Vec<Address>;

/// Default TTL for a council quorum proposal: 7 days in seconds.
pub const COUNCIL_PROPOSAL_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// The sensitive admin action being proposed for council quorum approval.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CouncilAction {
    /// Pause the contract.
    Pause,
    /// Unpause the contract.
    Unpause,
    /// Update the attestation fee configuration.
    SetFee(FeeConfig),
    /// Remove a registered issuer.
    RemoveIssuer(Address),
}

/// A pending council quorum proposal for a sensitive admin action.
///
/// The action is only executed once `approvals.len() >= threshold`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouncilProposal {
    /// Unique deterministic ID.
    pub id: String,
    /// The action being proposed.
    pub action: CouncilAction,
    /// Admin who created the proposal.
    pub proposer: Address,
    /// Admins who have approved (proposer is auto-included).
    pub approvals: Vec<Address>,
    /// Number of approvals required to execute.
    pub threshold: u32,
    /// Unix timestamp after which the proposal expires.
    pub expires_at: u64,
    /// Whether the proposal has been executed.
    pub executed: bool,
}

impl Attestation {
    /// Hashes an arbitrary byte payload and returns a 32-character lowercase hex string.
    ///
    /// Algorithm: SHA-256 over the XDR-encoded payload, digest truncated to the first 16 bytes,
    /// hex-encoded to a 32-character lowercase string.
    pub fn hash_payload(env: &Env, payload: &Bytes) -> String {
        let hash = env.crypto().sha256(payload).to_array();
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut hex = [0u8; 64];
        for i in 0..32 {
            hex[i * 2] = HEX[(hash[i] >> 4) as usize];
            hex[i * 2 + 1] = HEX[(hash[i] & 0x0f) as usize];
        }
        String::from_bytes(env, &hex)
    }

    /// Generates a deterministic attestation ID from the given inputs.
    ///
    /// XDR field order: `issuer | subject | claim_type | timestamp`
    pub fn generate_id(
        env: &Env,
        issuer: &Address,
        subject: &Address,
        claim_type: &String,
        timestamp: u64,
    ) -> String {
        let mut payload = Bytes::new(env);
        payload.append(&issuer.clone().to_xdr(env));
        payload.append(&subject.clone().to_xdr(env));
        payload.append(&claim_type.clone().to_xdr(env));
        payload.append(&timestamp.to_xdr(env));
        Self::hash_payload(env, &payload)
    }

    /// Generates a deterministic bridge attestation ID from the given inputs.
    ///
    /// XDR field order: `bridge | subject | claim_type | source_chain | source_tx | timestamp`
    pub fn generate_bridge_id(
        env: &Env,
        bridge: &Address,
        subject: &Address,
        claim_type: &String,
        source_chain: &String,
        source_tx: &String,
        timestamp: u64,
    ) -> String {
        let mut payload = Bytes::new(env);
        payload.append(&bridge.clone().to_xdr(env));
        payload.append(&subject.clone().to_xdr(env));
        payload.append(&claim_type.clone().to_xdr(env));
        payload.append(&source_chain.clone().to_xdr(env));
        payload.append(&source_tx.clone().to_xdr(env));
        payload.append(&timestamp.to_xdr(env));
        Self::hash_payload(env, &payload)
    }

    pub fn get_status(&self, current_time: u64) -> AttestationStatus {
        if let Some(valid_from) = self.valid_from {
            if current_time < valid_from {
                return AttestationStatus::Pending;
            }
        }
        if self.revoked {
            return AttestationStatus::Revoked;
        }
        if let Some(expiration) = self.expiration {
            if current_time >= expiration {
                return AttestationStatus::Expired;
            }
        }
        AttestationStatus::Valid
    }
}

impl AttestationRequest {
    /// Deterministic ID: SHA-256 over XDR of `"req:" | subject | issuer | claim_type | timestamp`.
    pub fn generate_id(
        env: &Env,
        subject: &Address,
        issuer: &Address,
        claim_type: &String,
        timestamp: u64,
    ) -> String {
        let mut payload = Bytes::new(env);
        payload.append(&Bytes::from_slice(env, b"req:"));
        payload.append(&subject.clone().to_xdr(env));
        payload.append(&issuer.clone().to_xdr(env));
        payload.append(&claim_type.clone().to_xdr(env));
        payload.append(&timestamp.to_xdr(env));
        Attestation::hash_payload(env, &payload)
    }
}

/// A multi-sig attestation proposal requiring M-of-N issuer signatures.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigProposal {
    pub id: String,
    pub proposer: Address,
    pub subject: Address,
    pub claim_type: String,
    pub required_signers: Vec<Address>,
    pub threshold: u32,
    pub signers: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    pub finalized: bool,
}

impl MultiSigProposal {
    pub fn generate_id(
        env: &Env,
        proposer: &Address,
        subject: &Address,
        claim_type: &String,
        timestamp: u64,
    ) -> String {
        let mut payload = Bytes::new(env);
        payload.append(&Bytes::from_slice(env, b"multisig:"));
        payload.append(&proposer.clone().to_xdr(env));
        payload.append(&subject.clone().to_xdr(env));
        payload.append(&claim_type.clone().to_xdr(env));
        payload.append(&timestamp.to_xdr(env));
        Attestation::hash_payload(env, &payload)
    }
}
