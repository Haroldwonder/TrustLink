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
use soroban_sdk::{Address, Env};

/// Authorization checks used by contract entry points.
pub struct Validation;

impl Validation {
    /// Assert that `caller` is the registered administrator.
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] — contract has not been initialized.
    /// - [`Error::Unauthorized`] — `caller` does not match the stored admin.
    pub fn require_admin(env: &Env, caller: &Address) -> Result<(), Error> {
        let admin = Storage::get_admin(env)?;
        if caller != &admin {
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
}
