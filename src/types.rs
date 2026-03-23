use soroban_sdk::{contracttype, contracterror, Address, Env, String};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    NotFound = 4,
    AlreadyRevoked = 5,
    DuplicateAttestation = 6,
    InvalidValidFrom = 7,
    InvalidExpiration = 8,
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
#[derive(Clone, Debug)]
pub struct Attestation {
    pub id: String,
    pub issuer: Address,
    pub subject: Address,
    pub claim_type: String,
    pub timestamp: u64,
    pub expiration: Option<u64>,
    pub revoked: bool,
    pub valid_from: Option<u64>,
}

impl Attestation {
    pub fn get_status(&self, current_time: u64) -> AttestationStatus {
        if let Some(vf) = self.valid_from {
            if current_time < vf {
                return AttestationStatus::Pending;
            }
        }
        if self.revoked {
            return AttestationStatus::Revoked;
        }
        if let Some(exp) = self.expiration {
            if current_time >= exp {
                return AttestationStatus::Expired;
            }
        }
        AttestationStatus::Valid
    }

use soroban_sdk::{contracterror, contracttype, Address, Env, String};

/// Contract metadata returned by `get_contract_metadata`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMetadata {
    /// Contract name.
    pub name: String,
    /// Semver version string, e.g. `"1.0.0"`.
    pub version: String,
    /// Short description of the contract.
    pub description: String,
}

/// A single attestation record stored on-chain.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    /// Deterministic hash-based identifier for this attestation.
    pub id: String,
    /// Address that created the attestation.
    pub issuer: Address,
    /// Address the attestation is about.
    pub subject: Address,
    /// Free-form claim label, e.g. `"KYC_PASSED"`.
    pub claim_type: String,
    /// Ledger timestamp (seconds) when the attestation was created.
    pub timestamp: u64,
    /// Optional Unix timestamp after which the attestation is expired.
    pub expiration: Option<u64>,
    /// `true` if the issuer has explicitly revoked this attestation.
    pub revoked: bool,
}

/// The current validity state of an attestation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationStatus {
    /// Attestation is active and has not expired.
    Valid,
    /// Attestation has passed its expiration timestamp.
    Expired,
    /// Attestation was explicitly revoked by its issuer.
    Revoked,
}

/// Errors returned by TrustLink contract functions.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// [`initialize`](crate::TrustLinkContract::initialize) was called more than once.
    AlreadyInitialized = 1,
    /// A function was called before [`initialize`](crate::TrustLinkContract::initialize).
    NotInitialized = 2,
    /// The caller lacks the required admin or issuer role.
    Unauthorized = 3,
    /// No attestation exists with the requested ID.
    NotFound = 4,
    /// An attestation with the same deterministic ID already exists.
    DuplicateAttestation = 5,
    /// The attestation has already been revoked.
    AlreadyRevoked = 6,
    /// The attestation has passed its expiration timestamp.
    Expired = 7,
}

impl Attestation {
    /// Generate a deterministic attestation ID by SHA-256 hashing the tuple
    /// `(issuer, subject, claim_type, timestamp)`.
    ///
    /// The first 16 bytes of the hash are used as the ID to keep it compact
    /// while still being collision-resistant for practical purposes.
    ///
    /// # Parameters
    /// - `issuer` — issuer address.
    /// - `subject` — subject address.
    /// - `claim_type` — claim label string.
    /// - `timestamp` — ledger timestamp at creation time.
    ///
    /// # Returns
    /// A [`String`] containing the raw 16-byte ID.
    pub fn generate_id(
        env: &Env,
        issuer: &Address,
        subject: &Address,
        claim_type: &String,
        timestamp: u64,
    ) -> String {
        use soroban_sdk::xdr::ToXdr;
        use soroban_sdk::Bytes;

        let mut data = Bytes::new(env);

        let issuer_xdr = issuer.clone().to_xdr(env);
        data.append(&issuer_xdr);

        let subject_xdr = subject.clone().to_xdr(env);
        data.append(&subject_xdr);

        let claim_bytes = claim_type.clone().to_xdr(env);
        data.append(&claim_bytes);

        let ts_bytes = timestamp.to_be_bytes();
        data.append(&Bytes::from_array(env, &ts_bytes));

        let hash = env.crypto().sha256(&data);
        let hex_chars: &[u8] = b"0123456789abcdef";
        let hash_bytes = hash.to_array();

        let mut arr = [0u8; 64];
        for (i, byte) in hash_bytes.iter().enumerate() {
            arr[i * 2] = hex_chars[(byte >> 4) as usize];
            arr[i * 2 + 1] = hex_chars[(byte & 0xf) as usize];
        }
        String::from_bytes(env, &arr)
        let data = (issuer.clone(), subject.clone(), claim_type.clone(), timestamp);
        let xdr_bytes = data.to_xdr(env);
        let hash = env.crypto().sha256(&xdr_bytes);
        let hash_bytes = hash.to_array();
        // Use first 16 bytes as ID
        String::from_bytes(env, &hash_bytes[..16])
    }

    /// Compute the current [`AttestationStatus`] given `current_time`.
    ///
    /// Revocation takes precedence: a revoked attestation always returns
    /// [`AttestationStatus::Revoked`] regardless of its expiration field.
    ///
    /// # Parameters
    /// - `current_time` — current ledger timestamp in seconds.
    pub fn get_status(&self, current_time: u64) -> AttestationStatus {
        if self.revoked {
            return AttestationStatus::Revoked;
        }
        if let Some(exp) = self.expiration {
            if current_time > exp {
                return AttestationStatus::Expired;
            }
        }
        AttestationStatus::Valid
    }
}
