//! Shared data types, error codes, and core attestation logic for TrustLink.
//!
//! ## Types
//!
//! - [`Attestation`] — the primary on-chain record. Stores the issuer, subject,
//!   claim type, creation timestamp, optional expiration, optional `valid_from`,
//!   and revocation flag. Its [`Attestation::generate_id`] method produces a
//!   deterministic 32-character hex ID from a SHA-256 hash of the key fields,
//!   and [`Attestation::get_status`] computes the current [`AttestationStatus`]
//!   from the ledger timestamp.
//! - [`AttestationStatus`] — four-variant enum: `Pending`, `Valid`, `Expired`,
//!   `Revoked`. Priority order: Pending > Revoked > Expired > Valid.
//! - [`IssuerMetadata`] — optional public profile an issuer can attach to their
//!   address (name, URL, description).
//! - [`ClaimTypeInfo`] — a registered claim type identifier paired with a
//!   human-readable description.
//! - [`ContractMetadata`] — static contract info (name, version, description)
//!   returned by `get_contract_metadata`.
//!
//! ## Error codes
//!
//! [`Error`] is a `#[contracterror]` enum whose `u32` discriminants are the
//! values surfaced to callers as `Error(Contract, #N)`:
//!
//! | # | Variant                | When raised                                      |
//! |---|------------------------|--------------------------------------------------|
//! | 1 | `AlreadyInitialized`   | `initialize` called a second time                |
//! | 2 | `NotInitialized`       | Any call before `initialize`                     |
//! | 3 | `Unauthorized`         | Caller is not admin or not a registered issuer   |
//! | 4 | `NotFound`             | Attestation ID does not exist in storage         |
//! | 5 | `DuplicateAttestation` | ID collision (same inputs at same timestamp)     |
//! | 6 | `AlreadyRevoked`       | Attempt to revoke or renew an already-revoked attestation |
//! | 7 | `Expired`              | Reserved                                         |
//! | 8 | `InvalidValidFrom`     | `valid_from` ≤ current ledger timestamp          |
//! | 9 | `InvalidExpiration`    | New expiration ≤ current ledger timestamp        |

use soroban_sdk::{contracterror, contracttype, Address, Env, String};

/// Contract metadata returned by `get_contract_metadata`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// A registered claim type with its description.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimTypeInfo {
    pub claim_type: String,
    pub description: String,
}

/// A single attestation record stored on-chain.
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
    pub valid_from: Option<u64>,
}

/// Metadata an issuer can associate with their address.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssuerMetadata {
    pub name: String,
    pub url: String,
    pub description: String,
}

/// The current validity state of an attestation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationStatus {
    Valid,
    Expired,
    Revoked,
    Pending,
}

/// Errors returned by TrustLink contract functions.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
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
    SubjectNotWhitelisted = 25,
}

impl Attestation {
    /// Generate a deterministic attestation ID by SHA-256 hashing
    /// `(issuer, subject, claim_type, timestamp)` and hex-encoding the first
    /// 16 bytes of the digest into a 32-character ASCII string.
    pub fn generate_id(
        env: &Env,
        issuer: &Address,
        subject: &Address,
        claim_type: &String,
        timestamp: u64,
    ) -> String {
        use soroban_sdk::Bytes;
        let mut issuer_buf = [0u8; 56];
        let mut subject_buf = [0u8; 56];
        issuer.to_string().copy_into_slice(&mut issuer_buf);
        subject.to_string().copy_into_slice(&mut subject_buf);

        let claim_len = claim_type.len() as usize;
        let mut claim_buf = [0u8; 128];
        claim_type.copy_into_slice(&mut claim_buf[..claim_len]);

        let mut buf = Bytes::new(env);
        buf.append(&Bytes::from_slice(env, &issuer_buf));
        buf.append(&Bytes::from_slice(env, &subject_buf));
        buf.append(&Bytes::from_slice(env, &claim_buf[..claim_len]));
        buf.append(&Bytes::from_slice(env, &timestamp.to_be_bytes()));

        let hash = env.crypto().sha256(&buf);
        let hash_arr = hash.to_array();

        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut hex_bytes = [0u8; 32];
        for i in 0..16 {
            hex_bytes[i * 2]     = HEX[(hash_arr[i] >> 4) as usize];
            hex_bytes[i * 2 + 1] = HEX[(hash_arr[i] & 0x0f) as usize];
        }
        String::from_str(env, core::str::from_utf8(&hex_bytes).unwrap_or(""))
    }

    /// Compute the current [`AttestationStatus`] given `current_time`.
    ///
    /// Priority: Pending > Revoked > Expired > Valid.
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
}
