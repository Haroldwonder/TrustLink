use soroban_sdk::{Address, Env};
use crate::storage::Storage;
use crate::types::Error;

pub struct Validation;

impl Validation {
    /// Verify that the caller is the admin
    pub fn require_admin(env: &Env, caller: &Address) -> Result<(), Error> {
        let admin = Storage::get_admin(env)?;
        if caller != &admin {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }
    
    /// Verify that the caller is an authorized issuer
    pub fn require_issuer(env: &Env, caller: &Address) -> Result<(), Error> {
        if !Storage::is_issuer(env, caller) {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }
}
