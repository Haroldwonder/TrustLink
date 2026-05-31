use soroban_sdk::{token::TokenClient, Address, Env, String, Vec};

use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    AdminCouncil, ClaimTypeInfo, ContractConfig, Delegation, Error, ExpirationHook, FeeConfig, IssuerMetadata,
    IssuerStats, IssuerTier, PendingAdminTransfer, RateLimitConfig, StorageLimits, TtlConfig,
};
use crate::validation::Validation;

// -----------------------------------------------------------------------
// Initialization & Admin
// -----------------------------------------------------------------------

pub fn initialize(env: &Env, admin: Address, ttl_days: Option<u32>) -> Result<(), Error> {
    admin.require_auth();
    if Storage::has_admin(env) {
        return Err(Error::AlreadyInitialized);
    }
    let mut council: AdminCouncil = Vec::new(env);
    council.push_back(admin.clone());
    Storage::set_admin_council(env, &council);
    Storage::set_version(env, &String::from_str(env, "1.0.0"));
    Storage::set_fee_config(env, &FeeConfig { attestation_fee: 0, fee_collector: admin.clone(), fee_token: None });
    let days = ttl_days.unwrap_or(30);
    Storage::set_ttl_config(env, &TtlConfig { ttl_days: days });
    Events::admin_initialized(env, &admin, env.ledger().timestamp());
    Ok(())
}

pub fn transfer_admin(env: &Env, current_admin: Address, new_admin: Address) -> Result<(), Error> {
    current_admin.require_auth();
    Validation::require_admin(env, &current_admin)?;
    Storage::add_admin(env, &new_admin);
    Storage::remove_admin(env, &current_admin);
    Events::admin_transferred(env, &current_admin, &new_admin);
    Ok(())
}

pub fn propose_admin_transfer(env: &Env, current_admin: Address, new_admin: Address) -> Result<(), Error> {
    current_admin.require_auth();
    Validation::require_admin(env, &current_admin)?;
    Storage::set_pending_admin_transfer(
        env,
        &PendingAdminTransfer { proposed_by: current_admin.clone(), new_admin: new_admin.clone() },
    );
    Events::admin_transfer_proposed(env, &current_admin, &new_admin);
    Ok(())
}

pub fn cancel_admin_transfer(env: &Env, current_admin: Address) -> Result<(), Error> {
    current_admin.require_auth();
    Validation::require_admin(env, &current_admin)?;
    let pending = Storage::get_pending_admin_transfer(env).ok_or(Error::NotFound)?;
    if pending.proposed_by != current_admin {
        return Err(Error::Unauthorized);
    }
    Storage::remove_pending_admin_transfer(env);
    Ok(())
}

pub fn accept_admin_transfer(env: &Env, new_admin: Address) -> Result<(), Error> {
    new_admin.require_auth();
    let pending = Storage::get_pending_admin_transfer(env).ok_or(Error::NotFound)?;
    if pending.new_admin != new_admin {
        return Err(Error::Unauthorized);
    }
    Storage::add_admin(env, &new_admin);
    Storage::remove_admin(env, &pending.proposed_by);
    Storage::remove_pending_admin_transfer(env);
    Events::admin_transferred(env, &pending.proposed_by, &new_admin);
    Ok(())
}

pub fn get_pending_admin_transfer(env: &Env) -> Option<PendingAdminTransfer> {
    Storage::get_pending_admin_transfer(env)
}

pub fn add_admin(env: &Env, existing_admin: Address, new_admin: Address) -> Result<(), Error> {
    existing_admin.require_auth();
    Validation::require_admin(env, &existing_admin)?;
    if Storage::is_admin(env, &new_admin) {
        return Ok(());
    }
    Storage::add_admin(env, &new_admin);
    Events::admin_added(env, &existing_admin, &new_admin, env.ledger().timestamp());
    Ok(())
}

pub fn remove_admin(env: &Env, existing_admin: Address, admin_to_remove: Address) -> Result<(), Error> {
    existing_admin.require_auth();
    Validation::require_admin(env, &existing_admin)?;
    let council = Storage::get_admin_council(env)?;
    if council.len() <= 1 {
        return Err(Error::LastAdminCannotBeRemoved);
    }
    if !Storage::is_admin(env, &admin_to_remove) {
        return Ok(());
    }
    Storage::remove_admin(env, &admin_to_remove);
    Events::admin_removed(env, &existing_admin, &admin_to_remove, env.ledger().timestamp());
    Ok(())
}

pub fn get_admin(env: &Env) -> Result<Address, Error> {
    Storage::get_admin(env)
}

pub fn get_admin_council(env: &Env) -> Result<AdminCouncil, Error> {
    Storage::get_admin_council(env)
}

// -----------------------------------------------------------------------
// Issuer management
// -----------------------------------------------------------------------

pub fn register_issuer(env: &Env, admin: Address, issuer: Address) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Validation::require_not_paused(env)?;
    if Storage::is_bridge(env, &issuer) {
        return Err(Error::Unauthorized);
    }
    Storage::add_issuer(env, &issuer);
    Storage::increment_total_issuers(env);
    Events::issuer_registered(env, &issuer, &admin, env.ledger().timestamp());
    Ok(())
}

pub fn remove_issuer(env: &Env, admin: Address, issuer: Address) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Storage::remove_issuer(env, &issuer);
    Storage::decrement_total_issuers(env);
    Events::issuer_removed(env, &issuer, &admin, env.ledger().timestamp());
    Ok(())
}

pub fn get_issuer_list(env: &Env, start: u32, limit: u32) -> Vec<Address> {
    crate::storage::paginate_addresses(env, &Storage::get_issuer_list(env), start, limit)
}

pub fn add_to_whitelist(env: &Env, issuer: Address, subject: Address) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Storage::add_to_whitelist(env, &issuer, &subject);
    Ok(())
}

pub fn bulk_add_to_whitelist(env: &Env, issuer: Address, subjects: Vec<Address>) -> Result<(), Error> {
    const MAX_BATCH: u32 = 50;
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    if subjects.len() > MAX_BATCH {
        return Err(Error::LimitExceeded);
    }
    for subject in subjects.iter() {
        Storage::add_to_whitelist(env, &issuer, &subject);
    }
    Ok(())
}

pub fn remove_from_whitelist(env: &Env, issuer: Address, subject: Address) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Storage::remove_from_whitelist(env, &issuer, &subject);
    Ok(())
}

pub fn is_whitelisted(env: &Env, issuer: Address, subject: Address) -> bool {
    Storage::is_whitelisted(env, &issuer, &subject)
}

pub fn is_whitelist_enabled(env: &Env, issuer: Address) -> bool {
    Storage::is_whitelist_enabled(env, &issuer)
}

pub fn set_issuer_tier(env: &Env, admin: Address, issuer: Address, tier: IssuerTier) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Validation::require_issuer(env, &issuer)?;
    Storage::set_issuer_tier(env, &issuer, &tier);
    Events::issuer_tier_updated(env, &issuer, &tier);
    Ok(())
}

pub fn get_confidence_score(env: &Env, attestation_id: String) -> Option<u32> {
    let attestation = Storage::get_attestation(env, &attestation_id).ok()?;
    let tier_score = match Storage::get_issuer_tier(env, &attestation.issuer) {
        Some(IssuerTier::Premium) => 90u32,
        Some(IssuerTier::Verified) => 60u32,
        Some(IssuerTier::Basic) | None => 30u32,
    };
    let endorsements = Storage::get_endorsements(env, &attestation_id);
    let endorsement_score = (endorsements.len() * 2).min(10);
    Some(tier_score + endorsement_score)
}

pub fn get_issuer_metadata(env: &Env, issuer: Address) -> Option<IssuerMetadata> {
    Storage::get_issuer_metadata(env, &issuer)
}

pub fn set_issuer_metadata(env: &Env, issuer: Address, metadata: IssuerMetadata) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Storage::set_issuer_metadata(env, &issuer, &metadata);
    Ok(())
}

pub fn get_issuer_stats(env: &Env, issuer: Address) -> IssuerStats {
    Storage::get_issuer_stats(env, &issuer)
}

pub fn is_issuer(env: &Env, address: Address) -> bool {
    Storage::is_issuer(env, &address)
}

pub fn get_issuer_tier(env: &Env, issuer: Address) -> Option<IssuerTier> {
    Storage::get_issuer_tier(env, &issuer)
}

// -----------------------------------------------------------------------
// Bridge management
// -----------------------------------------------------------------------

pub fn register_bridge(env: &Env, admin: Address, bridge_contract: Address) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    if Storage::is_issuer(env, &bridge_contract) {
        return Err(Error::Unauthorized);
    }
    Storage::add_bridge(env, &bridge_contract);
    Ok(())
}

pub fn is_bridge(env: &Env, address: Address) -> bool {
    Storage::is_bridge(env, &address)
}

pub fn get_bridge_list(env: &Env, start: u32, limit: u32) -> Vec<Address> {
    crate::storage::paginate_addresses(env, &Storage::get_bridge_list(env), start, limit)
}

// -----------------------------------------------------------------------
// Whitelist mode
// -----------------------------------------------------------------------

pub fn set_whitelist_enabled(env: &Env, issuer: Address, enabled: bool) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Storage::set_whitelist_enabled(env, &issuer, enabled);
    Ok(())
}

pub fn enable_whitelist_mode(env: &Env, issuer: Address) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Storage::set_whitelist_mode(env, &issuer, true);
    Events::whitelist_mode_enabled(env, &issuer);
    Ok(())
}

// -----------------------------------------------------------------------
// Fee & rate limit
// -----------------------------------------------------------------------

pub fn get_fee_config(env: &Env) -> Result<FeeConfig, Error> {
    Storage::get_fee_config(env).ok_or(Error::NotInitialized)
}

pub fn set_fee(env: &Env, admin: Address, fee: i128, collector: Address, fee_token: Option<Address>) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    validate_fee_config(env, fee, &fee_token)?;
    if admin == collector {
        return Err(Error::Unauthorized);
    }
    Storage::set_fee_config(env, &FeeConfig { attestation_fee: fee, fee_collector: collector, fee_token });
    Ok(())
}

pub fn set_rate_limit(env: &Env, admin: Address, min_issuance_interval: u64) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Storage::set_rate_limit_config(env, &RateLimitConfig { min_issuance_interval });
    Ok(())
}

pub fn get_rate_limit(env: &Env) -> Option<RateLimitConfig> {
    Storage::get_rate_limit_config(env)
}

/// Set a per-claim-type rate limit override.
///
/// When set, this overrides the global rate limit for the specified claim type.
/// If not set, the global rate limit applies.
///
/// # Errors
/// - [`Error::Unauthorized`] — caller is not an admin.
pub fn set_rate_limit_for_claim_type(
    env: &Env,
    admin: Address,
    claim_type: String,
    interval_secs: u64,
) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Validation::validate_claim_type(&claim_type)?;
    Storage::set_claim_type_rate_limit(env, &claim_type, interval_secs);
    Ok(())
}

/// Get the per-claim-type rate limit override for a claim type, or None if not set.
pub fn get_rate_limit_for_claim_type(env: &Env, claim_type: String) -> Option<u64> {
    Storage::get_claim_type_rate_limit(env, &claim_type)
}

// -----------------------------------------------------------------------
// Pause / unpause
// -----------------------------------------------------------------------

pub fn pause(env: &Env, admin: Address) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Storage::set_paused(env, true);
    Events::contract_paused(env, &admin, env.ledger().timestamp());
    Ok(())
}

pub fn unpause(env: &Env, admin: Address) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Storage::set_paused(env, false);
    Events::contract_unpaused(env, &admin, env.ledger().timestamp());
    Ok(())
}

pub fn is_paused(env: &Env) -> bool {
    Storage::is_paused(env)
}

// -----------------------------------------------------------------------
// Contract Config
// -----------------------------------------------------------------------

pub fn set_require_registered_claim_type(env: &Env, admin: Address, require: bool) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    
    let mut config = Storage::get_contract_config(env).unwrap_or_else(|| {
        ContractConfig {
            ttl_config: Storage::get_ttl_config(env).unwrap_or(TtlConfig { ttl_days: 30 }),
            limits: Storage::get_limits(env),
            fee_config: Storage::get_fee_config(env).unwrap_or(FeeConfig {
                attestation_fee: 0,
                fee_collector: admin.clone(),
                fee_token: None,
            }),
            require_registered_claim_type: false,
        }
    });
    
    config.require_registered_claim_type = require;
    Storage::set_contract_config(env, &config);
    Ok(())
}

pub fn get_require_registered_claim_type(env: &Env) -> bool {
    Storage::get_contract_config(env)
        .map(|config| config.require_registered_claim_type)
        .unwrap_or(false)
}

// -----------------------------------------------------------------------
// Limits
// -----------------------------------------------------------------------

pub fn get_limits(env: &Env) -> StorageLimits {
    Storage::get_limits(env)
}

pub fn set_limits(env: &Env, admin: Address, max_attestations_per_issuer: u32, max_attestations_per_subject: u32) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Storage::set_limits(env, &StorageLimits { max_attestations_per_issuer, max_attestations_per_subject });
    Ok(())
}

// -----------------------------------------------------------------------
// Claim type registry
// -----------------------------------------------------------------------

pub fn register_claim_type(env: &Env, admin: Address, claim_type: String, description: String) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Validation::validate_claim_type(&claim_type)?;
    let info = ClaimTypeInfo { claim_type: claim_type.clone(), description: description.clone() };
    Storage::set_claim_type(env, &info);
    Events::claim_type_registered(env, &claim_type, &description);
    Ok(())
}

pub fn get_claim_type_description(env: &Env, claim_type: String) -> Option<String> {
    Storage::get_claim_type(env, &claim_type).map(|info| info.description)
}

pub fn list_claim_types(env: &Env, start: u32, limit: u32) -> Vec<String> {
    crate::storage::paginate(env, &Storage::get_claim_type_list(env), start, limit)
}

// -----------------------------------------------------------------------
// Delegation
// -----------------------------------------------------------------------

pub fn delegate_claim_type(
    env: &Env,
    issuer: Address,
    delegate: Address,
    claim_type: String,
    expiration: Option<u64>,
) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    if issuer == delegate {
        return Err(Error::CannotDelegateToSelf);
    }
    crate::attestation::validate_native_expiration(env, expiration)?;
    let delegation = Delegation {
        delegator: issuer.clone(),
        delegate: delegate.clone(),
        claim_type: claim_type.clone(),
        expiration,
    };
    Storage::set_delegation(env, &delegation);
    Events::delegation_created(env, &issuer, &delegate, &claim_type, expiration);
    Ok(())
}

pub fn revoke_delegation(
    env: &Env,
    issuer: Address,
    delegate: Address,
    claim_type: String,
) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    if Storage::get_delegation(env, &issuer, &delegate, &claim_type).is_none() {
        return Err(Error::NotFound);
    }
    Storage::remove_delegation(env, &issuer, &delegate, &claim_type);
    Events::delegation_revoked(env, &issuer, &delegate, &claim_type);
    Ok(())
}

pub fn list_delegations_by_delegator(
    env: &Env,
    delegator: Address,
    start: u32,
    limit: u32,
) -> Vec<Delegation> {
    let current_time = env.ledger().timestamp();
    let index = Storage::get_delegator_index(env, &delegator);
    let mut result = Vec::new(env);
    let mut count: u32 = 0;
    let mut skipped: u32 = 0;
    for (delegate, claim_type) in index.iter() {
        if let Some(d) = Storage::get_delegation(env, &delegator, &delegate, &claim_type) {
            // Only include non-expired delegations
            if d.expiration.map_or(true, |exp| current_time < exp) {
                if skipped < start {
                    skipped += 1;
                    continue;
                }
                if count >= limit {
                    break;
                }
                result.push_back(d);
                count += 1;
            }
        }
    }
    result
}

// -----------------------------------------------------------------------
// Expiration hooks
// -----------------------------------------------------------------------

pub fn register_expiration_hook(env: &Env, subject: Address, callback_contract: Address, notify_days_before: u32) -> Result<(), Error> {
    if notify_days_before == 0 {
        return Err(Error::InvalidExpiration);
    }
    subject.require_auth();
    Storage::set_expiration_hook(env, &subject, &ExpirationHook { callback_contract, notify_days_before });
    Ok(())
}

pub fn get_expiration_hook(env: &Env, subject: Address) -> Option<ExpirationHook> {
    Storage::get_expiration_hook(env, &subject)
}

pub fn remove_expiration_hook(env: &Env, subject: Address) -> Result<(), Error> {
    subject.require_auth();
    Storage::remove_expiration_hook(env, &subject);
    Ok(())
}

// -----------------------------------------------------------------------
// Misc
// -----------------------------------------------------------------------

pub fn get_version(env: &Env) -> Result<String, Error> {
    Storage::get_version(env).ok_or(Error::NotInitialized)
}

pub fn health_check(env: &Env) -> crate::types::HealthStatus {
    let initialized = Storage::has_admin(env);
    let stats = Storage::get_global_stats(env);
    crate::types::HealthStatus {
        initialized,
        admin_set: initialized,
        issuer_count: stats.total_issuers,
        total_attestations: stats.total_attestations,
    }
}

// -----------------------------------------------------------------------
// Internal helpers
// -----------------------------------------------------------------------

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
