//! Shared data types and error definitions for TrustLink.

use soroban_sdk::{contracterror, contracttype, xdr::ToXdr, Address, Bytes, Env, String, Vec};

/// Default lifetime for a multi-sig proposal: 7 days in seconds.
pub const MULTISIG_PROPOSAL_TTL_SECS: u64 = 7 * 24 * 60 * 60;

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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuerMetadata {
    pub name: String,
    pub url: String,
    pub description: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub attestation_fee: i128,
    pub fee_collector: Address,
    pub fee_token: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TtlConfig {
    pub ttl_days: u32,
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
    pub metadata: Option<String>,
    pub valid_from: Option<u64>,
    pub imported: bool,
    pub bridged: bool,
    pub source_chain: Option<String>,
    pub source_tx: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationStatus {
    Valid,
    Expired,
    Revoked,
    Pending,
}

/// A reusable per-issuer blueprint that captures default values for attestation creation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationTemplate {
    /// Non-empty claim type identifier (e.g. "KYC", "AML").
    pub claim_type: String,
    /// Optional default expiration window in days from attestation creation time.
    pub default_expiration_days: Option<u32>,
    /// Optional default metadata string (max 256 bytes).
    pub metadata_template: Option<String>,
}

/// A multi-sig attestation proposal that becomes active once `threshold` issuers have co-signed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigProposal {
    /// Unique proposal identifier (hash of proposer+subject+claim_type+timestamp).
    pub id: String,
    /// The issuer who created the proposal.
    pub proposer: Address,
    /// The subject the attestation is about.
    pub subject: Address,
    /// The claim type being attested.
    pub claim_type: String,
    /// All addresses that must co-sign (includes proposer).
    pub required_signers: Vec<Address>,
    /// Number of signers needed to activate the attestation.
    pub threshold: u32,
    /// Addresses that have already signed (proposer signs on creation).
    pub signers: Vec<Address>,
    /// Ledger timestamp when the proposal was created.
    pub created_at: u64,
    /// Ledger timestamp after which the proposal expires if not completed.
    pub expires_at: u64,
    /// Whether the proposal has been finalized into an active attestation.
    pub finalized: bool,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    /// Caller lacks required permissions. Includes rejection when `issuer` equals `subject` in `create_attestation`.
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
    /// Threshold must be >= 1 and <= number of required signers.
    InvalidThreshold = 16,
    /// The signer is not in the proposal's required_signers list.
    NotRequiredSigner = 17,
    /// The signer has already co-signed this proposal.
    AlreadySigned = 18,
    /// The proposal has already been finalized.
    ProposalFinalized = 19,
    /// The proposal has expired without reaching threshold.
    ProposalExpired = 20,
    /// claim_type field is empty.
    InvalidClaimType = 21,
}

/// A cryptographic proof that an attestation existed at a specific ledger sequence.
///
/// ## Verification
///
/// To verify this proof against Stellar ledger history:
///
/// 1. Fetch the ledger header for `ledger_sequence` from a Stellar Horizon node:
///    `GET /ledgers/{ledger_sequence}`
/// 2. Confirm the returned `hash` field matches `ledger_hash` in this struct.
/// 3. Confirm the returned `closed_at` Unix timestamp matches `ledger_timestamp`.
/// 4. Recompute the attestation ID from `attestation.issuer`, `attestation.subject`,
///    `attestation.claim_type`, and `attestation.timestamp` using the same SHA-256
///    hashing scheme used by `Attestation::generate_id`.
/// 5. Confirm the recomputed ID matches `attestation.id`.
///
/// A proof is considered valid when all three checks pass, establishing that the
/// attestation was stored on-chain no later than `ledger_sequence`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationProof {
    /// The full attestation record at the time the proof was generated.
    pub attestation: Attestation,
    /// The Stellar ledger sequence number at which the proof was captured.
    pub ledger_sequence: u32,
    /// The ledger close timestamp (Unix seconds) for `ledger_sequence`.
    pub ledger_timestamp: u64,
    /// The SHA-256 hash of the ledger header, hex-encoded (32 bytes → 64 hex chars).
    /// Use this to cross-reference against Stellar Horizon or a Stellar Core node.
    pub ledger_hash: String,
}

impl Attestation {
    pub fn hash_payload(env: &Env, payload: &Bytes) -> String {
        let hash = env.crypto().sha256(payload).to_array();
        const HEX: &[u8; 16] = b"0123456789abcdef";

        let mut hex = [0u8; 32];
        for i in 0..16 {
            hex[i * 2] = HEX[(hash[i] >> 4) as usize];
            hex[i * 2 + 1] = HEX[(hash[i] & 0x0f) as usize];
        }

        String::from_bytes(env, &hex)
    }

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

impl MultiSigProposal {
    /// Generate a deterministic proposal ID from proposer + subject + claim_type + timestamp.
    pub fn generate_id(
        env: &Env,
        proposer: &Address,
        subject: &Address,
        claim_type: &String,
        timestamp: u64,
    ) -> String {
        let mut payload = Bytes::new(env);
        // Prefix to distinguish from regular attestation IDs.
        payload.append(&Bytes::from_slice(env, b"multisig:"));
        payload.append(&proposer.clone().to_xdr(env));
        payload.append(&subject.clone().to_xdr(env));
        payload.append(&claim_type.clone().to_xdr(env));
        payload.append(&timestamp.to_xdr(env));
        Attestation::hash_payload(env, &payload)
    }
}
