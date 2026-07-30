//! Shared data types and error codes for TrustLink.

use soroban_sdk::{contracterror, contracttype, xdr::ToXdr, Address, Bytes, Env, String, Vec};

/// Default lifetime for a multi-sig proposal: 7 days in seconds.
pub const MULTISIG_PROPOSAL_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// Default lifetime for an attestation request: 7 days in seconds.
pub const ATTESTATION_REQUEST_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// Seconds in one day.
pub const SECS_PER_DAY: u64 = 86_400;

/// Status of an attestation request.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RequestStatus {
    Pending = 0,
    Fulfilled = 1,
    Rejected = 2,
    Cancelled = 3,
}

/// A pull-based attestation request submitted by a subject to a registered issuer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationRequest {
    pub id: String,
    pub subject: Address,
    pub issuer: Address,
    pub claim_type: String,
    pub timestamp: u64,
    pub expires_at: u64,
    pub status: RequestStatus,
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

/// A registered expiration notification hook for a subject.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpirationHook {
    pub callback_contract: Address,
    pub notify_days_before: u32,
}

/// A multi-signature attestation proposal.
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
    /// Set to true when the proposer cancels the proposal before finalization.
    pub cancelled: bool,
}

/// Contract metadata returned by `get_contract_metadata`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Metadata about a registered issuer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuerMetadata {
    pub name: String,
    pub url: String,
    pub description: String,
}

/// Fee configuration for attestation creation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub attestation_fee: i128,
    pub fee_collector: Address,
    pub fee_token: Option<Address>,
}

/// Global contract statistics.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalStats {
    pub total_attestations: u64,
    pub total_revocations: u64,
    pub total_issuers: u64,
}

/// Health status for monitoring.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthStatus {
    pub initialized: bool,
    pub admin_set: bool,
    pub issuer_count: u64,
    pub total_attestations: u64,
}

/// TTL configuration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TtlConfig {
    pub ttl_days: u32,
}

/// Rate limiting configuration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitConfig {
    pub min_issuance_interval: u64,
}

/// Contract configuration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractConfig {
    pub contract_name: String,
    pub contract_version: String,
    pub contract_description: String,
    pub fee_config: FeeConfig,
    pub ttl_config: TtlConfig,
    pub require_registered_claim_type: bool,
    /// When `true`, the `metadata` field on new attestations must be either
    /// `None` or a 64-character lowercase hexadecimal string (SHA-256 hash).
    /// Enables enforcement of GDPR data-minimisation at the contract level.
    pub metadata_hash_only: bool,
    /// Optional maximum number of attestations per subject.
    /// When set, new attestations exceeding this limit will be rejected.
    /// When `None`, attestations are unlimited (default for backward compatibility).
    pub max_attestations_per_subject: Option<u32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimTypeInfo {
    pub claim_type: String,
    pub description: String,
}

/// Constraints for a specific claim type enforced during attestation creation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimTypeConstraints {
    pub min_metadata_len: Option<u32>,
    pub max_metadata_len: Option<u32>,
    pub require_metadata: bool,
}

/// Operations that require council quorum approval.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CouncilOperation {
    RemoveIssuer(Address),
    PauseContract,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouncilProposal {
    pub id: u32,
    pub operation: CouncilOperation,
    pub proposer: Address,
    pub approvals: Vec<Address>,
    pub executed: bool,
    /// Ledger timestamp at which the proposal reached quorum.
    /// `None` means quorum has not been reached yet. Used by the timelock
    /// guard in `execute_council_action`.
    pub quorum_reached_at: Option<u64>,
}

/// Describes how an attestation entered the system.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationOrigin {
    Native,
    Imported,
    Bridged,
}

/// A single issuer-created claim about a subject address.
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
    pub metadata: Option<String>,
    pub valid_from: Option<u64>,
    pub origin: AttestationOrigin,
    pub source_chain: Option<String>,
    pub source_tx: Option<String>,
    pub tags: Option<Vec<String>>,
    pub revocation_reason: Option<String>,
    pub deleted: bool,
    /// Optional: shared bundle ID if this attestation was created as part of a bundle.
    /// Allows verifiers to confirm a set of claims were issued atomically.
    pub bundle_id: Option<String>,
}

/// Metadata for a bundle of attestations issued atomically.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationBundle {
    /// Unique bundle identifier (SHA256 of issuer + subject + claim_types + timestamp)
    pub id: String,
    /// Issuer who created the bundle
    pub issuer: Address,
    /// Subject to whom all attestations in the bundle were issued
    pub subject: Address,
    /// List of claim types in the bundle (fixed order for deterministic ID)
    pub claim_types: Vec<String>,
    /// Timestamp when the bundle was created
    pub timestamp: u64,
    /// IDs of all attestations in this bundle (in same order as claim_types)
    pub attestation_ids: Vec<String>,
    /// Whether all attestations in the bundle are still valid (none revoked)
    pub all_valid: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationStatus {
    Valid,
    Expired,
    Revoked,
    Pending,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditAction {
    Created,
    Revoked,
    Renewed,
    Updated,
    Transferred,
    Deleted,
    Amended,
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

/// Configurable storage limits to prevent exhaustion attacks.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageLimits {
    pub max_attestations_per_issuer: u32,
    pub max_attestations_per_subject: u32,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    NotFound = 4,
    DuplicateAttestation = 5,
    AlreadyRevoked = 6,
    InvalidValidFrom = 7,
    InvalidExpiration = 8,
    MetadataTooLong = 9,
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

/// Storage key for the pending admin transfer (two-step pattern).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAdminTransfer {
    pub proposed_by: Address,
    pub new_admin: Address,
}

/// Admin council: ordered list of admin addresses.
pub type AdminCouncil = Vec<Address>;

/// A point-in-time snapshot of an attestation's mutable fields, saved before
/// each amendment so callers can reconstruct the full version history.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationVersionSnapshot {
    pub version: u32,
    pub metadata: Option<String>,
    pub amended_at: u64,
    pub amended_by: Address,
}

/// Configurable parameters for issuer reputation decay, applied at read time
/// inside `get_confidence_score`. Stored on-chain so they are adjustable
/// without a contract upgrade.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecayConfig {
    /// Number of days of inactivity after which the score is halved.
    /// Set to 0 to disable inactivity decay entirely.
    pub half_life_days: u32,
    /// Scaling factor (0–100) applied to the revocation ratio before
    /// subtracting from the score. 100 means a 100 % revocation rate
    /// would zero out the score entirely.
    pub revocation_weight: u32,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            half_life_days: 90,
            revocation_weight: 50,
        }
    }
}

/// An active dispute raised by a subject against one of their attestations.
/// The record is removed when the dispute is resolved.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeRecord {
    pub attestation_id: String,
    pub subject: Address,
    pub reason: String,
    pub disputed_at: u64,
}

/// A named attestation template owned by an issuer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationTemplate {
    pub claim_type: String,
    pub metadata_template: Option<String>,
    pub default_expiration_days: Option<u32>,
}

impl Attestation {
    pub fn generate_id(
        env: &Env,
        issuer: &Address,
        subject: &Address,
        claim_type: &String,
        timestamp: u64,
    ) -> String {
        let mut bytes = Bytes::new(env);
        bytes.append(&issuer.clone().to_xdr(env));
        bytes.append(&subject.clone().to_xdr(env));
        bytes.append(&claim_type.clone().to_xdr(env));
        bytes.append(&Bytes::from_slice(env, &timestamp.to_be_bytes()));

        let hash = env.crypto().sha256(&bytes).to_array();
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut id = [0u8; 32];
        for i in 0..16 {
            id[i * 2] = HEX[(hash[i] >> 4) as usize];
            id[i * 2 + 1] = HEX[(hash[i] & 0x0f) as usize];
        }
        String::from_str(env, core::str::from_utf8(&id).unwrap_or(""))
    }

    /// Compute the current validity state. A pending attestation is not usable,
    /// and revocation permanently takes precedence over expiration.
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
