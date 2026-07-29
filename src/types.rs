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

/// A registered claim type and its human-readable description.
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
}

/// Public metadata associated with an issuer.
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

/// The current validity state of an attestation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageLimits {
    pub max_attestations_per_issuer: u32,
    pub max_attestations_per_subject: u32,
}

/// Errors returned by TrustLink contract functions.
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
    /// Generate a compact deterministic identifier for an attestation.
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
