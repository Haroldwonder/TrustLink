//! Authorization helpers for TrustLink.
//!
//! This module centralizes all permission checks so that contract entry points
//! stay focused on business logic. Every guard returns `Result<(), Error>` and
//! is called with the `?` operator, short-circuiting on the first failure.
//!
//! ## Guards
//!
//! - [`Validation::require_admin`] — verifies the caller matches the stored
//!   admin address. Returns [`Error::NotInitialized`] if the contract has not
//!   been set up yet, or [`Error::Unauthorized`] if the addresses differ.
//! - [`Validation::require_issuer`] — verifies the caller is present in the
//!   issuer registry. Returns [`Error::Unauthorized`] if not registered.
//! - [`Validation::require_bridge`] — verifies the caller is present in the
//!   bridge registry. Returns [`Error::Unauthorized`] if not registered.

use crate::storage::Storage;
use crate::types::Error;
use soroban_sdk::{Address, Env, String};

// ─────────────────────────────────────────────────────────────────────────────
// Shared byte-copy helper
// ─────────────────────────────────────────────────────────────────────────────

/// Copies the bytes of a Soroban SDK `String` into a fixed 64-byte stack
/// buffer and returns both the buffer and the number of bytes written.
///
/// This single helper eliminates the duplicate hand-written buffer-copy logic
/// that previously appeared in both `validate_claim_type` and
/// `validate_metadata_hash_only`. Both validators now call this function,
/// ensuring consistent behaviour if the buffer size or bounds-check ever
/// needs to change.
///
/// # Panics
/// Panics (via an out-of-bounds slice) if `s.len() > 64`.
/// All callers **must** validate length ≤ 64 before invoking this helper.
fn copy_into_fixed_buffer(s: &String) -> ([u8; 64], usize) {
    let len = s.len() as usize;
    let mut buf = [0u8; 64];
    s.copy_into_slice(&mut buf[..len]);
    (buf, len)
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation guards
// ─────────────────────────────────────────────────────────────────────────────

/// Authorization checks used by contract entry points.
pub struct Validation;

impl Validation {
    /// Assert that `caller` is in the admin council.
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] — council not initialized.
    /// - [`Error::Unauthorized`] — `caller` not in council.
    pub fn require_admin(env: &Env, caller: &Address) -> Result<(), Error> {
        // Return NotInitialized if the council has never been set up.
        let council = Storage::get_admin_council(env)?;
        let mut found = false;
        for admin in council.iter() {
            if &admin == caller {
                found = true;
                break;
            }
        }
        if !found {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    /// Assert that `caller` is a registered issuer.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `caller` is not in the issuer registry.
    pub fn require_issuer(env: &Env, caller: &Address) -> Result<(), Error> {
        if !Storage::is_issuer(env, caller) {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    /// Assert that `caller` is a registered bridge contract.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `caller` is not in the bridge registry.
    pub fn require_bridge(env: &Env, caller: &Address) -> Result<(), Error> {
        if !Storage::is_bridge(env, caller) {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    /// Assert that `caller` is either a registered issuer or a registered bridge contract.
    ///
    /// Used by attestation creation paths that accept both issuers and bridges,
    /// eliminating the duplicated `require_issuer` / `require_bridge` pattern.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `caller` is neither a registered issuer nor a registered bridge.
    pub fn require_authorized_creator(env: &Env, caller: &Address) -> Result<(), Error> {
        if Storage::is_issuer(env, caller) || Storage::is_bridge(env, caller) {
            return Ok(());
        }
        Err(Error::Unauthorized)
    }

    /// Assert that the contract is not currently paused.
    ///
    /// # Errors
    /// - [`Error::ContractPaused`] — the contract has been paused by the admin.
    pub fn require_not_paused(env: &Env) -> Result<(), Error> {
        if Storage::is_paused(env) {
            return Err(Error::ContractPaused);
        }
        Ok(())
    }

    /// Validate a `claim_type` string.
    ///
    /// # Rules
    /// - Maximum 64 characters.
    /// - Only ASCII alphanumeric characters (`A-Z`, `a-z`, `0-9`) and underscores (`_`) are allowed.
    ///
    /// # Errors
    /// - [`Error::InvalidClaimType`] — length exceeds 64 or contains disallowed characters.
    pub fn validate_claim_type(claim_type: &String) -> Result<(), Error> {
        let len = claim_type.len();
        if len == 0 || len > 64 {
            return Err(Error::InvalidClaimType);
        }
        // Use the shared helper; length is already verified <= 64 above.
        let (buf, byte_len) = copy_into_fixed_buffer(claim_type);
        for &b in buf[..byte_len].iter() {
            let is_alpha = b.is_ascii_alphabetic();
            let is_digit = b.is_ascii_digit();
            let is_underscore = b == b'_';
            if !is_alpha && !is_digit && !is_underscore {
                return Err(Error::InvalidClaimType);
            }
        }
        Ok(())
    }

    /// Validate optional metadata string.
    ///
    /// # Rules
    /// - Maximum 256 characters.
    ///
    /// # Errors
    /// - [`Error::MetadataTooLong`] — metadata exceeds 256 characters.
    pub fn validate_metadata(_env: &Env, metadata: &Option<String>) -> Result<(), Error> {
        if let Some(value) = metadata {
            if value.len() > 256 {
                return Err(Error::MetadataTooLong);
            }
        }
        Ok(())
    }

    /// When `metadata_hash_only` mode is enabled in `ContractConfig`, enforce
    /// that the metadata value is a 64-character lowercase hexadecimal string
    /// (a SHA-256 hash). `None` is always accepted.
    ///
    /// # Errors
    /// - [`Error::InvalidMetadata`] — metadata is present but is not a valid
    ///   64-char hex hash while hash-only mode is active.
    pub fn validate_metadata_hash_only(env: &Env, metadata: &Option<String>) -> Result<(), Error> {
        let Some(value) = metadata else {
            return Ok(());
        };
        if let Some(config) = Storage::get_contract_config(env) {
            if config.metadata_hash_only {
                if value.len() != 64 {
                    return Err(Error::InvalidMetadata);
                }
                // Use the shared helper; length is exactly 64 verified above.
                let (buf, _) = copy_into_fixed_buffer(value);
                for &b in buf.iter() {
                    if !matches!(b, b'0'..=b'9' | b'a'..=b'f') {
                        return Err(Error::InvalidMetadata);
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if a claim type is registered when required by contract config.
    ///
    /// # Errors
    /// - [`Error::InvalidClaimType`] — claim type is not registered and contract requires registration.
    pub fn require_registered_claim_type(env: &Env, claim_type: &String) -> Result<(), Error> {
        if let Some(config) = Storage::get_contract_config(env) {
            if config.require_registered_claim_type {
                if Storage::get_claim_type(env, claim_type).is_none() {
                    return Err(Error::InvalidClaimType);
                }
            }
        }
        Ok(())
    }

    /// Validate that an attestation satisfies claim type constraints.
    ///
    /// # Errors
    /// - [`Error::ConstraintViolation`] — metadata does not satisfy constraints.
    pub fn validate_claim_constraints(
        env: &Env,
        claim_type: &String,
        metadata: &Option<String>,
    ) -> Result<(), Error> {
        if let Some(constraints) = Storage::get_claim_type_constraints(env, claim_type) {
            if constraints.require_metadata && metadata.is_none() {
                return Err(Error::ConstraintViolation);
            }
            if let Some(meta) = metadata {
                let len = meta.len();
                if let Some(min) = constraints.min_metadata_len {
                    if len < min as usize {
                        return Err(Error::ConstraintViolation);
                    }
                }
                if let Some(max) = constraints.max_metadata_len {
                    if len > max as usize {
                        return Err(Error::ConstraintViolation);
                    }
                }
            }
        }
        Ok(())
    }
}
