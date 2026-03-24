//! Shared data types and error definitions for TrustLink.

use soroban_sdk::{contracterror, contracttype, xdr::ToXdr, Address, Bytes, Env, String, Vec};

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

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
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
}

impl Attestation {
    fn hash_payload(env: &Env, payload: &Bytes) -> String {
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
