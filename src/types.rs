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

/// Activity metrics for a registered issuer.
///
/// Updated atomically alongside every `create_attestation` and
/// `revoke_attestation` call, so the counters are always consistent with
/// on-chain state.
///
/// ## Trustworthiness signals
///
/// - A high `total_revoked / total_issued` ratio may indicate an issuer that
///   frequently issues incorrect or fraudulent attestations.
/// - `registered_at` lets consumers weight newer issuers differently from
///   long-standing ones.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuerStats {
    /// Total number of attestations ever created by this issuer (includes
    /// batch, bridge, and multi-sig activations).
    pub total_issued: u64,
    /// Total number of attestations revoked by this issuer.
    pub total_revoked: u64,
    /// Ledger timestamp at which the issuer was first registered.
    pub registered_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractConfig {
    pub ttl_config: TtlConfig,
    pub fee_config: FeeConfig,
    pub contract_name: String,
    pub contract_version: String,
    pub contract_description: String,
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

/// A social-proof endorsement of an existing attestation by a registered issuer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Endorsement {
    /// The ID of the attestation being endorsed.
    pub attestation_id: String,
    /// The issuer who is endorsing the attestation.
    pub endorser: Address,
    /// Ledger timestamp when the endorsement was recorded.
    pub timestamp: u64,
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
    /// The endorser has already endorsed this attestation.
    AlreadyEndorsed = 21,
    /// An issuer cannot endorse their own attestation.
    CannotEndorseOwn = 22,
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
