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
pub use crate::validation::Validation;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env, String, Vec};

use crate::constants::SECS_PER_DAY;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    AdminCouncil, AttestationTemplate, Attestation, AttestationOrigin, AttestationRequest,
    AttestationStatus, AuditAction, AuditEntry, ClaimTypeInfo, ContractConfig, ContractMetadata,
    CouncilOperation, CouncilProposal, Delegation, Error, FeeConfig, GlobalStats,
    HealthStatus, IssuerMetadata, IssuerStats, IssuerTier, MultiSigProposal, PendingAdminTransfer,
    RateLimitConfig, RequestStatus, StorageLimits, TtlConfig, ATTESTATION_REQUEST_TTL_SECS,
    MULTISIG_PROPOSAL_TTL_SECS,
};
use crate::validation::Validation;

const MAX_SOURCE_CHAIN_LEN: u32 = 32;
const MAX_SOURCE_TX_LEN: u32 = 128;

mod callback {
pub(crate) mod callback {
    use soroban_sdk::{contractclient, Address, Env, String};
    #[contractclient(name = "ExpirationCallbackClient")]
    #[allow(dead_code)]
    pub trait ExpirationCallback {
        fn notify_expiring(env: Env, subject: Address, attestation_id: String, expiration: u64);
    }
}

fn validate_native_expiration(env: &Env, expiration: Option<u64>) -> Result<(), Error> {
    if let Some(v) = expiration {
        if v <= env.ledger().timestamp() {
            return Err(Error::InvalidExpiration);
        }
    }
    Ok(())
}

fn validate_import_timestamps(env: &Env, timestamp: u64, expiration: Option<u64>) -> Result<(), Error> {
    if timestamp > env.ledger().timestamp() {
        return Err(Error::InvalidTimestamp);
    }
    if let Some(v) = expiration {
        if v <= timestamp {
            return Err(Error::InvalidExpiration);
        }
    }
    Ok(())
}

fn validate_fee_config(env: &Env, fee: i128, fee_token: &Option<Address>) -> Result<(), Error> {
    if fee < 0 {
        return Err(Error::InvalidFee);
    }
    if fee > 0 && fee_token.is_none() {
        return Err(Error::FeeTokenRequired);
    }
    if let Some(token_addr) = fee_token {
        let token = TokenClient::new(env, token_addr);
        token
            .try_balance(&env.current_contract_address())
            .map_err(|_| Error::InvalidFeeToken)?;
    }
    Ok(())
}

fn check_rate_limit(env: &Env, issuer: &Address) -> Result<(), Error> {
    if let Some(config) = Storage::get_rate_limit_config(env) {
        if config.min_issuance_interval == 0 {
            return Ok(());
        }
        let current_time = env.ledger().timestamp();
        if let Some(last) = Storage::get_last_issuance_time(env, issuer) {
            if current_time.saturating_sub(last) < config.min_issuance_interval {
                return Err(Error::RateLimited);
            }
        }
    }
    Ok(())
}

fn default_fee_config(admin: &Address) -> FeeConfig {
    FeeConfig { attestation_fee: 0, fee_collector: admin.clone(), fee_token: None }
}

fn load_fee_config(env: &Env) -> Result<FeeConfig, Error> {
    Storage::get_fee_config(env).ok_or(Error::NotInitialized)
}

fn charge_attestation_fee(env: &Env, issuer: &Address) -> Result<(), Error> {
    let fee_config = load_fee_config(env)?;
    if fee_config.attestation_fee == 0 {
        return Ok(());
    }
    let fee_token = fee_config.fee_token.ok_or(Error::FeeTokenRequired)?;
    TokenClient::new(env, &fee_token).transfer(issuer, &fee_config.fee_collector, &fee_config.attestation_fee);
    Ok(())
}

fn store_attestation(env: &Env, attestation: &Attestation) {
    Storage::set_attestation(env, attestation);
    Storage::add_subject_attestation(env, &attestation.subject, &attestation.id);
    Storage::add_issuer_attestation(env, &attestation.issuer, &attestation.id);
    let mut stats = Storage::get_issuer_stats(env, &attestation.issuer);
    stats.total_issued += 1;
    Storage::set_issuer_stats(env, &attestation.issuer, &stats);
    Storage::increment_total_attestations(env, 1);
    Storage::increment_claim_type_count(env, &attestation.claim_type);
}

fn maybe_trigger_expiration_hook(
    env: &Env,
    subject: &Address,
    attestation_id: &String,
    expiration: u64,
    current_time: u64,
) {
    let hook = match Storage::get_expiration_hook(env, subject) {
        Some(h) => h,
        None => return,
    };
    let notify_window = (hook.notify_days_before as u64) * SECS_PER_DAY;
    let notify_from = expiration.saturating_sub(notify_window);
    if current_time >= notify_from && current_time < expiration {
        Events::expiration_hook_triggered(env, subject, attestation_id, expiration);
        let client = ExpirationCallbackClient::new(env, &hook.callback_contract);
        let _ = client.try_notify_expiring(subject, attestation_id, &expiration);
    }
}
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    AdminCouncil, Attestation, AttestationRequest, AttestationStatus, AttestationTemplate,
    AuditEntry, Endorsement, Error, ExpirationHook, FeeConfig, GlobalStats, HealthStatus,
    IssuerMetadata, IssuerStats, IssuerTier, MultiSigProposal, PendingAdminTransfer,
    RateLimitConfig, StorageLimits,
    Attestation, AttestationRequest, AttestationStatus, AttestationTemplate, AuditEntry, Delegation, Error,
    ExpirationHook, FeeConfig, GlobalStats, HealthStatus, IssuerMetadata, IssuerStats, IssuerTier,
    MultiSigProposal, PendingAdminTransfer, RateLimitConfig, StorageLimits,
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

    pub fn propose_admin_transfer(
        env: Env,
        current_admin: Address,
        new_admin: Address,
    ) -> Result<(), Error> {
        current_admin.require_auth();
        Validation::require_admin(&env, &current_admin)?;
        Storage::set_pending_admin_transfer(
            &env,
            &PendingAdminTransfer {
                proposed_by: current_admin.clone(),
                new_admin: new_admin.clone(),
            },
        );
        Events::admin_transfer_proposed(&env, &current_admin, &new_admin);
        Ok(())
    pub fn propose_admin_transfer(env: Env, current_admin: Address, new_admin: Address) -> Result<(), Error> {
        admin::propose_admin_transfer(&env, current_admin, new_admin)
    }

    pub fn cancel_admin_transfer(env: Env, current_admin: Address) -> Result<(), Error> {
        admin::cancel_admin_transfer(&env, current_admin)
    }

    pub fn accept_admin_transfer(env: Env, new_admin: Address) -> Result<(), Error> {
        admin::accept_admin_transfer(&env, new_admin)
    }

    pub fn add_admin(
        env: Env,
        existing_admin: Address,
        new_admin: Address,
    ) -> Result<(), Error> {
        existing_admin.require_auth();
        Validation::require_admin(&env, &existing_admin)?;
        if Storage::is_admin(&env, &new_admin) {
            return Ok(());
        }
        Storage::add_admin(&env, &new_admin);
        Events::admin_added(&env, &existing_admin, &new_admin, env.ledger().timestamp());
        Ok(())
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
    pub fn get_admin_council(env: Env) -> Result<AdminCouncil, Error> {
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

    pub fn add_to_whitelist(env: Env, issuer: Address, subject: Address) -> Result<(), Error> {
        admin::add_to_whitelist(&env, issuer, subject)
    }

    pub fn bulk_add_to_whitelist(env: Env, issuer: Address, subjects: Vec<Address>) -> Result<(), Error> {
        admin::bulk_add_to_whitelist(&env, issuer, subjects)
    }

    pub fn remove_from_whitelist(env: Env, issuer: Address, subject: Address) -> Result<(), Error> {
        admin::remove_from_whitelist(&env, issuer, subject)
    }

    #[must_use]
    pub fn is_whitelisted(env: Env, issuer: Address, subject: Address) -> bool {
        Storage::is_subject_whitelisted(&env, &issuer, &subject)
        admin::is_whitelisted(&env, issuer, subject)
    }

    #[must_use]
    pub fn is_whitelist_enabled(env: Env, issuer: Address) -> bool {
        admin::is_whitelist_enabled(&env, issuer)
    }

    pub fn set_issuer_tier(
        env: Env,
        admin: Address,
        issuer: Address,
        tier: IssuerTier,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Validation::require_issuer(&env, &issuer)?;
        Storage::set_issuer_tier(&env, &issuer, &tier);
        Events::issuer_tier_updated(&env, &issuer, &tier);
        Ok(())
    }

    /// Return a confidence score (0–100) based on issuer tier + endorsements.
    pub fn get_confidence_score(env: Env, attestation_id: String) -> Option<u32> {
        let attestation = Storage::get_attestation(&env, &attestation_id).ok()?;
        let tier_score = match Storage::get_issuer_tier(&env, &attestation.issuer) {
            Some(IssuerTier::Premium) => 90u32,
            Some(IssuerTier::Verified) => 60u32,
            Some(IssuerTier::Basic) | None => 30u32,
        };
        let endorsements = Storage::get_endorsements(&env, &attestation_id);
        let endorsement_bonus = (endorsements.len() * 2).min(10) as u32;
        Some(tier_score + endorsement_bonus)
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
    pub fn has_valid_claim_from_tier(
        env: Env,
        subject: Address,
        claim_type: String,
        min_tier: IssuerTier,
    ) -> bool {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        let min_rank = min_tier.rank();

        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if attestation.deleted {
                    continue;
                }
                if attestation.claim_type != claim_type {
                    continue;
                }
                if attestation.get_status(current_time) != AttestationStatus::Valid {
                    continue;
                }
                if let Some(tier) = Storage::get_issuer_tier(&env, &attestation.issuer) {
                    if tier.rank() >= min_rank {
                        return true;
                    }
                }
            }
        }
        false
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

    #[must_use]
    pub fn get_bridge_list(env: Env, start: u32, limit: u32) -> Vec<Address> {
        admin::get_bridge_list(&env, start, limit)
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

    /// Set a per-claim-type rate limit override.
    ///
    /// When set, this overrides the global rate limit for the specified claim type.
    /// If not set, the global rate limit applies.
    pub fn set_rate_limit_for_claim_type(
        env: Env,
        admin: Address,
        claim_type: String,
        interval_secs: u64,
    ) -> Result<(), Error> {
        admin::set_rate_limit_for_claim_type(&env, admin, claim_type, interval_secs)
    }

    /// Get the per-claim-type rate limit override for a claim type, or None if not set.
    #[must_use]
    pub fn get_rate_limit_for_claim_type(env: Env, claim_type: String) -> Option<u64> {
        admin::get_rate_limit_for_claim_type(&env, claim_type)
    }

    // -----------------------------------------------------------------------
    // Pause / unpause
    // -----------------------------------------------------------------------

    /// Pause all write operations on the contract.
    ///
    /// The optional `reason` (max 256 characters) is persisted in storage and
    /// included in the emitted `contract_paused` event. This allows operators to
    /// distinguish routine maintenance pauses from emergency security pauses
    /// without maintaining external state.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — caller is not admin.
    /// - [`Error::ReasonTooLong`] — reason exceeds 256 characters.
    pub fn pause(env: Env, admin: Address, reason: Option<String>) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        if let Some(ref r) = reason {
            if r.len() > 256 {
                return Err(Error::ReasonTooLong);
            }
        }
        Storage::set_paused(&env, true);
        Storage::set_pause_reason(&env, &reason);
        Events::contract_paused(&env, &admin, env.ledger().timestamp(), &reason);
        Ok(())
    pub fn pause(env: Env, admin: Address) -> Result<(), Error> {
        admin::pause(&env, admin)
    }

    /// Return the reason stored when `pause()` was last called, or `None`.
    ///
    /// The reason is cleared automatically when `unpause()` is called.
    #[must_use]
    pub fn get_pause_reason(env: Env) -> Option<String> {
        Storage::get_pause_reason(&env)
    }

    pub fn unpause(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::set_paused(&env, false);
        Storage::clear_pause_reason(&env);
        Events::contract_unpaused(&env, &admin, env.ledger().timestamp());
        Ok(())
        admin::unpause(&env, admin)
    }

    #[must_use]
    pub fn is_paused(env: Env) -> bool {
        admin::is_paused(&env)
    }

    // -----------------------------------------------------------------------
    // Attestation creation
    // -----------------------------------------------------------------------

    fn create_attestation_internal(
        env: &Env,
    // Contract Config
    // -----------------------------------------------------------------------

    pub fn set_registered_claim_type(env: Env, admin: Address, require: bool) -> Result<(), Error> {
        admin::set_require_registered_claim_type(&env, admin, require)
    }

    #[must_use]
    pub fn get_registered_claim_type(env: Env) -> bool {
        admin::get_require_registered_claim_type(&env)
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

    pub fn revoke_delegation(env: Env, issuer: Address, delegate: Address, claim_type: String) -> Result<(), Error> {
        admin::revoke_delegation(&env, issuer, delegate, claim_type)
    }

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

    /// Internal: execute the action encoded in a council proposal.
    fn execute_council_action(
        env: &Env,
        action: &CouncilAction,
        proposer: &Address,
    ) -> Result<(), Error> {
        match action {
            CouncilAction::Pause => {
                Storage::set_paused(env, true);
                Events::contract_paused(env, proposer, env.ledger().timestamp(), &None);
            }
            CouncilAction::Unpause => {
                Storage::set_paused(env, false);
                Events::contract_unpaused(env, proposer, env.ledger().timestamp());
            }
            CouncilAction::SetFee(fee_config) => {
                Storage::set_fee_config(env, fee_config);
            }
            CouncilAction::RemoveIssuer(issuer) => {
                Storage::remove_issuer(env, issuer);
                Storage::decrement_total_issuers(env);
                Events::issuer_removed(env, issuer, proposer, env.ledger().timestamp());
            }
        }
        Ok(())
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

        if Storage::is_whitelist_mode(env, &issuer) && !Storage::is_whitelisted(env, &issuer, &subject) {
            return Err(Error::SubjectNotWhitelisted);
        }

        check_rate_limit(env, &issuer)?;

        let limits = Storage::get_limits(env);
        let issuer_count = Storage::get_issuer_attestations(env, &issuer).len();
        if issuer_count >= limits.max_attestations_per_issuer {
            return Err(Error::LimitExceeded);
        }
        let subject_count = Storage::get_subject_attestations(env, &subject).len();
        if subject_count >= limits.max_attestations_per_subject {
            return Err(Error::LimitExceeded);
        }

        let timestamp = env.ledger().timestamp();
        let attestation_id = Attestation::generate_id(env, &issuer, &subject, &claim_type, timestamp);

        if Storage::has_attestation(env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: issuer.clone(),
            subject,
            claim_type,
            timestamp,
            expiration,
            revoked: false,
            deleted: false,
            metadata,
            jurisdiction,
            valid_from: None,
            origin: AttestationOrigin::Native,
            source_chain: None,
            source_tx: None,
            tags,
            revocation_reason: None,
        };

        store_attestation(env, &attestation);
        Storage::append_audit_entry(
            env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Created,
                actor: attestation.issuer.clone(),
                timestamp,
                details: None,
            },
        );
        Storage::set_last_issuance_time(env, &issuer, timestamp);
        charge_attestation_fee(env, &issuer)?;
        Events::attestation_created(env, &attestation);
        Ok(attestation_id)
    }

    pub fn create_attestation(
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
        bridge.require_auth();
        Validation::require_bridge(&env, &bridge)?;
        Validation::require_not_paused(&env)?;
        validate_source_reference(&source_chain, &source_tx)?;

        let timestamp = env.ledger().timestamp();
        let attestation_id = Attestation::generate_bridge_id(
            &env, &bridge, &subject, &claim_type, &source_chain, &source_tx, timestamp,
        );
        if Storage::has_attestation(&env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: bridge,
            subject,
            claim_type,
            timestamp,
            expiration: None,
            revoked: false,
            deleted: false,
            metadata: None,
            jurisdiction: None,
            valid_from: None,
            origin: AttestationOrigin::Bridged,
            source_chain: Some(source_chain),
            source_tx: Some(source_tx),
            tags: None,
            revocation_reason: None,
        };

        store_attestation(&env, &attestation);
        Events::attestation_bridged(&env, &attestation);
        Storage::append_audit_entry(&env, &attestation_id, &AuditEntry {
            action: AuditAction::Created,
            actor: attestation.issuer.clone(),
            timestamp,
            details: None,
        });
        Ok(attestation_id)
        attestation::bridge_attestation(&env, bridge, subject, claim_type, source_chain, source_tx)
    }

    pub fn create_attestations_batch(
        env: Env,
        issuer: Address,
        subjects: Vec<Address>,
        claim_type: String,
        expiration: Option<u64>,
    ) -> Result<Vec<String>, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Validation::require_not_paused(&env)?;
        validate_claim_type(&claim_type)?;
        validate_native_expiration(&env, expiration)?;
        check_rate_limit(&env, &issuer)?;

        let timestamp = env.ledger().timestamp();
        let limits = Storage::get_limits(&env);
        let issuer_count = Storage::get_issuer_attestations(&env, &issuer).len();
        if issuer_count.saturating_add(subjects.len()) > limits.max_attestations_per_issuer {
            return Err(Error::LimitExceeded);
        }

        let mut ids: Vec<String> = Vec::new(&env);

        for subject in subjects.iter() {
            let attestation_id =
                Attestation::generate_id(&env, &issuer, &subject, &claim_type, timestamp);

            if Storage::has_attestation(&env, &attestation_id) {
                return Err(Error::DuplicateAttestation);
            }

            let subject_count = Storage::get_subject_attestations(&env, &subject).len();
            if subject_count >= limits.max_attestations_per_subject {
                return Err(Error::LimitExceeded);
            }

            let attestation = Attestation {
                id: attestation_id.clone(),
                issuer: issuer.clone(),
                subject: subject.clone(),
                claim_type: claim_type.clone(),
                timestamp,
                expiration,
                revoked: false,
                deleted: false,
                metadata: None,
                jurisdiction: None,
                valid_from: None,
                origin: AttestationOrigin::Native,
                source_chain: None,
                source_tx: None,
                tags: None,
                revocation_reason: None,
            };

            store_attestation(&env, &attestation);
            Events::attestation_created(&env, &attestation);
            Storage::append_audit_entry(
                &env,
                &attestation_id,
                &AuditEntry {
                    action: AuditAction::Created,
                    actor: issuer.clone(),
                    timestamp,
                    details: None,
                },
            );
            ids.push_back(attestation_id);
        }

        Storage::set_last_issuance_time(&env, &issuer, timestamp);
        Ok(ids)
    }

    // -----------------------------------------------------------------------
    // Revocation & renewal
    // -----------------------------------------------------------------------

    pub fn revoke_attestation(
        env: Env,
        issuer: Address,
        attestation_id: String,
        reason: Option<String>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_not_paused(&env)?;
        Validation::require_issuer(&env, &issuer)?;
        validate_reason(&reason)?;

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;
        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        attestation.revoked = true;
        attestation.revocation_reason = reason.clone();
        Storage::set_attestation(&env, &attestation);
        Storage::remove_subject_attestation(&env, &attestation.subject, &attestation_id);
        Storage::remove_issuer_attestation(&env, &issuer, &attestation_id);
        Storage::decrement_claim_type_count(&env, &attestation.claim_type);

        Events::attestation_revoked(&env, &attestation_id, &issuer, &reason);
        Storage::append_audit_entry(&env, &attestation_id, &AuditEntry {
            action: AuditAction::Revoked,
            actor: issuer.clone(),
            timestamp: env.ledger().timestamp(),
            details: reason.clone(),
        });
        Storage::increment_total_revocations(&env, 1);
        Ok(())
        attestation::create_attestations_batch(&env, issuer, subjects, claim_type, expiration)
    }

    pub fn revoke_attestation(env: Env, issuer: Address, attestation_id: String, reason: Option<String>) -> Result<(), Error> {
        attestation::revoke_attestation(&env, issuer, attestation_id, reason)
    }

    pub fn renew_attestation(env: Env, issuer: Address, attestation_id: String, new_expiration: Option<u64>) -> Result<(), Error> {
        attestation::renew_attestation(&env, issuer, attestation_id, new_expiration)
    }

    pub fn revoke_attestations_batch(
        env: Env,
        issuer: Address,
        attestation_ids: Vec<String>,
        reason: Option<String>,
    ) -> Result<u32, Error> {
        const MAX_BATCH: u32 = 50;

        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        validate_reason(&reason)?;

        if attestation_ids.len() > MAX_BATCH {
            return Err(Error::LimitExceeded);
        }

        for id in attestation_ids.iter() {
            let attestation = Storage::get_attestation(&env, &id)?;
            if attestation.issuer != issuer {
                return Err(Error::Unauthorized);
            }
            if attestation.revoked {
                return Err(Error::AlreadyRevoked);
            }
        }

        let mut count: u32 = 0;
        for id in attestation_ids.iter() {
            let mut attestation = Storage::get_attestation(&env, &id)?;
            attestation.revoked = true;
            attestation.revocation_reason = reason.clone();
            Storage::set_attestation(&env, &attestation);
            Storage::remove_subject_attestation(&env, &attestation.subject, &id);
            Storage::remove_issuer_attestation(&env, &issuer, &id);
            Storage::decrement_claim_type_count(&env, &attestation.claim_type);
            Events::attestation_revoked_with_reason(&env, &id, &issuer, &reason);
            Storage::append_audit_entry(
                &env,
                &id,
                &AuditEntry {
                    action: AuditAction::Revoked,
                    actor: issuer.clone(),
                    timestamp: env.ledger().timestamp(),
                    details: reason.clone(),
                },
            );
            count += 1;
        }
    pub fn revoke_attestations_batch(env: Env, issuer: Address, attestation_ids: Vec<String>, reason: Option<String>) -> Result<u32, Error> {
        attestation::revoke_attestations_batch(&env, issuer, attestation_ids, reason)
    }

    pub fn update_expiration(env: Env, issuer: Address, attestation_id: String, new_expiration: Option<u64>) -> Result<(), Error> {
        attestation::update_expiration(&env, issuer, attestation_id, new_expiration)
    }

    // -----------------------------------------------------------------------
    // Claim verification
    // -----------------------------------------------------------------------
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

    // -----------------------------------------------------------------------
    // Attestation queries
    // -----------------------------------------------------------------------

    #[must_use]
    pub fn get_attestation(env: Env, attestation_id: String) -> Result<Attestation, Error> {
        query::get_attestation(&env, attestation_id)
    }

    pub fn request_deletion(
        env: Env,
        subject: Address,
        attestation_id: String,
    ) -> Result<(), Error> {
        subject.require_auth();

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;

        if attestation.subject != subject {
            return Err(Error::Unauthorized);
        }

        attestation.deleted = true;
        Storage::set_attestation(&env, &attestation);
        Storage::remove_subject_attestation(&env, &subject, &attestation_id);

        let timestamp = env.ledger().timestamp();
        Events::deletion_requested(&env, &subject, &attestation_id, timestamp);
        Ok(())
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

    /// Cursor-based pagination over a date range. This is the recommended API for
    /// pagination across GDPR deletions or other updates that may remove items from
    /// the subject's attestation index between page requests.
    #[must_use]
    pub fn get_attestations_in_range_after(
        env: Env,
        subject: Address,
        from_ts: u64,
        to_ts: u64,
        after_attestation_id: Option<String>,
        limit: u32,
    ) -> Vec<Attestation> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let mut filtered = Vec::new(&env);
        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if !attestation.deleted
                    && attestation.timestamp >= from_ts
                    && attestation.timestamp <= to_ts
                {
                    filtered.push_back(id);
                }
            }
        }
        let paginated_ids = crate::storage::paginate(&env, &filtered, start, limit);
        let mut result = Vec::new(&env);
        for id in paginated_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                result.push_back(attestation);
            }
        }
        result
        query::get_attestations_in_range_after(&env, subject, from_ts, to_ts, after_attestation_id, limit)
    }

    #[must_use]
    pub fn get_attestations_by_tag(env: Env, subject: Address, tag: String) -> Vec<String> {
        query::get_attestations_by_tag(&env, subject, tag)
    }

    #[must_use]
    pub fn get_attestations_by_jurisdiction(
        env: Env,
        subject: Address,
        jurisdiction: String,
        start: u32,
        limit: u32,
    ) -> Vec<String> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let mut filtered = Vec::new(&env);

        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if attestation.deleted {
                    continue;
                }
                if let Some(att_jurisdiction) = attestation.jurisdiction {
                    if att_jurisdiction == jurisdiction {
                        filtered.push_back(id.clone());
                    }
                }
            }
        }

        crate::storage::paginate(&env, &filtered, start, limit)
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
    pub fn get_attestation_by_type(
        env: Env,
        subject: Address,
        claim_type: String,
    ) -> Option<Attestation> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        let mut index = attestation_ids.len();
        while index > 0 {
            index -= 1;
            if let Some(attestation_id) = attestation_ids.get(index) {
                if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                    if !attestation.deleted
                        && attestation.claim_type == claim_type
                        && attestation.get_status(current_time) == AttestationStatus::Valid
                    {
                        return Some(attestation);
                    }
                }
            }
        }
        None
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

    pub fn set_issuer_metadata(
        env: Env,
        issuer: Address,
        metadata: IssuerMetadata,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::set_issuer_metadata(&env, &issuer, &metadata);
        Ok(())
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

    pub fn register_expiration_hook(
        env: Env,
        subject: Address,
        callback_contract: Address,
        notify_days_before: u32,
    ) -> Result<(), Error> {
        subject.require_auth();
        Storage::set_expiration_hook(
            &env,
            &subject,
            &crate::types::ExpirationHook { callback_contract, notify_days_before },
        );
        Ok(())
    }

    #[must_use]
    pub fn get_expiration_hook(
        env: Env,
        subject: Address,
    ) -> Option<crate::types::ExpirationHook> {
        Storage::get_expiration_hook(&env, &subject)
    }

    pub fn remove_expiration_hook(env: Env, subject: Address) -> Result<(), Error> {
        subject.require_auth();
        Storage::remove_expiration_hook(&env, &subject);
        Ok(())
    }

    pub fn get_fee_config(env: Env) -> Result<FeeConfig, Error> {
        load_fee_config(&env)
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

    pub fn cancel_request(env: Env, subject: Address, request_id: String) -> Result<(), Error> {
        request::cancel_request(&env, subject, request_id)
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
    // Attestation templates (issue #529)
    // -----------------------------------------------------------------------

    /// Enable or disable mandatory claim-type registry validation for templates.
    ///
    /// When `required` is `true`, `create_template` will return
    /// [`Error::ClaimTypeNotRegistered`] if the template's `claim_type` does not
    /// exist in the registry.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — caller is not admin.
    pub fn set_require_registered_claim_type(
        env: Env,
        admin: Address,
        required: bool,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::set_require_registered_claim_type(&env, required);
        Ok(())
    }

    /// Return `true` if `create_template` enforces registered claim types.
    #[must_use]
    pub fn get_require_registered_claim_type(env: Env) -> bool {
        Storage::get_require_registered_claim_type(&env)
    }

    /// Create (or overwrite) an attestation template owned by `issuer`.
    ///
    /// When `require_registered_claim_type` is enabled the `claim_type` field
    /// must already exist in the claim-type registry; otherwise
    /// [`Error::ClaimTypeNotRegistered`] is returned.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    /// - [`Error::InvalidClaimType`] — `claim_type` fails format validation.
    /// - [`Error::ClaimTypeNotRegistered`] — `claim_type` is not in the registry
    ///   and `require_registered_claim_type` is enabled.
    pub fn create_template(
        env: Env,
        issuer: Address,
        name: String,
        claim_type: String,
        default_metadata: Option<String>,
        default_expiration_secs: Option<u64>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Validation::validate_claim_type(&claim_type)?;
        validate_metadata(&env, &default_metadata)?;

        if Storage::get_require_registered_claim_type(&env) {
            if Storage::get_claim_type(&env, &claim_type).is_none() {
                return Err(Error::ClaimTypeNotRegistered);
            }
        }

        let template = AttestationTemplate {
            name: name.clone(),
            issuer: issuer.clone(),
            claim_type,
            default_metadata,
            default_expiration_secs,
        };
        Storage::set_attestation_template(&env, &template);
        Ok(())
    }

    /// Retrieve a template owned by `issuer` with the given `name`, or `None`.
    #[must_use]
    pub fn get_template(
        env: Env,
        issuer: Address,
        name: String,
    ) -> Option<AttestationTemplate> {
        Storage::get_attestation_template(&env, &issuer, &name)
    }

    // -----------------------------------------------------------------------
    // Multi-sig attestation proposals
    // Attestation Templates
    // -----------------------------------------------------------------------

    pub fn propose_attestation(
        env: Env,
        proposer: Address,
        subject: Address,
        claim_type: String,
        required_signers: Vec<Address>,
        threshold: u32,
    ) -> Result<String, Error> {
        proposer.require_auth();
        Validation::require_issuer(&env, &proposer)?;
        Validation::require_not_paused(&env)?;

        let accredited = String::from_str(&env, "ACCREDITED_INVESTOR");
        if claim_type == accredited {
            if let Some(IssuerTier::Premium) = Storage::get_issuer_tier(&env, &proposer) {
                let timestamp = env.ledger().timestamp();
                let attestation_id =
                    Attestation::generate_id(&env, &proposer, &subject, &claim_type, timestamp);
                let attestation = Attestation {
                    id: attestation_id.clone(),
                    issuer: proposer.clone(),
                    subject: subject.clone(),
                    claim_type: claim_type.clone(),
                    timestamp,
                    expiration: None,
                    revoked: false,
                    deleted: false,
                    metadata: None,
                    jurisdiction: None,
                    valid_from: None,
                    origin: AttestationOrigin::Native,
                    source_chain: None,
                    source_tx: None,
                    tags: None,
                    revocation_reason: None,
                };
                store_attestation(&env, &attestation);
                Events::attestation_created(&env, &attestation);
                return Ok(attestation_id);
            }
        }

        for signer in required_signers.iter() {
            Validation::require_issuer(&env, &signer)?;
        }
        let signer_count = required_signers.len();
        if threshold == 0 || threshold > signer_count {
            return Err(Error::InvalidThreshold);
        }
        let timestamp = env.ledger().timestamp();
        let proposal_id = MultiSigProposal::generate_id(&env, &proposer, &subject, &claim_type, timestamp);
        let mut signers = Vec::new(&env);
        signers.push_back(proposer.clone());
        let proposal = MultiSigProposal {
            id: proposal_id.clone(),
            proposer: proposer.clone(),
            subject: subject.clone(),
            claim_type,
            required_signers,
            threshold,
            signers,
            created_at: timestamp,
            expires_at: timestamp + MULTISIG_PROPOSAL_TTL_SECS,
            finalized: false,
        };
        Storage::set_multisig_proposal(&env, &proposal);
        Events::multisig_proposed(&env, &proposal_id, &proposer, &subject, threshold);
        Ok(proposal_id)
    }

    pub fn cosign_attestation(env: Env, issuer: Address, proposal_id: String) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Validation::require_not_paused(&env)?;

        let mut proposal = Storage::get_multisig_proposal(&env, &proposal_id)?;
        if proposal.finalized { return Err(Error::ProposalFinalized); }
        let current_time = env.ledger().timestamp();
        if current_time >= proposal.expires_at { return Err(Error::ProposalExpired); }

        let mut is_required = false;
        for signer in proposal.required_signers.iter() {
            if signer == issuer { is_required = true; break; }
        }
        if !is_required { return Err(Error::NotRequiredSigner); }

        for signer in proposal.signers.iter() {
            if signer == issuer { return Err(Error::AlreadySigned); }
        }

        proposal.signers.push_back(issuer.clone());
        let sig_count = proposal.signers.len();
        Events::multisig_cosigned(&env, &proposal_id, &issuer, sig_count, proposal.threshold);

        if sig_count >= proposal.threshold {
            proposal.finalized = true;
            Storage::set_multisig_proposal(&env, &proposal);

            let attestation_id = Attestation::generate_id(
                &env, &proposal.proposer, &proposal.subject, &proposal.claim_type, proposal.created_at,
            );
            let attestation = Attestation {
                id: attestation_id.clone(),
                issuer: proposal.proposer.clone(),
                subject: proposal.subject.clone(),
                claim_type: proposal.claim_type.clone(),
                timestamp: proposal.created_at,
                expiration: None,
                revoked: false,
                deleted: false,
                metadata: None,
                jurisdiction: None,
                valid_from: None,
                origin: AttestationOrigin::Native,
                source_chain: None,
                source_tx: None,
                tags: None,
                revocation_reason: None,
            };

            store_attestation(&env, &attestation);
            Events::attestation_created(&env, &attestation);
            Events::multisig_activated(&env, &proposal_id, &attestation_id);
        } else {
            Storage::set_multisig_proposal(&env, &proposal);
        }
        Ok(())
    }

    #[must_use]
    pub fn get_multisig_proposal(env: Env, proposal_id: String) -> Result<MultiSigProposal, Error> {
        Storage::get_multisig_proposal(&env, &proposal_id)
    }

    // -----------------------------------------------------------------------
    // Delegation
    // -----------------------------------------------------------------------

    pub fn delegate_claim_type(
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
        if issuer == delegate {
            return Err(Error::CannotDelegateToSelf);
        }
        validate_native_expiration(&env, expiration)?;
        let delegation = Delegation {
            delegator: issuer.clone(),
            delegate: delegate.clone(),
            claim_type: claim_type.clone(),
            expiration,
        };
        Storage::set_delegation(&env, &delegation);
        Events::delegation_created(&env, &issuer, &delegate, &claim_type, expiration);
        Ok(())
    }

    #[must_use]
    pub fn get_multisig_ttl(env: Env) -> u32 {
        Storage::get_multisig_ttl_days(&env)
    }

    // -----------------------------------------------------------------------
    // Contract configuration & stats
    // -----------------------------------------------------------------------

    #[must_use]
    pub fn get_limits(env: Env) -> StorageLimits {
        Storage::get_limits(&env)
    }

    #[must_use]
    pub fn get_version(env: Env) -> Result<String, Error> {
        Storage::get_version(&env).ok_or(Error::NotInitialized)
    }

    #[must_use]
    pub fn get_global_stats(env: Env) -> GlobalStats {
        Storage::get_global_stats(&env)
    }

    #[must_use]
    pub fn health_check(env: Env) -> HealthStatus {
        let initialized = Storage::has_admin(&env);
        let stats = Storage::get_global_stats(&env);
        HealthStatus {
            initialized,
            admin_set: initialized,
            issuer_count: stats.total_issuers,
            total_attestations: stats.total_attestations,
        }
    }

    pub fn get_contract_metadata(env: Env) -> Result<ContractMetadata, Error> {
        let version = Storage::get_version(&env).ok_or(Error::NotInitialized)?;
        Ok(ContractMetadata {
            name: String::from_str(&env, "TrustLink"),
            version,
            description: String::from_str(&env, "On-chain attestation and verification system for the Stellar blockchain."),
        })
    }

    #[must_use]
    pub fn get_config(env: Env) -> ContractConfig {
        let ttl_config = Storage::get_ttl_config(&env).unwrap_or(TtlConfig { ttl_days: 30 });
        let fee_config = Storage::get_fee_config(&env).unwrap_or(FeeConfig {
            attestation_fee: 0,
            fee_collector: env.current_contract_address(),
            fee_token: None,
        });
        let version = Storage::get_version(&env).unwrap_or(String::from_str(&env, ""));
        ContractConfig {
            contract_name: String::from_str(&env, "TrustLink"),
            contract_version: version,
            contract_description: String::from_str(
                &env,
                "On-chain attestation and verification system for the Stellar blockchain.",
            ),
            fee_config,
            ttl_config,
        }
    }

    // -----------------------------------------------------------------------
    // Council Quorum
    // -----------------------------------------------------------------------

    pub fn init_council(
        env: Env,
        admin: Address,
        members: Vec<Address>,
        quorum: u32,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;

        if quorum == 0 || quorum > members.len() {
            return Err(Error::InvalidThreshold);
        }

        let member_count = members.len();
        Storage::set_council(&env, &members);
        Events::council_initialized(&env, quorum, member_count);
        Ok(())
    }

    pub fn propose_council_action(
        env: Env,
        proposer: Address,
        operation: CouncilOperation,
    ) -> Result<u32, Error> {
        proposer.require_auth();

        let council = Storage::get_council(&env).ok_or(Error::NotInitialized)?;

        let mut is_member = false;
        for m in council.iter() {
            if m == proposer {
                is_member = true;
                break;
            }
        }
        if !is_member {
            return Err(Error::Unauthorized);
        }

        let id = Storage::next_proposal_id(&env);
        let mut approvals: Vec<Address> = Vec::new(&env);
        approvals.push_back(proposer.clone());

        let proposal = CouncilProposal {
            id,
            operation,
            proposer: proposer.clone(),
            approvals,
            executed: false,
        };

        Storage::set_proposal(&env, &proposal);
        Events::proposal_created(&env, id, &proposer);
        Ok(id)
    }

    pub fn approve_council_action(
        env: Env,
        approver: Address,
        proposal_id: u32,
    ) -> Result<(), Error> {
        approver.require_auth();

        let council = Storage::get_council(&env).ok_or(Error::NotInitialized)?;
        let mut proposal = Storage::get_proposal(&env, proposal_id).ok_or(Error::NotFound)?;

        if proposal.executed {
            return Err(Error::AlreadyRevoked);
        }

        let mut is_member = false;
        for m in council.iter() {
            if m == approver {
                is_member = true;
                break;
            }
        }
        if !is_member {
            return Err(Error::Unauthorized);
        }

        for a in proposal.approvals.iter() {
            if a == approver {
                return Err(Error::AlreadySigned);
            }
        }

        proposal.approvals.push_back(approver.clone());
        Storage::set_proposal(&env, &proposal);
        Events::proposal_approved(&env, proposal_id, &approver);
        Ok(())
    }

    pub fn execute_council_action(
        env: Env,
        executor: Address,
        proposal_id: u32,
    ) -> Result<(), Error> {
        executor.require_auth();

        let council = Storage::get_council(&env).ok_or(Error::NotInitialized)?;
        let mut proposal = Storage::get_proposal(&env, proposal_id).ok_or(Error::NotFound)?;

        if proposal.executed {
            return Err(Error::AlreadyRevoked);
        }

        let mut is_member = false;
        for m in council.iter() {
            if m == executor {
                is_member = true;
                break;
            }
        }
        if !is_member {
            return Err(Error::Unauthorized);
        }

        if proposal.approvals.len() < 1 {
            return Err(Error::Unauthorized);
        }

        match proposal.operation.clone() {
            CouncilOperation::RemoveIssuer(issuer) => {
                Storage::remove_issuer(&env, &issuer);
                let ts = env.ledger().timestamp();
                if let Some(first) = council.get(0) {
                    Events::issuer_removed(&env, &issuer, &first, ts);
                }
            }
            CouncilOperation::PauseContract => {
                Storage::set_paused(&env, true);
            }
        }

        proposal.executed = true;
        Storage::set_proposal(&env, &proposal);
        Events::proposal_executed(&env, proposal_id);
        Ok(())
    }

    pub fn get_council(env: Env) -> Option<AdminCouncil> {
        Storage::get_council(&env)
    }

    pub fn get_council_proposal(env: Env, proposal_id: u32) -> Option<CouncilProposal> {
        Storage::get_proposal(&env, proposal_id)
    }

    // -----------------------------------------------------------------------
    // Attestation Request Workflow
    // -----------------------------------------------------------------------

    pub fn request_attestation(
        Validation::validate_claim_type(&template.claim_type)?;
        Validation::validate_metadata(&env, &template.metadata_template)?;

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
        claim_type: String,
    ) -> Result<String, Error> {
        subject.require_auth();
        Validation::require_not_paused(&env)?;
        Validation::require_issuer(&env, &issuer)?;
        validate_claim_type(&claim_type)?;

        let timestamp = env.ledger().timestamp();
        let request_id =
            AttestationRequest::generate_id(&env, &subject, &issuer, &claim_type, timestamp);

        if Storage::get_request(&env, &request_id).is_ok() {
            return Err(Error::DuplicateAttestation);
        }

        let expires_at = timestamp + ATTESTATION_REQUEST_TTL_SECS;

        let request = AttestationRequest {
            id: request_id.clone(),
            subject: subject.clone(),
            issuer: issuer.clone(),
            claim_type: claim_type.clone(),
            timestamp,
            expires_at,
            status: RequestStatus::Pending,
            rejection_reason: None,
        };

        Storage::set_request(&env, &request);
        Storage::add_pending_request(&env, &issuer, &request_id);
        Events::attestation_requested(
            &env,
            &request_id,
            &subject,
            &issuer,
            &claim_type,
            expires_at,
        );

        Ok(request_id)
    }

    pub fn fulfill_request(
        env: Env,
        issuer: Address,
        request_id: String,
        template_id: String,
        subject: Address,
        expiration_override: Option<u64>,
        metadata_override: Option<String>,
    ) -> Result<String, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;

        let template = Storage::get_template(&env, &issuer, &template_id)
            .ok_or(Error::NotFound)?;

        if request.issuer != issuer {
            return Err(Error::Unauthorized);
        }

        match request.status {
            RequestStatus::Fulfilled => return Err(Error::ProposalFinalized),
            RequestStatus::Rejected => return Err(Error::AlreadyRevoked),
            RequestStatus::Pending => {}
        }

        let current_time = env.ledger().timestamp();
        if current_time >= request.expires_at {
            return Err(Error::Expired);
        }

        let attestation_id = Attestation::generate_id(
            &env,
            &issuer,
            &request.subject,
            &request.claim_type,
            current_time,
        );

        if Storage::has_attestation(&env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }

        let limits = Storage::get_limits(&env);
        let issuer_count = Storage::get_issuer_attestations(&env, &issuer).len();
        if issuer_count >= limits.max_attestations_per_issuer {
            return Err(Error::LimitExceeded);
        }
        let subject_count =
            Storage::get_subject_attestations(&env, &request.subject).len();
        if subject_count >= limits.max_attestations_per_subject {
            return Err(Error::LimitExceeded);
        }

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: issuer.clone(),
            subject: request.subject.clone(),
            claim_type: request.claim_type.clone(),
            timestamp: current_time,
            expiration: None,
            revoked: false,
            metadata: None,
            jurisdiction: None,
            valid_from: None,
            origin: AttestationOrigin::Native,
            source_chain: None,
            source_tx: None,
            tags: None,
            revocation_reason: None,
            deleted: false,
        };

        store_attestation(&env, &attestation);
        Events::attestation_created(&env, &attestation);

        request.status = RequestStatus::Fulfilled;
        Storage::set_request(&env, &request);
        Storage::remove_pending_request(&env, &issuer, &request_id);

        Events::request_fulfilled(&env, &request_id, &issuer, &attestation_id);
        // Validate overrides before resolving.
        Validation::validate_metadata(&env, &metadata_override)?;

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
        attestation::create_attestation_internal(
            &env,
            issuer,
            subject,
            template.claim_type,
            expiration,
            metadata,
            None,
            None,
            None,
        )
    }

    pub fn reject_request(
        env: Env,
        issuer: Address,
        request_id: String,
        reason: Option<String>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_not_paused(&env)?;
        Validation::require_issuer(&env, &issuer)?;
        validate_reason(&reason)?;

        let mut request = Storage::get_request(&env, &request_id)?;

        if request.issuer != issuer {
            return Err(Error::Unauthorized);
        }

        match request.status {
            RequestStatus::Fulfilled => return Err(Error::ProposalFinalized),
            RequestStatus::Rejected => return Err(Error::AlreadyRevoked),
            RequestStatus::Pending => {}
        }

        let current_time = env.ledger().timestamp();
        if current_time >= request.expires_at {
            return Err(Error::Expired);
        }

        request.status = RequestStatus::Rejected;
        request.rejection_reason = reason.clone();
        Storage::set_request(&env, &request);
        Storage::remove_pending_request(&env, &issuer, &request_id);

        Events::request_rejected(&env, &request_id, &issuer, &reason);

        Ok(())
    }

    pub fn get_pending_requests(
        env: Env,
        issuer: Address,
        start: u32,
        limit: u32,
    ) -> Vec<AttestationRequest> {
        let current_time = env.ledger().timestamp();
        let all_ids = Storage::get_pending_request_ids(&env, &issuer);
        let mut pending = Vec::new(&env);

        for id in all_ids.iter() {
            if let Ok(req) = Storage::get_request(&env, &id) {
                if req.status == RequestStatus::Pending && current_time < req.expires_at {
                    pending.push_back(req);
                }
            }
        }

        let total = pending.len();
        let start = start.min(total);
        let end = (start + limit).min(total);
        let mut result = Vec::new(&env);
        let mut i = start;
        while i < end {
            if let Some(req) = pending.get(i) {
                result.push_back(req);
            }
            i += 1;
        }
        result
    }

    pub fn get_request(env: Env, request_id: String) -> Result<AttestationRequest, Error> {
        Storage::get_request(&env, &request_id)
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

    // -----------------------------------------------------------------------
    // Issue #530: Template management
    // -----------------------------------------------------------------------

    /// Save (create or overwrite) an attestation template for the calling issuer.
    pub fn save_template(
        env: Env,
        issuer: Address,
        template_id: String,
        claim_type: String,
        metadata: Option<String>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Validation::validate_claim_type(&claim_type)?;
        validate_metadata(&env, &metadata)?;

        let template = AttestationTemplate {
            issuer: issuer.clone(),
            template_id: template_id.clone(),
            claim_type,
            metadata,
        };
        Storage::set_template(&env, &template);
        Ok(())
    }

    /// Return the template with `template_id` owned by `issuer`, or `NotFound`.
    pub fn get_template(
        env: Env,
        issuer: Address,
        template_id: String,
    ) -> Result<AttestationTemplate, Error> {
        Storage::get_template(&env, &issuer, &template_id)
    }

    /// Delete a template. Only the issuer who created it may call this.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — template does not exist.
    /// - [`Error::Unauthorized`] — caller is not the template's issuer.
    pub fn delete_template(
        env: Env,
        issuer: Address,
        template_id: String,
    ) -> Result<(), Error> {
        issuer.require_auth();
        let template = Storage::get_template(&env, &issuer, &template_id)?;
        if template.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        Storage::remove_template(&env, &issuer, &template_id);
        Events::template_deleted(&env, &issuer, &template_id);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Issue #532: Attestation analytics
    // -----------------------------------------------------------------------

    /// Return the total number of active (non-revoked) attestations for a claim type.
    #[must_use]
    pub fn get_claim_type_count(env: Env, claim_type: String) -> u64 {
        Storage::get_claim_type_count(&env, &claim_type)
    }
}
