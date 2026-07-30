//! Shared data types and error codes for TrustLink.

use soroban_sdk::{contracterror, contracttype, xdr::ToXdr, Address, Bytes, Env, String};

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

/// Issuer statistics.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuerStats {
    pub total_issued: u64,
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
