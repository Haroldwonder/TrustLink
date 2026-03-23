#![no_std]

//! # TrustLink
//!
//! An on-chain attestation and verification system for the Stellar blockchain.
//!
//! Trusted issuers register with an admin, then create signed attestations about
//! wallet addresses. Any contract or dApp can query TrustLink to verify claims
//! before executing financial operations.

mod storage;
pub mod types;
mod validation;
mod events;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};
use types::{Attestation, AttestationStatus, ContractMetadata, Error};
use storage::Storage;
use validation::Validation;
use events::Events;

/// The TrustLink smart contract.
///
/// Provides a shared attestation infrastructure: admins manage a registry of
/// trusted issuers, issuers create and revoke attestations, and any caller can
/// verify claims against the registry.
#[contract]
pub struct TrustLinkContract;

#[contractimpl]
impl TrustLinkContract {
    /// Initialize the contract and set the administrator.
    ///
    /// Must be called exactly once after deployment. The `admin` address
    /// must authorize this call.
    ///
    /// # Parameters
    /// - `admin` — address that will control issuer registration.
    ///
    /// # Errors
    /// - [`Error::AlreadyInitialized`] — contract has already been initialized.
    ///
    /// # Examples
    /// ```ignore
    /// client.initialize(&admin_address);
    /// ```
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if Storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();
        Storage::set_admin(&env, &admin);
        Storage::set_version(&env, &String::from_str(&env, "1.0.0"));
        Ok(())
    }

    /// Register an address as an authorized attestation issuer.
    ///
    /// Only the current admin may call this function.
    ///
    /// # Parameters
    /// - `admin` — current administrator address (must authorize).
    /// - `issuer` — address to grant issuer privileges.
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] — contract has not been initialized.
    /// - [`Error::Unauthorized`] — `admin` is not the registered administrator.
    ///
    /// # Examples
    /// ```ignore
    /// client.register_issuer(&admin, &issuer_address);
    /// ```
    pub fn register_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;

        Storage::add_issuer(&env, &issuer);
        Ok(())
    }
    /// Return a deduplicated list of valid claim types for a subject.
    ///
    /// Iterates all attestations for `subject` and collects claim types whose
    /// status is [`AttestationStatus::Valid`]. Revoked and expired attestations
    /// are silently skipped. Duplicate claim types (e.g. two valid KYC_PASSED
    /// attestations) appear only once in the result.
    ///
    /// # Parameters
    /// - `subject` — address to query.
    ///
    /// # Returns
    /// A [`Vec<String>`] of unique valid claim type strings. Empty if the
    /// subject has no valid attestations.
    ///
    /// # Examples
    /// ```ignore
    /// let claims = client.get_valid_claims(&user_address);
    /// // e.g. ["KYC_PASSED", "ACCREDITED_INVESTOR"]
    /// ```
    pub fn get_valid_claims(env: Env, subject: Address) -> Vec<String> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        let mut result: Vec<String> = Vec::new(&env);

        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if attestation.get_status(current_time) == AttestationStatus::Valid {
                    // Deduplicate: only add if not already present
                    let mut already_present = false;
                    for existing in result.iter() {
                        if existing == attestation.claim_type {
                            already_present = true;
                            break;
                        }
                    }
                    if !already_present {
                        result.push_back(attestation.claim_type);
                    }
                }
            }
        }

        result
    }



    /// Remove an address from the authorized issuer registry.
    ///
    /// Only the current admin may call this function. Removing an issuer does
    /// not revoke attestations they have already created.
    ///
    /// # Parameters
    /// - `admin` — current administrator address (must authorize).
    /// - `issuer` — address to revoke issuer privileges from.
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] — contract has not been initialized.
    /// - [`Error::Unauthorized`] — `admin` is not the registered administrator.
    ///
    /// # Examples
    /// ```ignore
    /// client.remove_issuer(&admin, &issuer_address);
    /// ```
    pub fn remove_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;

        Storage::remove_issuer(&env, &issuer);
        Ok(())
    }

    /// Create a new attestation about a subject address.
    ///
    /// The attestation ID is derived deterministically from `(issuer, subject,
    /// claim_type, timestamp)`, so the same combination at the same ledger
    /// timestamp will always produce the same ID.
    ///
    /// Emits an [`events::Events::attestation_created`] event on success.
    ///
    /// # Parameters
    /// - `issuer` — authorized issuer creating the attestation (must authorize).
    /// - `subject` — address the attestation is about.
    /// - `claim_type` — free-form claim label, e.g. `"KYC_PASSED"`.
    /// - `expiration` — optional Unix timestamp (seconds) after which the
    ///   attestation is considered expired. Pass `None` for no expiration.
    ///
    /// # Returns
    /// The deterministic attestation ID as a [`String`].
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    /// - [`Error::DuplicateAttestation`] — an attestation with the same ID
    ///   already exists (same issuer/subject/claim_type/timestamp).
    ///
    /// # Examples
    /// ```ignore
    /// // No expiration
    /// let id = client.create_attestation(&issuer, &user, &String::from_str(&env, "KYC_PASSED"), &None);
    ///
    /// // Expires in one year
    /// let exp = env.ledger().timestamp() + 365 * 24 * 3600;
    /// let id = client.create_attestation(&issuer, &user, &String::from_str(&env, "ACCREDITED_INVESTOR"), &Some(exp));
    /// ```
    pub fn create_attestation(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        valid_from: Option<u64>,
    ) -> Result<String, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;

        let timestamp = env.ledger().timestamp();
        
        if let Some(vf) = valid_from {
            if vf <= timestamp {
                return Err(Error::InvalidValidFrom);
            }
        }
        
        // Generate deterministic ID from attestation data

        let attestation_id = Attestation::generate_id(
            &env,
            &issuer,
            &subject,
            &claim_type,
            timestamp,
        );

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
            valid_from,
        };

        Storage::set_attestation(&env, &attestation);
        Storage::add_subject_attestation(&env, &subject, &attestation_id);
        Storage::add_issuer_attestation(&env, &issuer, &attestation_id);

        Events::attestation_created(&env, &attestation);

        Ok(attestation_id)
    }

    /// Revoke an existing attestation.
    ///
    /// Only the original issuer of the attestation may revoke it. Revocation is
    /// permanent — the attestation record is kept but marked as revoked.
    ///
    /// Emits an [`events::Events::attestation_revoked`] event on success.
    ///
    /// # Parameters
    /// - `issuer` — the issuer who created the attestation (must authorize).
    /// - `attestation_id` — ID of the attestation to revoke.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — no attestation exists with the given ID.
    /// - [`Error::Unauthorized`] — caller is not the original issuer.
    /// - [`Error::AlreadyRevoked`] — attestation has already been revoked.
    ///
    /// # Examples
    /// ```ignore
    /// client.revoke_attestation(&issuer, &attestation_id);
    /// ```
    pub fn revoke_attestation(
        env: Env,
        issuer: Address,
        attestation_id: String,
    ) -> Result<(), Error> {
        issuer.require_auth();

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;

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

    /// Renew an existing attestation with a new expiration (issuer only)
    pub fn renew_attestation(
        env: Env,
        issuer: Address,
        attestation_id: String,
        new_expiration: Option<u64>,
    ) -> Result<(), Error> {
        issuer.require_auth();

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;

        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }

        Validation::require_issuer(&env, &issuer)?;

        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        if let Some(t) = new_expiration {
            if t <= env.ledger().timestamp() {
                return Err(Error::InvalidExpiration);
            }
        }

        attestation.expiration = new_expiration;
        Storage::set_attestation(&env, &attestation);
        Events::attestation_renewed(&env, &attestation_id, &issuer, new_expiration);

        Ok(())
    }

    /// Check if an address has a valid attestation of a given type
    /// Revoke multiple attestations in a single call (issuer only).
    /// Auth is checked once for the issuer. Each attestation is validated
    /// individually — if any attestation does not belong to the caller or is
    /// already revoked the corresponding error is returned immediately and no
    /// further attestations are processed.
    /// Returns the count of successfully revoked attestations.
    pub fn revoke_attestations_batch(
        env: Env,
        issuer: Address,
        attestation_ids: Vec<String>,
    ) -> Result<u32, Error> {
        // Single auth check for the entire batch
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;

        let mut count: u32 = 0;

        for id in attestation_ids.iter() {
            let mut attestation = Storage::get_attestation(&env, &id)?;

            if attestation.issuer != issuer {
                return Err(Error::Unauthorized);
            }

            if attestation.revoked {
                return Err(Error::AlreadyRevoked);
            }

            attestation.revoked = true;
            Storage::set_attestation(&env, &attestation);
            Events::attestation_revoked(&env, &id, &issuer);

            count += 1;
        }

        Ok(count)
    }

    /// Check if an address has a valid attestation of a given type.
    /// Emits an `expired` event for any expired (non-revoked) attestation encountered.
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
                    match attestation.get_status(current_time) {
                        AttestationStatus::Valid => return true,
                        AttestationStatus::Expired => {
                            Events::attestation_expired(&env, &id, &subject);
                        }
                        AttestationStatus::Revoked => {}
                    }
                }
            }
        }

        false
    }

    /// Fetch the full attestation record by ID.
    ///
    /// # Parameters
    /// - `attestation_id` — the attestation ID returned by [`create_attestation`].
    ///
    /// # Returns
    /// The [`Attestation`] struct containing all fields.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — no attestation exists with the given ID.
    ///
    /// # Examples
    /// ```ignore
    /// let attestation = client.get_attestation(&id);
    /// assert_eq!(attestation.claim_type, String::from_str(&env, "KYC_PASSED"));
    /// ```
    pub fn get_attestation(
        env: Env,
        attestation_id: String,
    ) -> Result<Attestation, Error> {
        Storage::get_attestation(&env, &attestation_id)
    }

    /// Return the current status of an attestation.
    ///
    /// Emits an [`events::Events::attestation_expired`] event when the status
    /// is [`AttestationStatus::Expired`]. No event is emitted for revoked
    /// attestations (revocation takes precedence over expiration).
    ///
    /// # Parameters
    /// - `attestation_id` — the attestation ID to query.
    ///
    /// # Returns
    /// - [`AttestationStatus::Valid`] — active and not expired.
    /// - [`AttestationStatus::Expired`] — past its expiration timestamp.
    /// - [`AttestationStatus::Revoked`] — explicitly revoked by the issuer.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — no attestation exists with the given ID.
    ///
    /// # Examples
    /// ```ignore
    /// match client.get_attestation_status(&id) {
    ///     AttestationStatus::Valid   => { /* proceed */ }
    ///     AttestationStatus::Expired => { /* re-issue */ }
    ///     AttestationStatus::Revoked => { /* deny */ }
    /// }
    /// ```
    pub fn get_attestation_status(
        env: Env,
        attestation_id: String,
    ) -> Result<AttestationStatus, Error> {
        let attestation = Storage::get_attestation(&env, &attestation_id)?;
        let current_time = env.ledger().timestamp();
        let status = attestation.get_status(current_time);
        if status == AttestationStatus::Expired {
            Events::attestation_expired(&env, &attestation_id, &attestation.subject);
        }
        Ok(status)
    }

    /// Return a paginated list of attestation IDs for a subject.
    ///
    /// # Parameters
    /// - `subject` — address whose attestations to list.
    /// - `start` — zero-based index of the first item to return.
    /// - `limit` — maximum number of items to return.
    ///
    /// # Returns
    /// A [`Vec<String>`] of attestation IDs. May be shorter than `limit` if
    /// fewer attestations exist beyond `start`.
    ///
    /// # Examples
    /// ```ignore
    /// let page1 = client.get_subject_attestations(&user, &0, &10);
    /// let page2 = client.get_subject_attestations(&user, &10, &10);
    /// ```
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

    /// Return a paginated list of attestation IDs created by an issuer.
    ///
    /// # Parameters
    /// - `issuer` — issuer address whose attestations to list.
    /// - `start` — zero-based index of the first item to return.
    /// - `limit` — maximum number of items to return.
    ///
    /// # Returns
    /// A [`Vec<String>`] of attestation IDs. May be shorter than `limit` if
    /// fewer attestations exist beyond `start`.
    ///
    /// # Examples
    /// ```ignore
    /// let issued = client.get_issuer_attestations(&issuer, &0, &50);
    /// ```
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

    /// Return a deduplicated list of valid claim types for a subject.
    ///
    /// Iterates all attestations for `subject` and collects claim types whose
    /// status is [`AttestationStatus::Valid`]. Revoked and expired attestations
    /// are silently skipped. Duplicate claim types appear only once in the result.
    ///
    /// # Parameters
    /// - `subject` — address to query.
    ///
    /// # Returns
    /// A [`Vec<String>`] of unique valid claim type strings. Empty if the
    /// subject has no valid attestations.
    pub fn get_valid_claims(env: Env, subject: Address) -> Vec<String> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        let mut result: Vec<String> = Vec::new(&env);

        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if attestation.get_status(current_time) == AttestationStatus::Valid {
                    let mut already_present = false;
                    for existing in result.iter() {
                        if existing == attestation.claim_type {
                            already_present = true;
                            break;
                        }
                    }
                    if !already_present {
                        result.push_back(attestation.claim_type);
                    }
                }
            }
        }

        result
    }

    /// Check whether an address is a registered issuer.
    ///
    /// # Parameters
    /// - `address` — address to check.
    ///
    /// # Returns
    /// `true` if the address is in the issuer registry, `false` otherwise.
    ///
    /// # Examples
    /// ```ignore
    /// assert!(client.is_issuer(&issuer_address));
    /// ```
    pub fn is_issuer(env: Env, address: Address) -> bool {
        Storage::is_issuer(&env, &address)
    }

    /// Find the most recent valid attestation for a subject by claim type.
    /// Iterates the subject's attestations in reverse (most recent first) and
    /// returns the first one that is neither revoked nor expired.
    /// Returns Error::NotFound if no valid attestation exists.
    pub fn get_attestation_by_type(
        env: Env,
        subject: Address,
        claim_type: String,
    ) -> Result<Attestation, Error> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        let len = attestation_ids.len();

        // Iterate in reverse so the most recently added attestation is checked first
        let mut i = len;
        while i > 0 {
            i -= 1;
            if let Some(id) = attestation_ids.get(i) {
                if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                    if attestation.claim_type == claim_type
                        && attestation.get_status(current_time) == AttestationStatus::Valid
                    {
                        return Ok(attestation);
                    }
                }
            }
        }

        Err(Error::NotFound)
    }

    /// Get the admin address
    /// Return the current administrator address.
    ///
    /// # Returns
    /// The admin [`Address`] set during [`initialize`].
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] — contract has not been initialized.
    ///
    /// # Examples
    /// ```ignore
    /// let admin = client.get_admin();
    /// ```
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        Storage::get_admin(&env)
    }

    /// Register a known claim type with a human-readable description (admin only).
    ///
    /// Pre-registers standard types on first deployment. Re-registering an
    /// existing claim type updates its description.
    ///
    /// Emits a `clmtype` event on success.
    ///
    /// # Parameters
    /// - `admin` — current administrator address (must authorize).
    /// - `claim_type` — identifier string, e.g. `"KYC_PASSED"`.
    /// - `description` — human-readable description of the claim type.
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] — contract has not been initialized.
    /// - [`Error::Unauthorized`] — `admin` is not the registered administrator.
    pub fn register_claim_type(
        env: Env,
        admin: Address,
        claim_type: String,
        description: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;

        let info = ClaimTypeInfo { claim_type: claim_type.clone(), description: description.clone() };
        Storage::set_claim_type(&env, &info);
        Events::claim_type_registered(&env, &claim_type, &description);
        Ok(())
    }

    /// Return the description for a registered claim type, or `None` if unknown.
    ///
    /// # Parameters
    /// - `claim_type` — identifier to look up.
    pub fn get_claim_type_description(env: Env, claim_type: String) -> Option<String> {
        Storage::get_claim_type(&env, &claim_type).map(|info| info.description)
    }

    /// Return a paginated list of registered claim type identifiers.
    ///
    /// # Parameters
    /// - `start` — zero-based index of the first item to return.
    /// - `limit` — maximum number of items to return.
    ///
    /// # Returns
    /// A [`Vec<String>`] of claim type strings in registration order.
    pub fn list_claim_types(env: Env, start: u32, limit: u32) -> Vec<String> {
        let all = Storage::get_claim_type_list(&env);
        let total = all.len();
        let mut result = Vec::new(&env);
        let end = (start + limit).min(total);
        for i in start..end {
            if let Some(ct) = all.get(i) {
                result.push_back(ct);
            }
        }
        result
    }

    /// Update the expiration of an existing attestation.
    ///
    /// Only the original issuer may update the expiration. The attestation must
    /// not be revoked. The expiration can be extended, shortened, or removed
    /// entirely by passing `None`.
    ///
    /// Emits an `attestation_updated` event on success.
    ///
    /// # Parameters
    /// - `issuer` — the issuer who created the attestation (must authorize).
    /// - `attestation_id` — ID of the attestation to update.
    /// - `new_expiration` — new expiration timestamp, or `None` to remove expiration.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — no attestation exists with the given ID.
    /// - [`Error::Unauthorized`] — caller is not the original issuer.
    /// - [`Error::AlreadyRevoked`] — attestation has already been revoked.
    pub fn update_expiration(
        env: Env,
        issuer: Address,
        attestation_id: String,
        new_expiration: Option<u64>,
    ) -> Result<(), Error> {
        issuer.require_auth();

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;

        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }

        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        attestation.expiration = new_expiration;
        Storage::set_attestation(&env, &attestation);

        Events::attestation_updated(&env, &attestation_id, &issuer, new_expiration);

        Ok(())
    }

    /// Return the semver version string set at initialization (e.g. `"1.0.0"`).
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] — contract has not been initialized.
    pub fn get_version(env: Env) -> Result<String, Error> {
        Storage::get_version(&env).ok_or(Error::NotInitialized)
    }

    /// Return static metadata about this contract.
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] — contract has not been initialized.
    pub fn get_contract_metadata(env: Env) -> Result<ContractMetadata, Error> {
        let version = Storage::get_version(&env).ok_or(Error::NotInitialized)?;
        Ok(ContractMetadata {
            name: String::from_str(&env, "TrustLink"),
            version,
            description: String::from_str(
                &env,
                "On-chain attestation and verification system for the Stellar blockchain.",
            ),
        })
    }
}
