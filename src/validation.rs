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
        // Copy bytes out of the host-side String for inspection.
        // len is u32 in Soroban SDK; safe to cast since we already checked <= 64.
        let mut buf = [0u8; 64];
        let slice = &mut buf[..len as usize];
        claim_type.copy_into_slice(slice);
        for &b in slice.iter() {
            let is_alpha = b.is_ascii_alphabetic();
            let is_digit = b.is_ascii_digit();
            let is_underscore = b == b'_';
            if !is_alpha && !is_digit && !is_underscore {
                return Err(Error::InvalidClaimType);
            }
        }
        Ok(())
    }
}
