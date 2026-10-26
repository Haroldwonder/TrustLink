#![no_std]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

#[cfg(test)]
extern crate std;

mod admin;
mod attestation;
mod bundle;
mod errors;
mod events;
mod multisig;
mod query;
mod request;
mod storage;
mod constants;
mod template;
pub mod types;
mod validation;
pub use crate::validation::Validation;

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
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    AdminCouncil, Attestation, AttestationBundle, AttestationRequest, AttestationStatus,
    AttestationTemplate, AttestationVersionSnapshot, AuditAction, AuditEntry, ClaimTypeConstraints,
    ClaimTypeInfo, ContractConfig, ContractMetadata, CouncilOperation, CouncilProposal, DecayConfig,
    Delegation, DisputeRecord, Endorsement, Error, ExpirationHook, FeeConfig, GlobalStats,
    HealthStatus, IssuerMetadata, IssuerStats, IssuerTier, MultiSigProposal, PendingAdminTransfer,
    RateLimitConfig, RequestStatus, RevocationList, RevocationListFormat, StorageLimits, TtlConfig,
    ATTESTATION_REQUEST_TTL_SECS, MULTISIG_PROPOSAL_TTL_SECS, SECS_PER_DAY,
};

/// The TrustLink smart contract.
///
/// Every public function in this `impl` block is a **one-line delegation** to
/// the appropriate sub-module.  All business logic lives in:
///
/// - [`admin`]      — initialization, issuer/admin management, config
/// - [`attestation`] — attestation CRUD
/// - [`bundle`]     — atomic attestation bundles
/// - [`multisig`]   — multi-sig proposals
/// - [`query`]      — read-only queries and exports
/// - [`request`]    — pull-based attestation requests
/// - [`template`]   — attestation templates
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

    #[must_use]
    pub fn get_pending_admin_transfer(env: Env) -> Option<PendingAdminTransfer> {
        admin::get_pending_admin_transfer(&env)
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

    #[must_use]
    pub fn get_admin_council(env: Env) -> Result<Vec<Address>, Error> {
        admin::get_admin_council(&env)
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

    #[must_use]
    pub fn get_issuer_list(env: Env, start: u32, limit: u32) -> Vec<Address> {
        admin::get_issuer_list(&env, start, limit)
    }

    pub fn set_issuer_tier(env: Env, admin: Address, issuer: Address, tier: IssuerTier) -> Result<(), Error> {
        admin::set_issuer_tier(&env, admin, issuer, tier)
    }

    /// Create a new attestation about a subject address.
    ///
    /// The attestation ID is derived deterministically from `(issuer, subject,
    /// claim_type, timestamp)`, so the same combination at the same ledger
    /// timestamp will always produce the same ID.
    ///
    /// Emits an [`events::Events::attestation_created`] event on success.
    pub fn add_to_whitelist(env: Env, issuer: Address, subject: Address) -> Result<(), Error> {
        admin::add_to_whitelist(&env, issuer, subject)
    }

    /// Bulk-add multiple subjects to the issuer's whitelist.
    ///
    /// Adds each subject to the whitelist in a single call. Emits a whitelist
    /// event for each successful addition.
    ///
    /// # Parameters
    /// - `issuer` — registered issuer address (must authorize).
    /// - `subjects` — list of subject addresses to whitelist.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    /// - [`Error::LimitExceeded`] — `subjects.len()` exceeds 50.
    /// - [`Error::ContractPaused`] — contract is paused.
    pub fn bulk_add_to_whitelist(env: Env, issuer: Address, subjects: Vec<Address>) -> Result<(), Error> {
        admin::bulk_add_to_whitelist(&env, issuer, subjects)
    }

    pub fn remove_from_whitelist(env: Env, issuer: Address, subject: Address) -> Result<(), Error> {
        admin::remove_from_whitelist(&env, issuer, subject)
    }

    #[must_use]
    pub fn is_whitelisted(env: Env, issuer: Address, subject: Address) -> bool {
        admin::is_whitelisted(&env, issuer, subject)
    }

    #[must_use]
    pub fn get_issuer_tier(env: Env, issuer: Address) -> Option<IssuerTier> {
        admin::get_issuer_tier(&env, issuer)
    }

    pub fn get_confidence_score(env: Env, attestation_id: String) -> Option<u32> {
        admin::get_confidence_score(&env, attestation_id)
    }

    pub fn set_decay_config(env: Env, admin: Address, config: DecayConfig) -> Result<(), Error> {
        admin::set_decay_config(&env, admin, config)
    }

    #[must_use]
    pub fn get_decay_config(env: Env) -> DecayConfig {
        admin::get_decay_config(&env)
    }

    #[must_use]
    pub fn is_decay_config_set(env: Env) -> bool {
        admin::is_decay_config_set(&env)
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

    // -----------------------------------------------------------------------
    // Bridge management
    // -----------------------------------------------------------------------

    pub fn register_bridge(env: Env, admin: Address, bridge_contract: Address) -> Result<(), Error> {
        admin::register_bridge(&env, admin, bridge_contract)
    }

    pub fn is_bridge(env: Env, address: Address) -> bool {
        admin::is_bridge(&env, address)
    }

    #[must_use]
    pub fn get_bridge_list(env: Env, start: u32, limit: u32) -> Vec<Address> {
        admin::get_bridge_list(&env, start, limit)
    }

    // -----------------------------------------------------------------------
    // Whitelist mode
    // -----------------------------------------------------------------------

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

    pub fn set_ttl_days(env: Env, admin: Address, days: u32) -> Result<(), Error> {
        admin::set_ttl_days(&env, admin, days)
    }

    pub fn set_rate_limit(env: Env, admin: Address, min_issuance_interval: u64) -> Result<(), Error> {
        admin::set_rate_limit(&env, admin, min_issuance_interval)
    }

    #[must_use]
    pub fn get_rate_limit(env: Env) -> Option<RateLimitConfig> {
        admin::get_rate_limit(&env)
    }

    pub fn set_rate_limit_for_claim_type(env: Env, admin: Address, claim_type: String, interval_secs: u64) -> Result<(), Error> {
        admin::set_rate_limit_for_claim_type(&env, admin, claim_type, interval_secs)
    }

    #[must_use]
    pub fn get_rate_limit_for_claim_type(env: Env, claim_type: String) -> Option<u64> {
        admin::get_rate_limit_for_claim_type(&env, claim_type)
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
    // Contract Config
    // -----------------------------------------------------------------------

    pub fn set_registered_claim_type(env: Env, admin: Address, require: bool) -> Result<(), Error> {
        admin::set_require_registered_claim_type(&env, admin, require)
    }

    #[must_use]
    pub fn get_registered_claim_type(env: Env) -> bool {
        admin::get_require_registered_claim_type(&env)
    }

    pub fn set_metadata_hash_only(env: Env, admin: Address, enabled: bool) -> Result<(), Error> {
        admin::set_metadata_hash_only(&env, admin, enabled)
    }

    #[must_use]
    pub fn get_metadata_hash_only(env: Env) -> bool {
        admin::get_metadata_hash_only(&env)
    }

    pub fn set_max_attestations_per_subject(env: Env, admin: Address, limit: Option<u32>) -> Result<(), Error> {
        admin::set_max_attestations_per_subject(&env, admin, limit)
    }

    #[must_use]
    pub fn get_max_attestations_per_subject(env: Env) -> Option<u32> {
        admin::get_max_attestations_per_subject(&env)
    }

    pub fn set_chunk_size(env: Env, admin: Address, chunk_size: u32) -> Result<(), Error> {
        admin::set_chunk_size(&env, admin, chunk_size)
    }

    #[must_use]
    pub fn get_chunk_size(env: Env) -> u32 {
        admin::get_chunk_size(&env)
    }

    #[must_use]
    pub fn get_config(env: Env) -> ContractConfig {
        admin::get_config(&env)
    }

    pub fn get_contract_metadata(env: Env) -> Result<ContractMetadata, Error> {
        admin::get_contract_metadata(&env)
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

    pub fn set_claim_type_status(env: Env, admin: Address, claim_type: String, status: types::ClaimTypeStatus) -> Result<(), Error> {
        admin::set_claim_type_status(&env, admin, claim_type, status)
    }

    #[must_use]
    pub fn get_claim_type_description(env: Env, claim_type: String) -> Option<String> {
        admin::get_claim_type_description(&env, claim_type)
    }

    #[must_use]
    pub fn list_claim_types(env: Env, start: u32, limit: u32) -> Vec<String> {
        admin::list_claim_types(&env, start, limit)
    }

    pub fn set_claim_type_constraints(env: Env, admin: Address, claim_type: String, constraints: ClaimTypeConstraints) -> Result<(), Error> {
        admin::set_claim_type_constraints(&env, admin, claim_type, constraints)
    }

    /// Clear (remove) constraints for a specific claim type.
    ///
    /// Removes all metadata constraints previously set for the claim type,
    /// reverting to the default behavior (no constraints).
    ///
    /// # Parameters
    /// - `admin` — current administrator address (must authorize).
    /// - `claim_type` — the claim type to clear constraints for.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `admin` is not the registered administrator.
    /// - [`Error::NotFound`] — `claim_type` is not in the registry.
    /// - [`Error::ContractPaused`] — contract is paused.
    pub fn clear_claim_type_constraints(env: Env, admin: Address, claim_type: String) -> Result<(), Error> {
        admin::clear_claim_type_constraints(&env, admin, claim_type)
    }

    #[must_use]
    pub fn get_claim_type_constraints(env: Env, claim_type: String) -> Option<ClaimTypeConstraints> {
        admin::get_claim_type_constraints(&env, claim_type)
    }

    // -----------------------------------------------------------------------
    // Delegation
    // -----------------------------------------------------------------------

    pub fn delegate_claim_type(env: Env, issuer: Address, delegate: Address, claim_type: String, expiration: Option<u64>) -> Result<(), Error> {
        admin::delegate_claim_type(&env, issuer, delegate, claim_type, expiration)
    }

    pub fn revoke_delegation(env: Env, issuer: Address, delegate: Address, claim_type: String) -> Result<(), Error> {
        admin::revoke_delegation(&env, issuer, delegate, claim_type)
    }

    pub fn revoke_delegation_all(env: Env, delegator: Address) -> Result<(), Error> {
        admin::revoke_delegation_all(&env, delegator)
    }

    #[must_use]
    pub fn get_delegation(env: Env, delegator: Address, delegate: Address, claim_type: String) -> Option<Delegation> {
        query::get_delegation(&env, delegator, delegate, claim_type)
    }

    #[must_use]
    pub fn list_delegations_by_delegator(env: Env, delegator: Address, start: u32, limit: u32) -> Vec<Delegation> {
        admin::list_delegations_by_delegator(&env, delegator, start, limit)
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

    pub fn create_attestation_versioned(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
        tags: Option<Vec<String>>,
        expected_version: Option<String>,
    ) -> Result<String, Error> {
        attestation::create_attestation_versioned(
            &env, issuer, subject, claim_type, expiration, metadata, tags, expected_version,
        )
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

    pub fn create_attestation_bundle(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_types: Vec<String>,
        expiration: Option<u64>,
        metadata: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<String, Error> {
        bundle::create_attestation_bundle(&env, issuer, subject, claim_types, expiration, metadata, tags)
    }

    pub fn revoke_attestation(env: Env, issuer: Address, attestation_id: String, reason: Option<String>) -> Result<(), Error> {
        attestation::revoke_attestation(&env, issuer, attestation_id, reason)
    }

    pub fn renew_attestation(env: Env, issuer: Address, attestation_id: String, new_expiration: Option<u64>) -> Result<(), Error> {
        attestation::renew_attestation(&env, issuer, attestation_id, new_expiration)
    }

    pub fn update_expiration(env: Env, issuer: Address, attestation_id: String, new_expiration: Option<u64>) -> Result<(), Error> {
        attestation::update_expiration(&env, issuer, attestation_id, new_expiration)
    }

    /// Permissionless bump of an attestation's TTL (and all of its related
    /// keys) so it stays live until its expiration. Intended to be called by a
    /// keeper job on a periodic schedule, since the host silently clamps
    /// persistent extensions beyond `max_entry_ttl` (roughly 6 months on
    /// mainnet). No authorization is required.
    pub fn bump_attestation_ttl(env: Env, attestation_id: String) -> Result<(), Error> {
        Storage::bump_attestation_ttl(&env, &attestation_id)
    }

    pub fn transfer_attestation(env: Env, admin: Address, attestation_id: String, new_issuer: Address) -> Result<(), Error> {
        attestation::transfer_attestation(&env, admin, attestation_id, new_issuer)
    }

    pub fn request_deletion(env: Env, subject: Address, attestation_id: String) -> Result<(), Error> {
        attestation::request_deletion(&env, subject, attestation_id)
    }

    pub fn amend_attestation(env: Env, issuer: Address, attestation_id: String, new_metadata: Option<String>) -> Result<(), Error> {
        attestation::amend_attestation(&env, issuer, attestation_id, new_metadata)
    }

    pub fn endorse_attestation(env: Env, endorser: Address, attestation_id: String) -> Result<(), Error> {
        attestation::endorse_attestation(&env, endorser, attestation_id)
    }

    #[must_use]
    pub fn get_endorsement_count(env: Env, attestation_id: String) -> u32 {
        attestation::get_endorsement_count(&env, attestation_id)
    }

    #[must_use]
    pub fn list_endorsements_by_endorser(env: Env, endorser: Address, start: u32, limit: u32) -> Vec<Endorsement> {
        attestation::list_endorsements_by_endorser(&env, endorser, start, limit)
    }

    pub fn create_attestation_as_delegate(
        env: Env,
        delegate: Address,
        delegator: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
    ) -> Result<String, Error> {
        attestation::create_attestation_as_delegate(&env, delegate, delegator, subject, claim_type, expiration, metadata)
    }

    pub fn simulate_create_attestation(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<(String, i128), Error> {
        attestation::simulate_create_attestation(&env, issuer, subject, claim_type, expiration, metadata, tags)
    }

    pub fn revoke_attestations_batch(env: Env, issuer: Address, attestation_ids: Vec<String>, reason: Option<String>) -> Result<u32, Error> {
        attestation::revoke_attestations_batch(&env, issuer, attestation_ids, reason)
    }

    // -----------------------------------------------------------------------
    // Bundles
    // -----------------------------------------------------------------------

    pub fn create_attestation_bundle(
        env: Env,
        issuer: Address,
        subjects: Vec<Address>,
        claim_type: String,
        expiration: Option<u64>,
    ) -> Result<String, Error> {
        attestation::create_attestation_bundle(&env, issuer, subjects, claim_type, expiration)
    }

    #[must_use]
    pub fn is_bundle_valid(env: Env, bundle_id: String) -> Result<bool, Error> {
        attestation::is_bundle_valid(&env, bundle_id)
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
    pub fn has_valid_claim_batch(env: Env, subjects: Vec<Address>, claim_type: String) -> Vec<bool> {
        query::has_valid_claim_batch(&env, subjects, claim_type)
    }

    /// Issue #1567: Check if a subject has a valid attestation of a specific claim type
    /// that is NOT under an open dispute. Verifiers can use this to enforce
    /// "valid and not under dispute" semantics.
    #[must_use]
    pub fn has_valid_undisputed_claim(env: Env, subject: Address, claim_type: String) -> bool {
        query::has_valid_undisputed_claim(&env, subject, claim_type)
    }

    /// Issue #1567: Check if a subject has a valid attestation from a specific issuer
    /// of a given claim type that is NOT under an open dispute.
    #[must_use]
    pub fn has_valid_undisputed_claim_from_issuer(
        env: Env,
        subject: Address,
        claim_type: String,
        issuer: Address,
    ) -> bool {
        query::has_valid_undisputed_claim_from_issuer(&env, subject, claim_type, issuer)
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
    pub fn export_revocation_list(
        env: Env,
        issuer: Address,
        claim_type: Option<String>,
        format: RevocationListFormat,
        start: u32,
        limit: u32,
    ) -> Result<RevocationList, Error> {
        query::export_revocation_list(&env, issuer, claim_type, format, start, limit)
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
    pub fn get_attestations_in_range_after(
        env: Env,
        subject: Address,
        from_ts: u64,
        to_ts: u64,
        after_attestation_id: Option<String>,
        limit: u32,
    ) -> Vec<Attestation> {
        query::get_attestations_in_range_after(&env, subject, from_ts, to_ts, after_attestation_id, limit)
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

    pub fn get_expiring_attestations(
        env: Env,
        subject: Address,
        within_days: u32,
        start: u32,
        limit: u32,
    ) -> Result<Vec<Attestation>, Error> {
        query::get_expiring_attestations(&env, subject, within_days, start, limit)
    }

    pub fn get_issuer_expiring_attestations(
        env: Env,
        issuer: Address,
        days_window: u32,
        start: u32,
        limit: u32,
    ) -> Result<Vec<Attestation>, Error> {
        query::get_issuer_expiring_attestations(&env, issuer, days_window, start, limit)
    }

    #[must_use]
    pub fn get_global_stats(env: Env) -> GlobalStats {
        query::get_global_stats(&env)
    }

    #[must_use]
    pub fn get_attestation_history(env: Env, attestation_id: String) -> Vec<AttestationVersionSnapshot> {
        query::get_attestation_history(&env, attestation_id)
    }

    pub fn dispute_attestation(env: Env, subject: Address, attestation_id: String, reason: String) -> Result<(), Error> {
        query::dispute_attestation(&env, subject, attestation_id, reason)
    }

    #[must_use]
    pub fn get_dispute(env: Env, attestation_id: String) -> Option<DisputeRecord> {
        query::get_dispute(&env, attestation_id)
    }

    pub fn resolve_dispute(env: Env, resolver: Address, attestation_id: String) -> Result<(), Error> {
        admin::resolve_dispute(&env, resolver, attestation_id)
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

    pub fn set_multisig_ttl(env: Env, admin: Address, days: u32) -> Result<(), Error> {
        multisig::set_multisig_ttl(&env, admin, days)
    }

    pub fn cancel_multisig_proposal(env: Env, proposer: Address, proposal_id: String) -> Result<(), Error> {
        multisig::cancel_multisig_proposal(&env, proposer, proposal_id)
    }

    #[must_use]
    pub fn list_open_proposals(env: Env, subject: Address, start: u32, limit: u32) -> Vec<MultiSigProposal> {
        multisig::list_open_proposals(&env, subject, start, limit)
    }

    // -----------------------------------------------------------------------
    // Attestation requests
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

    pub fn cancel_request(env: Env, subject: Address, request_id: String) -> Result<(), Error> {
        request::cancel_request(&env, subject, request_id)
    }

    #[must_use]
    pub fn get_pending_requests(env: Env, issuer: Address, start: u32, limit: u32) -> Vec<AttestationRequest> {
        request::get_pending_requests(&env, issuer, start, limit)
    }

    pub fn get_attestation_request(env: Env, request_id: String) -> Result<AttestationRequest, Error> {
        request::get_request(&env, request_id)
    }

    pub fn cleanup_expired_requests(env: Env, issuer: Address) -> Result<(), Error> {
        request::cleanup_expired_requests(&env, issuer)
    }

    /// Set the TTL (time-to-live) for multisig proposals.
    ///
    /// Controls how long a multisig proposal remains valid before expiring.
    /// The TTL is specified in days and applies only to proposals created
    /// after this call; existing proposals retain their original expiration.
    ///
    /// Default: 7 days.
    ///
    /// # Parameters
    /// - `admin` — current administrator address (must authorize).
    /// - `days` — TTL in days (1-365, subject to bounds validation once added).
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `admin` is not the registered administrator.
    pub fn set_multisig_ttl(env: Env, admin: Address, days: u32) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::set_multisig_ttl(&env, days);
        Ok(())
    }

    /// Cancel an unfinalized multisig proposal.
    ///
    /// Only the original proposer may cancel. Cancelled proposals are excluded
    /// from future `cosign_attestation` calls (returns `ProposalExpired`).
    ///
    /// # Errors
    /// - [`Error::NotFound`] — proposal does not exist.
    /// - [`Error::Unauthorized`] — caller is not the original proposer.
    /// - [`Error::ProposalFinalized`] — proposal has already been finalized.
    /// - [`Error::ProposalCancelled`] — proposal is already cancelled.
    pub fn cancel_multisig_proposal(
        env: Env,
        proposer: Address,
        proposal_id: String,
    ) -> Result<(), Error> {
        proposer.require_auth();
        Validation::require_not_paused(&env)?;

        let mut proposal = Storage::get_multisig_proposal(&env, &proposal_id)?;

        if proposal.proposer != proposer {
            return Err(Error::Unauthorized);
        }

        if proposal.finalized {
            return Err(Error::ProposalFinalized);
        }

        if proposal.cancelled {
            return Err(Error::ProposalCancelled);
        }

        proposal.cancelled = true;
        Storage::set_multisig_proposal(&env, &proposal);
        Events::multisig_cancelled(&env, &proposal_id, &proposer);
        Ok(())
    }

    /// List open (unfinalized, unexpired, uncancelled) proposals for a subject.
    ///
    /// Returns a paginated slice of proposals where `finalized == false`,
    /// `cancelled == false`, and `expires_at > now`.
    ///
    /// # Parameters
    /// - `subject` — the subject address whose proposals to query.
    /// - `start` — zero-based offset into the filtered list.
    /// - `limit` — maximum number of proposals to return.
    pub fn list_open_proposals(
        env: Env,
        issuer: Address,
        template_id: String,
        subject: Address,
        expiration_override: Option<u64>,
        metadata_override: Option<String>,
    ) -> Result<String, Error> {
        template::create_attestation_from_template(&env, issuer, template_id, subject, expiration_override, metadata_override)
    }

    #[must_use]
    pub fn list_templates(env: Env, issuer: Address) -> Vec<String> {
        template::list_templates(&env, issuer)
    }

    pub fn get_template(env: Env, issuer: Address, template_id: String) -> Result<AttestationTemplate, Error> {
        template::get_template(&env, issuer, template_id)
    }

    // -----------------------------------------------------------------------
    // Bundle queries
    // -----------------------------------------------------------------------

    pub fn get_bundle(env: Env, bundle_id: String) -> Result<AttestationBundle, Error> {
        bundle::get_bundle(&env, bundle_id)
    }

    pub fn get_bundle_attestations(env: Env, bundle_id: String) -> Result<Vec<Attestation>, Error> {
        bundle::get_bundle_attestations(&env, &bundle_id)
    }

    pub fn is_bundle_valid(env: Env, bundle_id: String) -> Result<bool, Error> {
        bundle::is_bundle_valid(&env, &bundle_id)
    }

    pub fn get_issuer_bundles(env: Env, issuer: Address) -> Vec<String> {
        bundle::get_issuer_bundles(&env, issuer)
    }

    pub fn get_subject_bundles(env: Env, subject: Address) -> Vec<String> {
        bundle::get_subject_bundles(&env, subject)
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
    // Council actions with timelock
    // -----------------------------------------------------------------------

    pub fn create_council_proposal(env: Env, proposer: Address, operation: CouncilOperation) -> Result<u32, Error> {
        admin::create_council_proposal(&env, proposer, operation)
    }

    pub fn approve_council_proposal(env: Env, approver: Address, proposal_id: u32) -> Result<(), Error> {
        admin::approve_council_proposal(&env, approver, proposal_id)
    }

    pub fn execute_council_action(env: Env, executor: Address, proposal_id: u32) -> Result<(), Error> {
        admin::execute_council_action(&env, executor, proposal_id)
    }

    #[must_use]
    pub fn get_council_proposal(env: Env, proposal_id: u32) -> Option<CouncilProposal> {
        admin::get_council_proposal(&env, proposal_id)
    }

    pub fn set_council_timelock_delay(env: Env, admin: Address, delay_seconds: u64) -> Result<(), Error> {
        admin::set_council_timelock_delay(&env, admin, delay_seconds)
    }

    #[must_use]
    pub fn get_council_timelock_delay(env: Env) -> u64 {
        admin::get_council_timelock_delay(&env)
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
    /// Issue #1579: Enhanced validation to:
    /// - Validate template_id (charset/length)
    /// - Validate default_expiration_days (1..=MAX_TEMPLATE_DAYS)
    /// - Reject silent overwrites (check for existing template)
    /// - Use checked arithmetic for expiration calculation
    /// - Prevent creation while paused
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    /// - [`Error::InvalidClaimType`] — `claim_type` is empty or invalid.
    /// - [`Error::MetadataTooLong`] — `metadata_template` exceeds 256 bytes.
    /// - [`Error::InvalidTemplateId`] — `template_id` is empty, too long, or contains invalid characters.
    /// - [`Error::InvalidExpiration`] — `default_expiration_days` is 0 or exceeds MAX_TEMPLATE_DAYS.
    /// - [`Error::ConflictError`] — `template_id` already exists (silent overwrite prevention).
    pub fn create_template(
        env: Env,
        issuer: Address,
        template_id: String,
        template: AttestationTemplate,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Validation::require_not_paused(&env)?;
        Validation::validate_claim_type(&template.claim_type)?;
        Validation::validate_metadata(&env, &template.metadata_template)?;
        
        // Issue #1579: Validate template_id
        Validation::validate_template_id(&template_id)?;
        
        // Issue #1579: Validate default_expiration_days
        if let Some(days) = template.default_expiration_days {
            if days < crate::constants::MIN_TEMPLATE_EXPIRATION_DAYS || days > crate::constants::MAX_TEMPLATE_EXPIRATION_DAYS {
                return Err(Error::InvalidExpiration);
            }
        }
        
        // Issue #1579: Prevent silent overwrites
        if let Some(_existing) = Storage::get_template(&env, &issuer, &template_id) {
            return Err(Error::ConflictError); // Template already exists
        }

        Storage::set_template(&env, &issuer, &template_id, &template);
        Storage::add_to_template_registry(&env, &issuer, &template_id);
        
        // Issue #1579: Emit template creation event
        Events::template_created(&env, &issuer, &template_id);
        
        Ok(())
    }

    /// Update an existing attestation template for the calling issuer.
    ///
    /// Issue #1579: Explicit update function (separate from create) to prevent confusion.
    /// Allows updating template configuration while maintaining an explicit audit trail.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    /// - [`Error::NotFound`] — `template_id` does not exist for this issuer.
    /// - [`Error::InvalidClaimType`] — `claim_type` is empty or invalid.
    /// - [`Error::MetadataTooLong`] — `metadata_template` exceeds 256 bytes.
    /// - [`Error::InvalidExpiration`] — `default_expiration_days` is 0 or exceeds MAX_TEMPLATE_DAYS.
    pub fn update_template(
        env: Env,
        issuer: Address,
        template_id: String,
        template: AttestationTemplate,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Validation::require_not_paused(&env)?;
        Validation::validate_claim_type(&template.claim_type)?;
        Validation::validate_metadata(&env, &template.metadata_template)?;
        
        // Issue #1579: Validate default_expiration_days
        if let Some(days) = template.default_expiration_days {
            if days < crate::constants::MIN_TEMPLATE_EXPIRATION_DAYS || days > crate::constants::MAX_TEMPLATE_EXPIRATION_DAYS {
                return Err(Error::InvalidExpiration);
            }
        }
        
        // Verify template exists
        if Storage::get_template(&env, &issuer, &template_id).is_none() {
            return Err(Error::NotFound);
        }

        Storage::set_template(&env, &issuer, &template_id, &template);
        
        // Emit template update event
        Events::template_updated(&env, &issuer, &template_id);
        
        Ok(())
    }

    /// Instantiate an attestation from a template, with optional field overrides.
    ///
    /// Loads the template for `(issuer, template_id)`, resolves the final
    /// expiration and metadata (override wins over template default), then
    /// creates and stores the attestation using the same logic as
    /// [`create_attestation`].
    ///
    /// Issue #1579: Uses checked arithmetic to prevent overflow in expiration calculation.
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
        Validation::validate_metadata(&env, &metadata_override)?;

        let current_time = env.ledger().timestamp();
        if let Some(ts) = expiration_override {
            if ts <= current_time {
                return Err(Error::InvalidExpiration);
            }
        }

        // Resolve final expiration: override > template default > None.
        // Issue #1579: Use checked arithmetic to prevent overflow
        let expiration = if let Some(ts) = expiration_override {
            Some(ts)
        } else if let Some(days) = template.default_expiration_days {
            // Use checked_mul and checked_add to prevent overflow
            let secs = (days as u64).checked_mul(crate::constants::SECS_PER_DAY)
                .ok_or(Error::InvalidExpiration)?;
            let expiry = current_time.checked_add(secs)
                .ok_or(Error::InvalidExpiration)?;
            Some(expiry)
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
        attestation::create_attestation_internal(
            &env,
            issuer,
            subject,
            template.claim_type.clone(),
            expiration,
            metadata,
            None,
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

    /// Get the tier level for an issuer.
    ///
    /// # Parameters
    /// - `issuer` — issuer address to query.
    ///
    /// # Returns
    /// The issuer's current [`IssuerTier`].
    ///
    /// # Errors
    /// - [`Error::NotFound`] — `issuer` is not registered.
    ///
    /// # Examples
    /// ```ignore
    /// let tier = client.get_issuer_tier(&issuer_address);
    /// ```
    pub fn get_issuer_tier(env: Env, issuer: Address) -> Result<IssuerTier, Error> {
        let metadata = Storage::get_issuer_metadata(&env, &issuer)?;
        Ok(metadata.tier)
    }
}
