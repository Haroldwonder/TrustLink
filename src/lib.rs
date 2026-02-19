#![no_std]

mod storage;
mod types;
mod validation;
mod events;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};
use types::{Attestation, AttestationStatus, Error};
use storage::Storage;
use validation::Validation;
use events::Events;

#[contract]
pub struct TrustLinkContract;

#[contractimpl]
impl TrustLinkContract {
    /// Initialize the contract with an admin address
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if Storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        
        admin.require_auth();
        Storage::set_admin(&env, &admin);
        Ok(())
    }

    /// Register a new authorized issuer (admin only)
    pub fn register_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        
        Storage::add_issuer(&env, &issuer);
        Ok(())
    }

    /// Remove an authorized issuer (admin only)
    pub fn remove_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        
        Storage::remove_issuer(&env, &issuer);
        Ok(())
    }

    /// Create a new attestation (authorized issuers only)
    pub fn create_attestation(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
    ) -> Result<String, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        
        let timestamp = env.ledger().timestamp();
        
        // Generate deterministic ID from attestation data
        let attestation_id = Attestation::generate_id(
            &env,
            &issuer,
            &subject,
            &claim_type,
            timestamp,
        );
        
        // Check for duplicates
        if Storage::has_attestation(&env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }
        
        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: issuer.clone(),
            subject: subject.clone(),
            claim_type: claim_type.clone(),
            timestamp,
            expiration,
            revoked: false,
        };
        
        Storage::set_attestation(&env, &attestation);
        Storage::add_subject_attestation(&env, &subject, &attestation_id);
        Storage::add_issuer_attestation(&env, &issuer, &attestation_id);
        
        Events::attestation_created(&env, &attestation);
        
        Ok(attestation_id)
    }

    /// Revoke an existing attestation (issuer only)
    pub fn revoke_attestation(
        env: Env,
        issuer: Address,
        attestation_id: String,
    ) -> Result<(), Error> {
        issuer.require_auth();
        
        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;
        
        // Only the original issuer can revoke
        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        
        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }
        
        attestation.revoked = true;
        Storage::set_attestation(&env, &attestation);
        
        Events::attestation_revoked(&env, &attestation_id, &issuer);
        
        Ok(())
    }

    /// Check if an address has a valid attestation of a given type
    pub fn has_valid_claim(
        env: Env,
        subject: Address,
        claim_type: String,
    ) -> bool {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        
        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if attestation.claim_type == claim_type {
                    let status = attestation.get_status(current_time);
                    if status == AttestationStatus::Valid {
                        return true;
                    }
                }
            }
        }
        
        false
    }

    /// Get a specific attestation by ID
    pub fn get_attestation(
        env: Env,
        attestation_id: String,
    ) -> Result<Attestation, Error> {
        Storage::get_attestation(&env, &attestation_id)
    }

    /// Get attestation status (valid, expired, or revoked)
    pub fn get_attestation_status(
        env: Env,
        attestation_id: String,
    ) -> Result<AttestationStatus, Error> {
        let attestation = Storage::get_attestation(&env, &attestation_id)?;
        let current_time = env.ledger().timestamp();
        Ok(attestation.get_status(current_time))
    }

    /// List attestations for a subject (paginated)
    pub fn get_subject_attestations(
        env: Env,
        subject: Address,
        start: u32,
        limit: u32,
    ) -> Vec<String> {
        let all_ids = Storage::get_subject_attestations(&env, &subject);
        let total = all_ids.len();
        
        let mut result = Vec::new(&env);
        let end = (start + limit).min(total);
        
        for i in start..end {
            if let Some(id) = all_ids.get(i) {
                result.push_back(id);
            }
        }
        
        result
    }

    /// List attestations issued by an issuer (paginated)
    pub fn get_issuer_attestations(
        env: Env,
        issuer: Address,
        start: u32,
        limit: u32,
    ) -> Vec<String> {
        let all_ids = Storage::get_issuer_attestations(&env, &issuer);
        let total = all_ids.len();
        
        let mut result = Vec::new(&env);
        let end = (start + limit).min(total);
        
        for i in start..end {
            if let Some(id) = all_ids.get(i) {
                result.push_back(id);
            }
        }
        
        result
    }

    /// Check if an address is an authorized issuer
    pub fn is_issuer(env: Env, address: Address) -> bool {
        Storage::is_issuer(&env, &address)
    }

    /// Get the admin address
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        Storage::get_admin(&env)
    }
}
