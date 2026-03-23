//! Authorization helpers for TrustLink.

use soroban_sdk::{Address, Env};
use crate::storage::Storage;
use crate::types::Error;

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
}
