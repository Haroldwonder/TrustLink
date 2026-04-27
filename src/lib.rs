#![no_std]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod admin;
mod attestation;
mod errors;
mod events;
mod multisig;
mod query;
mod request;
mod storage;
mod constants;
pub mod types;
mod validation;

#[cfg(test)]
mod test;

pub(crate) mod callback {
    use soroban_sdk::{contractclient, Address, Env, String};
    #[contractclient(name = "ExpirationCallbackClient")]
    #[allow(dead_code)]
    pub trait ExpirationCallback {
        fn notify_expiring(env: Env, subject: Address, attestation_id: String, expiration: u64);
    }
}

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

use crate::types::{
    Attestation, AttestationRequest, AttestationStatus, AuditEntry, Error,
    ExpirationHook, FeeConfig, GlobalStats, HealthStatus, IssuerMetadata, IssuerStats, IssuerTier,
    MultiSigProposal, RateLimitConfig, StorageLimits,
};

#[contract]
pub struct TrustLinkContract;

#[contractimpl]
impl TrustLinkContract {
    // -----------------------------------------------------------------------
    // Initialization & Admin
    // -----------------------------------------------------------------------

    pub fn initialize(env: Env, admin: Address, ttl_days: Option<u32>) -> Result<(), Error> {
        admin::initialize(&env, admin, ttl_days)
    }

    pub fn transfer_admin(env: Env, current_admin: Address, new_admin: Address) -> Result<(), Error> {
        admin::transfer_admin(&env, current_admin, new_admin)
    }

    pub fn propose_admin_transfer(env: Env, current_admin: Address, new_admin: Address) -> Result<(), Error> {
        admin::propose_admin_transfer(&env, current_admin, new_admin)
    }

    pub fn cancel_admin_transfer(env: Env, current_admin: Address) -> Result<(), Error> {
        admin::cancel_admin_transfer(&env, current_admin)
    }

    pub fn accept_admin_transfer(env: Env, new_admin: Address) -> Result<(), Error> {
        admin::accept_admin_transfer(&env, new_admin)
    }

    pub fn add_admin(env: Env, existing_admin: Address, new_admin: Address) -> Result<(), Error> {
        admin::add_admin(&env, existing_admin, new_admin)
    }

    pub fn remove_admin(env: Env, existing_admin: Address, admin_to_remove: Address) -> Result<(), Error> {
        admin::remove_admin(&env, existing_admin, admin_to_remove)
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        admin::get_admin(&env)
    }

    // -----------------------------------------------------------------------
    // Issuer management
    // -----------------------------------------------------------------------

    pub fn register_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin::register_issuer(&env, admin, issuer)
    }

    pub fn remove_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin::remove_issuer(&env, admin, issuer)
    }

    pub fn add_to_whitelist(env: Env, issuer: Address, subject: Address) -> Result<(), Error> {
        admin::add_to_whitelist(&env, issuer, subject)
    }

    pub fn remove_from_whitelist(env: Env, issuer: Address, subject: Address) -> Result<(), Error> {
        admin::remove_from_whitelist(&env, issuer, subject)
    }

    #[must_use]
    pub fn is_whitelisted(env: Env, issuer: Address, subject: Address) -> bool {
        admin::is_whitelisted(&env, issuer, subject)
    }

    #[must_use]
    pub fn is_whitelist_enabled(env: Env, issuer: Address) -> bool {
        admin::is_whitelist_enabled(&env, issuer)
    }

    pub fn set_issuer_tier(env: Env, admin: Address, issuer: Address, tier: IssuerTier) -> Result<(), Error> {
        admin::set_issuer_tier(&env, admin, issuer, tier)
    }

    pub fn get_confidence_score(env: Env, attestation_id: String) -> Option<u32> {
        admin::get_confidence_score(&env, attestation_id)
    }

    pub fn get_issuer_metadata(env: Env, issuer: Address) -> Option<IssuerMetadata> {
        admin::get_issuer_metadata(&env, issuer)
    }

    pub fn set_issuer_metadata(env: Env, issuer: Address, metadata: IssuerMetadata) -> Result<(), Error> {
        admin::set_issuer_metadata(&env, issuer, metadata)
    }

    #[must_use]
    pub fn get_issuer_stats(env: Env, issuer: Address) -> IssuerStats {
        admin::get_issuer_stats(&env, issuer)
    }

    #[must_use]
    pub fn is_issuer(env: Env, address: Address) -> bool {
        admin::is_issuer(&env, address)
    }

    #[must_use]
    pub fn get_issuer_tier(env: Env, issuer: Address) -> Option<IssuerTier> {
        admin::get_issuer_tier(&env, issuer)
    }

    // -----------------------------------------------------------------------
    // Bridge management
    // -----------------------------------------------------------------------

    pub fn register_bridge(env: Env, admin: Address, bridge_contract: Address) -> Result<(), Error> {
        admin::register_bridge(&env, admin, bridge_contract)
    }

    pub fn is_bridge(env: Env, address: Address) -> bool {
        admin::is_bridge(&env, address)
    }

    // -----------------------------------------------------------------------
    // Whitelist mode
    // -----------------------------------------------------------------------

    pub fn set_whitelist_enabled(env: Env, issuer: Address, enabled: bool) -> Result<(), Error> {
        admin::set_whitelist_enabled(&env, issuer, enabled)
    }

    pub fn enable_whitelist_mode(env: Env, issuer: Address) -> Result<(), Error> {
        admin::enable_whitelist_mode(&env, issuer)
    }

    // -----------------------------------------------------------------------
    // Fee & rate limit
    // -----------------------------------------------------------------------

    pub fn get_fee_config(env: Env) -> Result<FeeConfig, Error> {
        admin::get_fee_config(&env)
    }

    pub fn set_fee(env: Env, admin: Address, fee: i128, collector: Address, fee_token: Option<Address>) -> Result<(), Error> {
        admin::set_fee(&env, admin, fee, collector, fee_token)
    }

    pub fn set_rate_limit(env: Env, admin: Address, min_issuance_interval: u64) -> Result<(), Error> {
        admin::set_rate_limit(&env, admin, min_issuance_interval)
    }

    #[must_use]
    pub fn get_rate_limit(env: Env) -> Option<RateLimitConfig> {
        admin::get_rate_limit(&env)
    }

    // -----------------------------------------------------------------------
    // Pause / unpause
    // -----------------------------------------------------------------------

    pub fn pause(env: Env, admin: Address) -> Result<(), Error> {
        admin::pause(&env, admin)
    }

    pub fn unpause(env: Env, admin: Address) -> Result<(), Error> {
        admin::unpause(&env, admin)
    }

    #[must_use]
    pub fn is_paused(env: Env) -> bool {
        admin::is_paused(&env)
    }

    // -----------------------------------------------------------------------
    // Limits
    // -----------------------------------------------------------------------

    #[must_use]
    pub fn get_limits(env: Env) -> StorageLimits {
        admin::get_limits(&env)
    }

    pub fn set_limits(env: Env, admin: Address, max_attestations_per_issuer: u32, max_attestations_per_subject: u32) -> Result<(), Error> {
        admin::set_limits(&env, admin, max_attestations_per_issuer, max_attestations_per_subject)
    }

    // -----------------------------------------------------------------------
    // Claim type registry
    // -----------------------------------------------------------------------

    pub fn register_claim_type(env: Env, admin: Address, claim_type: String, description: String) -> Result<(), Error> {
        admin::register_claim_type(&env, admin, claim_type, description)
    }

    #[must_use]
    pub fn get_claim_type_description(env: Env, claim_type: String) -> Option<String> {
        admin::get_claim_type_description(&env, claim_type)
    }

    #[must_use]
    pub fn list_claim_types(env: Env, start: u32, limit: u32) -> Vec<String> {
        admin::list_claim_types(&env, start, limit)
    }

    // -----------------------------------------------------------------------
    // Delegation
    // -----------------------------------------------------------------------

    pub fn delegate_claim_type(env: Env, issuer: Address, delegate: Address, claim_type: String, expiration: Option<u64>) -> Result<(), Error> {
        admin::delegate_claim_type(&env, issuer, delegate, claim_type, expiration)
    }

    // -----------------------------------------------------------------------
    // Expiration hooks
    // -----------------------------------------------------------------------

    pub fn register_expiration_hook(env: Env, subject: Address, callback_contract: Address, notify_days_before: u32) -> Result<(), Error> {
        admin::register_expiration_hook(&env, subject, callback_contract, notify_days_before)
    }

    #[must_use]
    pub fn get_expiration_hook(env: Env, subject: Address) -> Option<ExpirationHook> {
        admin::get_expiration_hook(&env, subject)
    }

    pub fn remove_expiration_hook(env: Env, subject: Address) -> Result<(), Error> {
        admin::remove_expiration_hook(&env, subject)
    }

    // -----------------------------------------------------------------------
    // Attestation creation
    // -----------------------------------------------------------------------

    pub fn create_attestation(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<String, Error> {
        attestation::create_attestation(&env, issuer, subject, claim_type, expiration, metadata, tags)
    }

    pub fn create_attestation_valid_from(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
        tags: Option<Vec<String>>,
        valid_from: u64,
    ) -> Result<String, Error> {
        attestation::create_attestation_valid_from(&env, issuer, subject, claim_type, expiration, metadata, tags, valid_from)
    }

    pub fn create_attestation_jurisdiction(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
        jurisdiction: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<String, Error> {
        attestation::create_attestation_jurisdiction(&env, issuer, subject, claim_type, expiration, metadata, jurisdiction, tags)
    }

    pub fn import_attestation(
        env: Env,
        admin: Address,
        issuer: Address,
        subject: Address,
        claim_type: String,
        timestamp: u64,
        expiration: Option<u64>,
    ) -> Result<String, Error> {
        attestation::import_attestation(&env, admin, issuer, subject, claim_type, timestamp, expiration)
    }

    pub fn bridge_attestation(
        env: Env,
        bridge: Address,
        subject: Address,
        claim_type: String,
        source_chain: String,
        source_tx: String,
    ) -> Result<String, Error> {
        attestation::bridge_attestation(&env, bridge, subject, claim_type, source_chain, source_tx)
    }

    pub fn create_attestations_batch(
        env: Env,
        issuer: Address,
        subjects: Vec<Address>,
        claim_type: String,
        expiration: Option<u64>,
    ) -> Result<Vec<String>, Error> {
        attestation::create_attestations_batch(&env, issuer, subjects, claim_type, expiration)
    }

    pub fn revoke_attestation(env: Env, issuer: Address, attestation_id: String, reason: Option<String>) -> Result<(), Error> {
        attestation::revoke_attestation(&env, issuer, attestation_id, reason)
    }

    pub fn renew_attestation(env: Env, issuer: Address, attestation_id: String, new_expiration: Option<u64>) -> Result<(), Error> {
        attestation::renew_attestation(&env, issuer, attestation_id, new_expiration)
    }

    pub fn revoke_attestations_batch(env: Env, issuer: Address, attestation_ids: Vec<String>, reason: Option<String>) -> Result<u32, Error> {
        attestation::revoke_attestations_batch(&env, issuer, attestation_ids, reason)
    }

    pub fn update_expiration(env: Env, issuer: Address, attestation_id: String, new_expiration: Option<u64>) -> Result<(), Error> {
        attestation::update_expiration(&env, issuer, attestation_id, new_expiration)
    }

    pub fn transfer_attestation(env: Env, admin: Address, attestation_id: String, new_issuer: Address) -> Result<(), Error> {
        attestation::transfer_attestation(&env, admin, attestation_id, new_issuer)
    }

    pub fn request_deletion(env: Env, subject: Address, attestation_id: String) -> Result<(), Error> {
        attestation::request_deletion(&env, subject, attestation_id)
    }

    pub fn endorse_attestation(env: Env, endorser: Address, attestation_id: String) -> Result<(), Error> {
        attestation::endorse_attestation(&env, endorser, attestation_id)
    }

    #[must_use]
    pub fn get_endorsement_count(env: Env, attestation_id: String) -> u32 {
        attestation::get_endorsement_count(&env, attestation_id)
    }

    // -----------------------------------------------------------------------
    // Query
    // -----------------------------------------------------------------------

    #[must_use]
    pub fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool {
        query::has_valid_claim(&env, subject, claim_type)
    }

    pub fn has_valid_claim_from_issuer(env: Env, subject: Address, claim_type: String, issuer: Address) -> bool {
        query::has_valid_claim_from_issuer(&env, subject, claim_type, issuer)
    }

    #[must_use]
    pub fn has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool {
        query::has_any_claim(&env, subject, claim_types)
    }

    #[must_use]
    pub fn has_all_claims(env: Env, subject: Address, claim_types: Vec<String>) -> bool {
        query::has_all_claims(&env, subject, claim_types)
    }

    #[must_use]
    pub fn get_attestation(env: Env, attestation_id: String) -> Result<Attestation, Error> {
        query::get_attestation(&env, attestation_id)
    }

    #[must_use]
    pub fn get_audit_log(env: Env, attestation_id: String) -> Vec<AuditEntry> {
        query::get_audit_log(&env, attestation_id)
    }

    #[must_use]
    pub fn get_attestation_status(env: Env, attestation_id: String) -> Result<AttestationStatus, Error> {
        query::get_attestation_status(&env, attestation_id)
    }

    #[must_use]
    pub fn get_subject_attestations(env: Env, subject: Address, start: u32, limit: u32) -> Vec<String> {
        query::get_subject_attestations(&env, subject, start, limit)
    }

    #[must_use]
    pub fn get_attestations_in_range(env: Env, subject: Address, from_ts: u64, to_ts: u64, start: u32, limit: u32) -> Vec<Attestation> {
        query::get_attestations_in_range(&env, subject, from_ts, to_ts, start, limit)
    }

    #[must_use]
    pub fn get_attestations_by_tag(env: Env, subject: Address, tag: String) -> Vec<String> {
        query::get_attestations_by_tag(&env, subject, tag)
    }

    #[must_use]
    pub fn get_attestations_by_jurisdiction(env: Env, subject: Address, jurisdiction: String, start: u32, limit: u32) -> Vec<String> {
        query::get_attestations_by_jurisdiction(&env, subject, jurisdiction, start, limit)
    }

    #[must_use]
    pub fn get_issuer_attestations(env: Env, issuer: Address, start: u32, limit: u32) -> Vec<String> {
        query::get_issuer_attestations(&env, issuer, start, limit)
    }

    pub fn get_issuer_attestation_count(env: Env, issuer: Address) -> u32 {
        query::get_issuer_attestation_count(&env, issuer)
    }

    #[must_use]
    pub fn get_valid_claims(env: Env, subject: Address) -> Vec<String> {
        query::get_valid_claims(&env, subject)
    }

    #[must_use]
    pub fn get_attestation_by_type(env: Env, subject: Address, claim_type: String) -> Option<Attestation> {
        query::get_attestation_by_type(&env, subject, claim_type)
    }

    pub fn get_subject_attestation_count(env: Env, subject: Address) -> u32 {
        query::get_subject_attestation_count(&env, subject)
    }

    pub fn get_valid_claim_count(env: Env, subject: Address) -> u32 {
        query::get_valid_claim_count(&env, subject)
    }

    #[must_use]
    pub fn get_global_stats(env: Env) -> GlobalStats {
        query::get_global_stats(&env)
    }

    // -----------------------------------------------------------------------
    // Multi-sig
    // -----------------------------------------------------------------------

    pub fn propose_attestation(
        env: Env,
        proposer: Address,
        subject: Address,
        claim_type: String,
        required_signers: Vec<Address>,
        threshold: u32,
    ) -> Result<String, Error> {
        multisig::propose_attestation(&env, proposer, subject, claim_type, required_signers, threshold)
    }

    pub fn cosign_attestation(env: Env, issuer: Address, proposal_id: String) -> Result<(), Error> {
        multisig::cosign_attestation(&env, issuer, proposal_id)
    }

    #[must_use]
    pub fn get_multisig_proposal(env: Env, proposal_id: String) -> Result<MultiSigProposal, Error> {
        multisig::get_multisig_proposal(&env, proposal_id)
    }

    #[must_use]
    pub fn get_multisig_ttl(env: Env) -> u32 {
        multisig::get_multisig_ttl(&env)
    }

    // -----------------------------------------------------------------------
    // Attestation request workflow
    // -----------------------------------------------------------------------

    pub fn request_attestation(env: Env, subject: Address, issuer: Address, claim_type: String) -> Result<String, Error> {
        request::request_attestation(&env, subject, issuer, claim_type)
    }

    pub fn fulfill_request(env: Env, issuer: Address, request_id: String, expiration: Option<u64>) -> Result<String, Error> {
        request::fulfill_request(&env, issuer, request_id, expiration)
    }

    pub fn reject_request(env: Env, issuer: Address, request_id: String, reason: Option<String>) -> Result<(), Error> {
        request::reject_request(&env, issuer, request_id, reason)
    }

    pub fn get_pending_requests(env: Env, issuer: Address, start: u32, limit: u32) -> Vec<AttestationRequest> {
        request::get_pending_requests(&env, issuer, start, limit)
    }

    pub fn get_request(env: Env, request_id: String) -> Result<AttestationRequest, Error> {
        request::get_request(&env, request_id)
    }

    /// Alias for `get_request`.
    pub fn get_attestation_request(env: Env, request_id: String) -> Result<AttestationRequest, Error> {
        request::get_request(&env, request_id)
    }

    // -----------------------------------------------------------------------
    // Misc
    // -----------------------------------------------------------------------

    #[must_use]
    pub fn get_version(env: Env) -> Result<String, Error> {
        admin::get_version(&env)
    }

    #[must_use]
    pub fn health_check(env: Env) -> HealthStatus {
        admin::health_check(&env)
    }

    // -----------------------------------------------------------------------
    // Attestation Templates
    // -----------------------------------------------------------------------

    /// Create or overwrite a named attestation template for the calling issuer.
    ///
    /// Templates capture default values for `claim_type`, optional expiration
    /// window, and optional metadata. They can be instantiated later via
    /// [`create_attestation_from_template`].
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    /// - [`Error::InvalidClaimType`] — `claim_type` is empty or invalid.
    /// - [`Error::MetadataTooLong`] — `metadata_template` exceeds 256 bytes.
    pub fn create_template(
        env: Env,
        issuer: Address,
        template_id: String,
        template: AttestationTemplate,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Validation::validate_claim_type(&template.claim_type)?;
        validate_metadata(&env, &template.metadata_template)?;

        Storage::set_template(&env, &issuer, &template_id, &template);
        Storage::add_to_template_registry(&env, &issuer, &template_id);
        Events::template_created(&env, &issuer, &template_id);
        Ok(())
    }

    /// Instantiate an attestation from a template, with optional field overrides.
    ///
    /// Loads the template for `(issuer, template_id)`, resolves the final
    /// expiration and metadata (override wins over template default), then
    /// creates and stores the attestation using the same logic as
    /// [`create_attestation`].
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    /// - [`Error::NotFound`] — `template_id` does not exist for this issuer.
    /// - [`Error::MetadataTooLong`] — `metadata_override` exceeds 256 bytes.
    /// - [`Error::InvalidExpiration`] — `expiration_override` ≤ current ledger timestamp.
    pub fn create_attestation_from_template(
        env: Env,
        issuer: Address,
        template_id: String,
        subject: Address,
        expiration_override: Option<u64>,
        metadata_override: Option<String>,
    ) -> Result<String, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;

        let template = Storage::get_template(&env, &issuer, &template_id)
            .ok_or(Error::NotFound)?;

        // Validate overrides before resolving.
        validate_metadata(&env, &metadata_override)?;

        let current_time = env.ledger().timestamp();
        if let Some(ts) = expiration_override {
            if ts <= current_time {
                return Err(Error::InvalidExpiration);
            }
        }

        // Resolve final expiration: override > template default > None.
        let expiration = if let Some(ts) = expiration_override {
            Some(ts)
        } else if let Some(days) = template.default_expiration_days {
            Some(current_time + (days as u64) * crate::constants::SECS_PER_DAY)
        } else {
            None
        };

        // Resolve final metadata: override > template default.
        let metadata = if metadata_override.is_some() {
            metadata_override
        } else {
            template.metadata_template.clone()
        };

        // Delegate to the shared internal creation path.
        Self::create_attestation_internal(
            &env,
            issuer,
            subject,
            template.claim_type,
            expiration,
            metadata,
            None,
            None,
        )
    }

    /// Return the ordered list of template IDs registered for `issuer`.
    ///
    /// Returns an empty `Vec` if the issuer has no templates. IDs are in
    /// insertion order (first-created first).
    #[must_use]
    pub fn list_templates(env: Env, issuer: Address) -> Vec<String> {
        Storage::get_template_registry(&env, &issuer)
    }

    /// Retrieve a single template by issuer and template ID.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — `template_id` does not exist for this issuer.
    pub fn get_template(
        env: Env,
        issuer: Address,
        template_id: String,
    ) -> Result<AttestationTemplate, Error> {
        Storage::get_template(&env, &issuer, &template_id).ok_or(Error::NotFound)
    }
}
