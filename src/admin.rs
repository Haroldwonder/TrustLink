use soroban_sdk::{token::TokenClient, Address, Env, String, Vec};

use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    AdminCouncil, AuditAction, AuditEntry, ClaimTypeInfo, ContractConfig, CouncilOperation,
    CouncilProposal, DecayConfig, Delegation, DisputeRecord, DisputeStatus, Error,
    ExpirationHook, FeeConfig, IssuerMetadata, IssuerStats, IssuerTier, PendingAdminTransfer,
    RateLimitConfig, StorageLimits, TtlConfig,
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
    let days = ttl_days.unwrap_or(crate::constants::DEFAULT_TTL_DAYS);
    if days == 0 || days > crate::constants::MAX_TTL_DAYS {
        return Err(Error::InvalidTtl);
    }
    let mut council: AdminCouncil = Vec::new(env);
    council.push_back(admin.clone());
    Storage::set_admin_council(env, &council);
    Storage::set_version(env, &String::from_str(env, env!("CARGO_PKG_VERSION")));
    Storage::set_fee_config(env, &FeeConfig { attestation_fee: 0, fee_collector: admin.clone(), fee_token: None });
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
    Validation::require_not_paused(env)?;
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
    Validation::require_not_paused(env)?;
    Storage::add_to_whitelist(env, &issuer, &subject);
    Events::whitelist_updated(env, &issuer, &subject, true);
    Ok(())
}

pub fn bulk_add_to_whitelist(env: &Env, issuer: Address, subjects: Vec<Address>) -> Result<(), Error> {
    const MAX_BATCH: u32 = 50;
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;
    if subjects.len() > MAX_BATCH {
        return Err(Error::LimitExceeded);
    }
    for subject in subjects.iter() {
        Storage::add_to_whitelist(env, &issuer, &subject);
        Events::whitelist_updated(env, &issuer, &subject, true);
    }
    Ok(())
}

pub fn remove_from_whitelist(env: &Env, issuer: Address, subject: Address) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;
    Storage::remove_from_whitelist(env, &issuer, &subject);
    Events::whitelist_updated(env, &issuer, &subject, false);
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
    use crate::constants::SECS_PER_DAY;

    let attestation = Storage::get_attestation(env, &attestation_id).ok()?;
    // Only currently valid attestations are scored. Deleted ones return None so
    // the score does not reveal that they exist.
    if attestation.deleted
        || !matches!(attestation.get_status(env.ledger().timestamp()), AttestationStatus::Valid)
    {
        return None;
    }
    let tier_score = match Storage::get_issuer_tier(env, &attestation.issuer) {
        Some(IssuerTier::Premium) => 90u32,
        Some(IssuerTier::Verified) => 60u32,
        Some(IssuerTier::Basic) | None => 30u32,
    };
    let endorsements = Storage::get_endorsements(env, &attestation_id);
    let endorsement_score = (endorsements.len() * 2).min(10);
    let base_score = (tier_score + endorsement_score) as u64;

    let cfg = Storage::get_decay_config(env).unwrap_or_default();

    // Inactivity decay: score halves every `half_life_days` days.
    // Linear approximation: penalty_bps = days_inactive * 5000 / half_life_days, capped at 10000.
    let activity_factor_bps: u64 = if cfg.half_life_days == 0 {
        10_000
    } else {
        let days_inactive = if let Some(last) = Storage::get_last_issuance_time(env, &attestation.issuer) {
            env.ledger().timestamp().saturating_sub(last) / SECS_PER_DAY
        } else {
            0
        };
        let penalty = (days_inactive.saturating_mul(5_000) / cfg.half_life_days as u64).min(10_000);
        10_000 - penalty
    };

    // Revocation-ratio decay: reduces score proportionally to how often the
    // issuer revokes attestations, weighted by `revocation_weight` (0–100).
    let reputation_factor_bps: u64 = if cfg.revocation_weight == 0 {
        10_000
    } else {
        let stats = Storage::get_issuer_stats(env, &attestation.issuer);
        if stats.total_issued == 0 {
            10_000
        } else {
            let revocations = Storage::get_issuer_revocations(env, &attestation.issuer);
            let ratio_bps = (revocations.saturating_mul(10_000) / stats.total_issued).min(10_000);
            let penalty = (ratio_bps.saturating_mul(cfg.revocation_weight as u64) / 100).min(10_000);
            10_000 - penalty
        }
    };

    let decayed = (base_score
        .saturating_mul(activity_factor_bps) / 10_000)
        .saturating_mul(reputation_factor_bps) / 10_000;

    Some(decayed as u32)
}

pub fn set_decay_config(env: &Env, admin: Address, config: DecayConfig) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Storage::set_decay_config(env, &config);
    Ok(())
}

pub fn get_decay_config(env: &Env) -> DecayConfig {
    Storage::get_decay_config(env).unwrap_or_default()
}

pub fn is_decay_config_set(env: &Env) -> bool {
    Storage::get_decay_config(env).is_some()
}

pub fn get_issuer_metadata(env: &Env, issuer: Address) -> Option<IssuerMetadata> {
    Storage::get_issuer_metadata(env, &issuer)
}

const MAXIssuer_NAME_LEN: u32 = 64;
const MAXIssuer_URL_LEN: u32 = 256;
const MAXIssuer_DESC_LEN: u32 = 512;

pub fn set_issuer_metadata(env: &Env, issuer: Address, metadata: IssuerMetadata) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;

    // Validate name length
    let name_len = metadata.name.len();
    if name_len > MAXIssuer_NAME_LEN {
        return Err(Error::MetadataTooLong);
    }

    // Validate URL length and scheme
    let url_len = metadata.url.len();
    if url_len > MAXIssuer_URL_LEN {
        return Err(Error::MetadataTooLong);
    }
    if url_len > 0 {
        let url_slice = metadata.url.as_bytes();
        if url_slice.len() < 6 || &url_slice[..6] != b"https:" {
            return Err(Error::InvalidMetadata);
        }
    }

    // Validate description length
    let desc_len = metadata.description.len();
    if desc_len > MAXIssuer_DESC_LEN {
        return Err(Error::MetadataTooLong);
    }

    Storage::set_issuer_metadata(env, &issuer, &metadata);
    Events::issuer_metadata_updated(env, &issuer, &metadata);
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
    Validation::require_not_paused(env)?;
    Storage::set_whitelist_enabled(env, &issuer, enabled);
    if enabled {
        Events::whitelist_mode_enabled(env, &issuer);
    } else {
        Events::whitelist_mode_disabled(env, &issuer);
    }
    Ok(())
}

pub fn enable_whitelist_mode(env: &Env, issuer: Address) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;
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
    let fee_config = FeeConfig { attestation_fee: fee, fee_collector: collector.clone(), fee_token };
    Storage::set_fee_config(env, &fee_config);
    if let Some(mut config) = Storage::get_contract_config(env) {
        config.fee_config = fee_config;
        Storage::set_contract_config(env, &config);
    }
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

    let mut config = Storage::get_contract_config(env).unwrap_or_else(|| ContractConfig::default_for(env));
    config.require_registered_claim_type = require;
    Storage::set_contract_config(env, &config);
    Ok(())
}

pub fn get_require_registered_claim_type(env: &Env) -> bool {
    Storage::get_contract_config(env)
        .map(|config| config.require_registered_claim_type)
        .unwrap_or(false)
}

pub fn set_metadata_hash_only(env: &Env, admin: Address, enabled: bool) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;

    if let Some(mut config) = Storage::get_contract_config(env) {
        config.metadata_hash_only = enabled;
        Storage::set_contract_config(env, &config);
    }
    // If no config is stored yet, the flag defaults to false (no-op when disabling).
    // Enabling before any config exists is a no-op; callers should store config first.
    Ok(())
}

pub fn get_metadata_hash_only(env: &Env) -> bool {
    Storage::get_contract_config(env)
        .map(|config| config.metadata_hash_only)
        .unwrap_or(false)
}

pub fn set_max_attestations_per_subject(env: &Env, admin: Address, limit: Option<u32>) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;

    let mut config = Storage::get_contract_config(env).unwrap_or_else(|| ContractConfig::default_for(env));
    config.max_attestations_per_subject = limit;
    Storage::set_contract_config(env, &config);
    Events::max_attestations_per_subject_changed(env, &admin, limit);
    Ok(())
}

pub fn get_max_attestations_per_subject(env: &Env) -> Option<u32> {
    Storage::get_contract_config(env)
        .and_then(|config| config.max_attestations_per_subject)
}

/// Sets the `ChunkedIndex` chunk size stored in `ContractConfig`.
///
/// The chunk size controls how many attestation IDs are packed into each
/// on-chain storage entry for the subject/issuer indexes.
///
/// **This should only be called before any attestations are written.** Changing
/// the chunk size after index data exists will cause the old chunks (written
/// with the previous size) to be read correctly, but new writes will use the
/// new size, producing inconsistently-sized chunks. A full index migration is
/// required to normalise an existing index to a new chunk size.
///
/// # Errors
/// - [`Error::Unauthorized`] — caller is not a registered admin.
/// - [`Error::InvalidChunkSize`] — `chunk_size` is 0.
pub fn set_chunk_size(env: &Env, admin: Address, chunk_size: u32) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    if chunk_size == 0 {
        return Err(Error::InvalidChunkSize);
    }
    let mut config = Storage::get_contract_config(env).unwrap_or_else(|| ContractConfig::default_for(env));
    config.chunk_size = chunk_size;
    Storage::set_contract_config(env, &config);
    Ok(())
}

pub fn set_ttl_days(env: &Env, admin: Address, days: u32) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    if days == 0 || days > crate::constants::MAX_TTL_DAYS {
        return Err(Error::InvalidTtl);
    }
    Storage::set_ttl_config(env, &TtlConfig { ttl_days: days });
    if let Some(mut config) = Storage::get_contract_config(env) {
        config.ttl_config = TtlConfig { ttl_days: days };
        Storage::set_contract_config(env, &config);
    }
    Ok(())
}

/// Returns the currently configured `ChunkedIndex` chunk size (default: 50).
pub fn get_chunk_size(env: &Env) -> u32 {
    Storage::get_contract_config(env)
        .map(|config| config.chunk_size)
        .filter(|&s| s >= 1)
        .unwrap_or(50)
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
    Validation::validate_description(&description)?;
    let info = ClaimTypeInfo {
        claim_type: claim_type.clone(),
        description: description.clone(),
    };
    Storage::set_claim_type(env, &info);
    // Initialize status to Active
    Storage::set_claim_type_status(env, &claim_type, crate::types::ClaimTypeStatus::Active);
    Events::claim_type_registered(env, &claim_type, &description);
    Ok(())
}

pub fn set_claim_type_status(env: &Env, admin: Address, claim_type: String, status: crate::types::ClaimTypeStatus) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Validation::validate_claim_type(&claim_type)?;

    // Verify that the claim type is registered
    Storage::get_claim_type(env, &claim_type)
        .ok_or(Error::InvalidClaimType)?;

    Storage::set_claim_type_status(env, &claim_type, status);
    Events::claim_type_status_changed(env, &claim_type, status);
    Ok(())
}

pub fn get_claim_type_description(env: &Env, claim_type: String) -> Option<String> {
    Storage::get_claim_type(env, &claim_type).map(|info| info.description)
}

pub fn list_claim_types(env: &Env, start: u32, limit: u32) -> Vec<String> {
    crate::storage::paginate(env, &Storage::get_claim_type_list(env), start, limit)
}

const MAX_METADATA_LEN: u32 = 256;

pub fn set_claim_type_constraints(env: &Env, admin: Address, claim_type: String, constraints: crate::types::ClaimTypeConstraints) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Validation::validate_claim_type(&claim_type)?;
    Validation::require_not_paused(env)?;

    // Validate min <= max if both are set
    if let (Some(min), Some(max)) = (constraints.min_metadata_len, constraints.max_metadata_len) {
        if min > max {
            return Err(Error::InvalidConfigValue);
        }
        if max > MAX_METADATA_LEN {
            return Err(Error::MetadataTooLong);
        }
    }

    // If min is set but require_metadata is false, warn but allow (for compatibility)
    // In practice, this config is confusing but not technically invalid

    Storage::set_claim_type_constraints(env, &claim_type, &constraints);
    Events::claim_type_constraints_updated(env, &claim_type, &Some(constraints));
    Ok(())
}

pub fn clear_claim_type_constraints(env: &Env, admin: Address, claim_type: String) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Validation::validate_claim_type(&claim_type)?;
    Validation::require_not_paused(env)?;

    Storage::remove_claim_type_constraints(env, &claim_type);
    Events::claim_type_constraints_updated(env, &claim_type, &None);
    Ok(())
}

pub fn get_claim_type_constraints(env: &Env, claim_type: String) -> Option<crate::types::ClaimTypeConstraints> {
    Storage::get_claim_type_constraints(env, &claim_type)
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
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &issuer)?;
    Validation::validate_claim_type(&claim_type)?;
    Validation::require_registered_claim_type(env, &claim_type)?;
    if issuer == delegate {
        return Err(Error::CannotDelegateToSelf);
    }
    // Bridges and admins already hold privileged roles; they must not also
    // act as sub-issuers.
    if Storage::is_bridge(env, &delegate) || Storage::is_admin(env, &delegate) {
        return Err(Error::Unauthorized);
    }
    crate::attestation::validate_native_expiration(env, expiration)?;
    let is_update = Storage::get_delegation(env, &issuer, &delegate, &claim_type).is_some();
    let delegation = Delegation {
        delegator: issuer.clone(),
        delegate: delegate.clone(),
        claim_type: claim_type.clone(),
        expiration,
    };
    Storage::set_delegation(env, &delegation);
    if is_update {
        Events::delegation_updated(env, &issuer, &delegate, &claim_type, expiration);
    } else {
        Events::delegation_created(env, &issuer, &delegate, &claim_type, expiration);
    }
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

pub fn revoke_delegation_all(env: &Env, delegator: Address) -> Result<(), Error> {
    delegator.require_auth();
    Validation::require_not_paused(env)?;
    let index = Storage::get_delegator_index(env, &delegator);
    for (delegate, claim_type) in index.iter() {
        if Storage::get_delegation(env, &delegator, &delegate, &claim_type).is_some() {
            Storage::remove_delegation(env, &delegator, &delegate, &claim_type);
            Events::delegation_revoked(env, &delegator, &delegate, &claim_type);
        }
    }
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
    let issuer_count = Storage::get_issuer_list(env).len();
    let bridge_count = Storage::get_bridge_list(env).len();
    let version = Storage::get_version(env)
        .unwrap_or_else(|| soroban_sdk::String::from_str(env, ""));
    crate::types::HealthStatus {
        initialized,
        paused: Storage::is_paused(env),
        issuer_count,
        bridge_count,
        total_attestations: stats.total_attestations,
        version,
    }
}

// -----------------------------------------------------------------------
// Council actions with timelock (Issue #790)
// -----------------------------------------------------------------------

pub fn create_council_proposal(
    env: &Env,
    proposer: Address,
    operation: CouncilOperation,
) -> Result<u32, Error> {
    proposer.require_auth();
    Validation::require_admin(env, &proposer)?;

    let id = Storage::next_proposal_id(env);
    let mut approvals: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(env);
    approvals.push_back(proposer.clone());

    let proposal = CouncilProposal {
        id,
        operation,
        proposer: proposer.clone(),
        approvals,
        executed: false,
        quorum_reached_at: None,
    };
    Storage::set_proposal(env, &proposal);
    Events::proposal_created(env, id, &proposer);
    Ok(id)
}

pub fn approve_council_proposal(
    env: &Env,
    approver: Address,
    proposal_id: u32,
) -> Result<(), Error> {
    approver.require_auth();
    Validation::require_admin(env, &approver)?;

    let mut proposal = Storage::get_proposal(env, proposal_id).ok_or(Error::NotFound)?;
    if proposal.executed {
        return Err(Error::CouncilProposalExecuted);
    }
    for a in proposal.approvals.iter() {
        if a == approver {
            return Err(Error::AlreadyApproved);
        }
    }

    proposal.approvals.push_back(approver.clone());
    Events::proposal_approved(env, proposal_id, &approver);

    // Simple-majority quorum among the current council.
    let council = Storage::get_admin_council(env)?;
    let quorum = council.len() / 2 + 1;

    if proposal.approvals.len() >= quorum && proposal.quorum_reached_at.is_none() {
        let ts = env.ledger().timestamp();
        proposal.quorum_reached_at = Some(ts);
        Events::council_timelock_started(env, proposal_id, ts);
    }

    Storage::set_proposal(env, &proposal);
    Ok(())
}

pub fn execute_council_action(
    env: &Env,
    executor: Address,
    proposal_id: u32,
) -> Result<(), Error> {
    executor.require_auth();
    Validation::require_admin(env, &executor)?;

    let mut proposal = Storage::get_proposal(env, proposal_id).ok_or(Error::NotFound)?;
    if proposal.executed {
        return Err(Error::CouncilProposalExecuted);
    }
    let quorum_at = proposal.quorum_reached_at.ok_or(Error::Unauthorized)?;

    let delay = Storage::get_council_timelock_delay(env);
    let now = env.ledger().timestamp();
    if now < quorum_at.saturating_add(delay) {
        return Err(Error::TimelockNotReady);
    }

    match proposal.operation.clone() {
        CouncilOperation::RemoveIssuer(issuer) => {
            Storage::remove_issuer(env, &issuer);
            Storage::decrement_total_issuers(env);
            Events::issuer_removed(env, &issuer, &executor, now);
        }
        CouncilOperation::PauseContract => {
            Storage::set_paused(env, true);
            Events::contract_paused(env, &executor, now);
        }
    }

    proposal.executed = true;
    Storage::set_proposal(env, &proposal);
    Events::proposal_executed(env, proposal_id);
    Ok(())
}

pub fn get_council_proposal(env: &Env, proposal_id: u32) -> Option<CouncilProposal> {
    Storage::get_proposal(env, proposal_id)
}

pub fn set_council_timelock_delay(env: &Env, admin: Address, delay_seconds: u64) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Storage::set_council_timelock_delay(env, delay_seconds);
    Ok(())
}

pub fn get_council_timelock_delay(env: &Env) -> u64 {
    Storage::get_council_timelock_delay(env)
}

// -----------------------------------------------------------------------
// Dispute resolution (Issue #791)
// -----------------------------------------------------------------------

pub fn resolve_dispute(
    env: &Env,
    resolver: Address,
    attestation_id: String,
    resolution_note: Option<String>,
) -> Result<(), Error> {
    use crate::types::{DisputeStatus, AuditAction, AuditEntry};
    
    resolver.require_auth();

    let attestation = Storage::get_attestation(env, &attestation_id)?;
    if attestation.issuer != resolver && !Storage::is_admin(env, &resolver) {
        return Err(Error::Unauthorized);
    }
    
    let mut dispute = Storage::get_dispute(env, &attestation_id).ok_or(Error::NotDisputed)?;
    
    // Issue #1567: Cannot resolve already-resolved disputes
    if dispute.status != DisputeStatus::Open {
        return Err(Error::InvalidAttestation); // or new error: DisputeAlreadyResolved
    }

    Validation::require_not_paused(env)?;

    let timestamp = env.ledger().timestamp();
    
    // Issue #1566: Record resolution outcome instead of deleting
    dispute.status = DisputeStatus::Upheld; // Default to Upheld when resolver acts
    dispute.resolved_by = Some(resolver.clone());
    dispute.resolved_at = Some(timestamp);
    dispute.resolution_note = resolution_note.clone();
    
    Storage::set_dispute(env, &attestation_id, &dispute);
    
    // Issue #1566: Add resolution to audit log
    let audit_entry = AuditEntry {
        action: AuditAction::DisputeResolved,
        actor: resolver.clone(),
        timestamp,
        details: resolution_note,
    };
    Storage::append_audit_entry(env, &attestation_id, &audit_entry);
    
    Events::dispute_resolved(env, &attestation_id, &resolver, timestamp);
    Ok(())
}

// -----------------------------------------------------------------------
// Config & metadata getters
// -----------------------------------------------------------------------

/// Return a full snapshot of the contract configuration.
///
/// Falls back to sensible defaults for any field not yet explicitly set,
/// ensuring backward compatibility with contracts initialised before a
/// particular config key existed.
pub fn get_config(env: &Env) -> ContractConfig {
    let mut config = Storage::get_contract_config(env).unwrap_or_else(|| ContractConfig::default_for(env));
    if let Some(version) = Storage::get_version(env) {
        config.contract_version = version;
    }
    if let Some(fee_config) = Storage::get_fee_config(env) {
        config.fee_config = fee_config;
    }
    if let Some(ttl_config) = Storage::get_ttl_config(env) {
        config.ttl_config = ttl_config;
    }
    if config.contract_description.is_empty() {
        config.contract_description = String::from_str(
            env,
            "On-chain attestation and verification system for the Stellar blockchain.",
        );
    }
    if config.chunk_size == 0 {
        config.chunk_size = get_chunk_size(env);
    }
    config
}

/// Return static contract metadata (name, version, description).
///
/// # Errors
/// - [`Error::NotInitialized`] — contract has not been initialised.
pub fn get_contract_metadata(env: &Env) -> Result<crate::types::ContractMetadata, Error> {
    let version = Storage::get_version(env).ok_or(Error::NotInitialized)?;
    Ok(crate::types::ContractMetadata {
        name: soroban_sdk::String::from_str(env, "TrustLink"),
        version,
        description: soroban_sdk::String::from_str(
            env,
            "On-chain attestation and verification system for the Stellar blockchain.",
        ),
    })
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
        // try_balance returns Result<Result<i128, Error>, Error> where the outer Result
        // covers host errors and the inner Result covers token-specific errors (e.g., not a SEP-41 token).
        match token.try_balance(&env.current_contract_address()) {
            Ok(Ok(_)) => {}
            _ => return Err(Error::InvalidFeeToken),
        }
    }
    Ok(())
}
