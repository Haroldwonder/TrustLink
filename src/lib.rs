#![no_std]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod errors;
mod events;
mod storage;
pub mod types;
mod validation;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env, String, Vec};

use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    AdminCouncil, Attestation, AttestationRequest, AttestationStatus, AuditAction, AuditEntry, ClaimTypeInfo,
    ContractConfig, ContractMetadata, Endorsement, Error, FeeConfig, GlobalStats, HealthStatus,
    IssuerMetadata, IssuerStats, IssuerTier, MultiSigProposal, RateLimitConfig, RequestStatus,
    StorageLimits, TtlConfig, ATTESTATION_REQUEST_TTL_SECS, MULTISIG_PROPOSAL_TTL_SECS,
};
use crate::validation::Validation;

const SECS_PER_DAY: u64 = 86_400;
const MAX_SOURCE_CHAIN_LEN: u32 = 32;
const MAX_SOURCE_TX_LEN: u32 = 128;

mod callback {
    use soroban_sdk::{contractclient, Address, Env, String};
    #[contractclient(name = "ExpirationCallbackClient")]
    #[allow(dead_code)]
    pub trait ExpirationCallback {
        fn notify_expiring(env: Env, subject: Address, attestation_id: String, expiration: u64);
    }
}
use callback::ExpirationCallbackClient;

fn validate_metadata(env: &Env, metadata: &Option<String>) -> Result<(), Error> {
    Validation::validate_metadata(env, metadata)
}

fn validate_claim_type(claim_type: &String) -> Result<(), Error> {
    Validation::validate_claim_type(claim_type)
}

fn validate_reason(reason: &Option<String>) -> Result<(), Error> {
    if let Some(r) = reason {
        if r.len() > 128 {
            return Err(Error::ReasonTooLong);
        }
    }
    Ok(())
}

fn validate_source_reference(source_chain: &String, source_tx: &String) -> Result<(), Error> {
    if source_chain.len() > MAX_SOURCE_CHAIN_LEN || source_tx.len() > MAX_SOURCE_TX_LEN {
        return Err(Error::MetadataTooLong);
    }
    Ok(())
}

fn validate_tags(tags: &Option<Vec<String>>) -> Result<(), Error> {
    if let Some(t) = tags {
        if t.len() > 5 {
            return Err(Error::TooManyTags);
        }
        for tag in t.iter() {
            if tag.len() > 32 {
                return Err(Error::TagTooLong);
            }
        }
    }
    Ok(())
}

fn validate_jurisdiction(env: &Env, jurisdiction: &Option<String>) -> Result<(), Error> {
    if let Some(code) = jurisdiction {
        if code.len() != 2 {
            return Err(Error::InvalidJurisdiction);
        }
        let valid_codes = [
            "AF","AX","AL","DZ","AS","AD","AO","AI","AQ","AG","AR","AM","AW","AU","AT","AZ",
            "BS","BH","BD","BB","BY","BE","BZ","BJ","BM","BT","BO","BQ","BA","BW","BV","BR",
            "IO","BN","BG","BF","BI","CV","KH","CM","CA","KY","CF","TD","CL","CN","CX","CC",
            "CO","KM","CG","CD","CK","CR","CI","HR","CU","CW","CY","CZ","DK","DJ","DM","DO",
            "EC","EG","SV","GQ","ER","EE","SZ","ET","FK","FO","FJ","FI","FR","GF","PF","TF",
            "GA","GM","GE","DE","GH","GI","GR","GL","GD","GP","GU","GT","GG","GN","GW","GY",
            "HT","HM","VA","HN","HK","HU","IS","IN","ID","IR","IQ","IE","IM","IL","IT","JM",
            "JP","JE","JO","KZ","KE","KI","KP","KR","KW","KG","LA","LV","LB","LS","LR","LY",
            "LI","LT","LU","MO","MK","MG","MW","MY","MV","ML","MT","MH","MQ","MR","MU","YT",
            "MX","FM","MD","MC","MN","ME","MS","MA","MZ","MM","NA","NR","NP","NL","NC","NZ",
            "NI","NE","NG","NU","NF","MP","NO","OM","PK","PW","PS","PA","PG","PY","PE","PH",
            "PN","PL","PT","PR","QA","RE","RO","RU","RW","BL","SH","KN","LC","MF","PM","VC",
            "WS","SM","ST","SA","SN","RS","SC","SL","SG","SX","SK","SI","SB","SO","ZA","GS",
            "SS","ES","LK","SD","SR","SJ","SE","CH","SY","TW","TJ","TZ","TH","TL","TG","TK",
            "TO","TT","TN","TR","TM","TC","TV","UG","UA","AE","GB","US","UM","UY","UZ","VU",
            "VE","VN","VG","VI","WF","EH","YE","ZM","ZW",
        ];
        let mut valid = false;
        for iso in valid_codes.iter() {
            if code == &String::from_str(env, iso) {
                valid = true;
                break;
            }
        }
        if !valid {
            return Err(Error::InvalidJurisdiction);
        }
    }
    Ok(())
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
        let _ = token.balance(&env.current_contract_address());
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

/// Shared helper: persist attestation + update indexes + increment counters.
fn store_attestation(env: &Env, attestation: &Attestation) {
    Storage::set_attestation(env, attestation);
    Storage::add_subject_attestation(env, &attestation.subject, &attestation.id);
    Storage::add_issuer_attestation(env, &attestation.issuer, &attestation.id);
    let mut stats = Storage::get_issuer_stats(env, &attestation.issuer);
    stats.total_issued += 1;
    Storage::set_issuer_stats(env, &attestation.issuer, &stats);
    Storage::increment_total_attestations(env, 1);
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

#[contract]
pub struct TrustLinkContract;

#[contractimpl]
impl TrustLinkContract {
    // -----------------------------------------------------------------------
    // Initialization & Admin
    // -----------------------------------------------------------------------

    pub fn initialize(env: Env, admin: Address, ttl_days: Option<u32>) -> Result<(), Error> {
        admin.require_auth();
        if Storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        let mut council: AdminCouncil = Vec::new(&env);
        council.push_back(admin.clone());
        Storage::set_admin_council(&env, &council);
        Storage::set_version(&env, &String::from_str(&env, "1.0.0"));
        Storage::set_fee_config(&env, &default_fee_config(&admin));
        let days = ttl_days.unwrap_or(30);
        Storage::set_ttl_config(&env, &TtlConfig { ttl_days: days });
        Events::admin_initialized(&env, &admin, env.ledger().timestamp());
        Ok(())
    }

    pub fn transfer_admin(env: Env, current_admin: Address, new_admin: Address) -> Result<(), Error> {
        current_admin.require_auth();
        Validation::require_admin(&env, &current_admin)?;
        Storage::add_admin(&env, &new_admin);
        Storage::remove_admin(&env, &current_admin);
        Events::admin_transferred(&env, &current_admin, &new_admin);
        Ok(())
    }

    pub fn add_admin(env: Env, existing_admin: Address, new_admin: Address) -> Result<(), Error> {
        existing_admin.require_auth();
        Validation::require_admin(&env, &existing_admin)?;
        if Storage::is_admin(&env, &new_admin) {
            return Ok(());
        }
        Storage::add_admin(&env, &new_admin);
        Events::admin_added(&env, &existing_admin, &new_admin, env.ledger().timestamp());
        Ok(())
    }

    pub fn remove_admin(env: Env, existing_admin: Address, admin_to_remove: Address) -> Result<(), Error> {
        existing_admin.require_auth();
        Validation::require_admin(&env, &existing_admin)?;
        let council = Storage::get_admin_council(&env)?;
        if council.len() <= 1 {
            return Err(Error::LastAdminCannotBeRemoved);
        }
        if !Storage::is_admin(&env, &admin_to_remove) {
            return Ok(());
        }
        Storage::remove_admin(&env, &admin_to_remove);
        Events::admin_removed(&env, &existing_admin, &admin_to_remove, env.ledger().timestamp());
        Ok(())
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        Storage::get_admin(&env)
    }

    // -----------------------------------------------------------------------
    // Issuer management
    // -----------------------------------------------------------------------

    pub fn register_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::add_issuer(&env, &issuer);
        Storage::increment_total_issuers(&env);
        Events::issuer_registered(&env, &issuer, &admin, env.ledger().timestamp());
        Ok(())
    }

    pub fn remove_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::remove_issuer(&env, &issuer);
        Storage::decrement_total_issuers(&env);
        Events::issuer_removed(&env, &issuer, &admin, env.ledger().timestamp());
        Ok(())
    }

    /// Add `subject` to `issuer`'s whitelist.
    pub fn add_to_whitelist(env: Env, issuer: Address, subject: Address) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::add_to_whitelist(&env, &issuer, &subject);
        Ok(())
    }

    /// Remove `subject` from `issuer`'s whitelist.
    pub fn remove_from_whitelist(env: Env, issuer: Address, subject: Address) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::remove_from_whitelist(&env, &issuer, &subject);
        Ok(())
    }

    /// Return `true` if `subject` is whitelisted for `issuer`.
    #[must_use]
    pub fn is_whitelisted(env: Env, issuer: Address, subject: Address) -> bool {
        Storage::is_whitelisted(&env, &issuer, &subject)
    }

    /// Return `true` if whitelist mode is enabled for `issuer`.
    #[must_use]
    pub fn is_whitelist_enabled(env: Env, issuer: Address) -> bool {
        Storage::is_whitelist_enabled(&env, &issuer)
    }

    /// Update the trust tier of an already-registered issuer.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not registered.
    pub fn update_issuer_tier(
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

    /// Return the trust tier of `issuer`, or `None` if not registered.
    #[must_use]
    pub fn get_issuer_tier(env: Env, issuer: Address) -> Option<IssuerTier> {
        Storage::get_issuer_tier(&env, &issuer)
    }

    /// Return `true` if `subject` holds a valid `claim_type` attestation issued
    /// by an issuer whose tier is >= `min_tier`.
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

    pub fn get_issuer_metadata(env: Env, issuer: Address) -> Option<IssuerMetadata> {
        Storage::get_issuer_metadata(&env, &issuer)
    }

    // -----------------------------------------------------------------------
    // Bridge management
    // -----------------------------------------------------------------------

    pub fn register_bridge(env: Env, admin: Address, bridge_contract: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::add_bridge(&env, &bridge_contract);
        Ok(())
    }

    pub fn is_bridge(env: Env, address: Address) -> bool {
        Storage::is_bridge(&env, &address)
    }

    // -----------------------------------------------------------------------
    // Whitelist management
    // -----------------------------------------------------------------------

    pub fn set_whitelist_enabled(env: Env, issuer: Address, enabled: bool) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::set_whitelist_enabled(&env, &issuer, enabled);
        Ok(())
    }

    pub fn enable_whitelist_mode(env: Env, issuer: Address) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::set_whitelist_mode(&env, &issuer, true);
        Events::whitelist_mode_enabled(&env, &issuer);
        Ok(())
    }

    /// Create a new attestation about a subject address.    ///
    /// The attestation ID is derived deterministically from `(issuer, subject,
    /// claim_type, timestamp)`, so the same combination at the same ledger
    /// timestamp will always produce the same ID.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — caller is not the admin.
    pub fn set_rate_limit(
        env: Env,
        admin: Address,
        min_issuance_interval: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        validate_fee_config(&env, fee, &fee_token)?;
        if admin == collector {
            return Err(Error::Unauthorized);
        }
        Storage::set_fee_config(&env, &FeeConfig { attestation_fee: fee, fee_collector: collector, fee_token });
        Ok(())
    }

    pub fn get_fee_config(env: Env) -> Result<FeeConfig, Error> {
        load_fee_config(&env)
    }

    pub fn set_rate_limit(env: Env, admin: Address, min_issuance_interval: u64) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::set_rate_limit_config(&env, &RateLimitConfig { min_issuance_interval });
        Ok(())
    }

    /// Retrieve the current rate limit configuration, or `None` if not set.
    #[must_use]
    pub fn get_rate_limit(env: Env) -> Option<RateLimitConfig> {
        Storage::get_rate_limit_config(&env)
    }

    // -----------------------------------------------------------------------
    // Pause / unpause
    // -----------------------------------------------------------------------

    pub fn pause(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::set_paused(&env, true);
        Events::contract_paused(&env, &admin, env.ledger().timestamp());
        Ok(())
    }

    pub fn unpause(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::set_paused(&env, false);
        Events::contract_unpaused(&env, &admin, env.ledger().timestamp());
        Ok(())
    }

    /// Return `true` if the contract is currently paused.
    #[must_use]
    pub fn is_paused(env: Env) -> bool {
        Storage::is_paused(&env)
    }

    // -----------------------------------------------------------------------
    // Storage limits
    // -----------------------------------------------------------------------

    pub fn set_limits(env: Env, admin: Address, max_attestations_per_issuer: u32, max_attestations_per_subject: u32) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::set_limits(&env, &StorageLimits { max_attestations_per_issuer, max_attestations_per_subject });
        Ok(())
    }

    pub fn get_limits(env: Env) -> StorageLimits {
        Storage::get_limits(&env)
    }

    // -----------------------------------------------------------------------
    // Attestation creation
    // -----------------------------------------------------------------------

    fn create_attestation_internal(
        env: &Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
        jurisdiction: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<String, Error> {
        issuer.require_auth();
        Validation::require_not_paused(&env)?;
        Validation::require_issuer(&env, &issuer)?;
        Validation::validate_claim_type(&claim_type)?;
        validate_metadata(&env, &metadata)?;
        validate_jurisdiction(env, &jurisdiction)?;
        validate_tags(&tags)?;
        validate_native_expiration(env, expiration)?;

        if issuer == subject {
            return Err(Error::Unauthorized);
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
            deleted: false,
        };

        // Persist state BEFORE external token call (reentrancy guard).
        store_attestation(env, &attestation);
        Events::attestation_created(env, &attestation);
        Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
            action: AuditAction::Created,
            actor: attestation.issuer.clone(),
            timestamp,
            details: None,
        });

        charge_attestation_fee(env, &issuer)?;
        Storage::set_last_issuance_time(env, &issuer, timestamp);
        Ok(attestation_id)
    }

    pub fn create_attestation(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<String, Error> {
        Self::create_attestation_internal(&env, issuer, subject, claim_type, expiration, metadata, None, tags)
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
        Self::create_attestation_internal(&env, issuer, subject, claim_type, expiration, metadata, jurisdiction, tags)
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
        validate_claim_type(&claim_type)?;
        validate_native_expiration(&env, expiration)?;
        check_rate_limit(&env, &issuer)?;

        let timestamp = env.ledger().timestamp();
        let limits = Storage::get_limits(&env);
        let issuer_count = Storage::get_issuer_attestations(&env, &issuer).len();
        if issuer_count.saturating_add(subjects.len()) > limits.max_attestations_per_issuer {
            return Err(Error::LimitExceeded);
        }

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer,
            subject,
            claim_type,
            timestamp,
            expiration,
            revoked: false,
            deleted: false,
            metadata: None,
            jurisdiction: None,
            valid_from: None,
            origin: AttestationOrigin::Imported,
            source_chain: None,
            source_tx: None,
            tags: None,
            revocation_reason: None,
            deleted: false,
        };

        store_attestation(&env, &attestation);
        Events::attestation_imported(&env, &attestation);
        Storage::append_audit_entry(
            &env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Created,
                actor: admin.clone(),
                timestamp,
                details: None,
            },
        );
        Ok(attestation_id)
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
        validate_source_reference(&source_chain, &source_tx)?;

        if source_chain.len() > 32 {
            return Err(Error::SourceRefTooLong);
        }
        if source_tx.len() > 128 {
            return Err(Error::SourceRefTooLong);
        }

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
            deleted: false,
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
        validate_claim_type(&claim_type)?;
        validate_native_expiration(&env, expiration)?;
        check_rate_limit(&env, &issuer)?;

        let timestamp = env.ledger().timestamp();

        // Enforce issuer-level limit up front for the whole batch
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

            // Per-subject limit check
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
                deleted: false,
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

    /// Revoke an attestation.
    ///
    /// SECURITY (FINDING-002): `require_issuer` ensures the caller is a
    /// currently registered issuer, preventing deregistered issuers from
    /// revoking attestations.
    pub fn revoke_attestation(
        env: Env,
        issuer: Address,
        attestation_id: String,
        reason: Option<String>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_not_paused(&env)?;
        // FINDING-002 fix: verify caller is a currently registered issuer.
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

        Events::attestation_revoked(&env, &attestation_id, &issuer, &reason);
        Storage::append_audit_entry(&env, &attestation_id, &AuditEntry {
            action: AuditAction::Revoked,
            actor: issuer.clone(),
            timestamp: env.ledger().timestamp(),
            details: reason.clone(),
        });
        Storage::increment_total_revocations(&env, 1);
        Ok(())
    }

    pub fn renew_attestation(
        env: Env,
        issuer: Address,
        attestation_id: String,
        new_expiration: Option<u64>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        validate_native_expiration(&env, new_expiration)?;

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;
        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        attestation.expiration = new_expiration;
        Storage::set_attestation(&env, &attestation);
        Events::attestation_renewed(&env, &attestation_id, &issuer, new_expiration);
        Storage::append_audit_entry(&env, &attestation_id, &AuditEntry {
            action: AuditAction::Renewed,
            actor: issuer.clone(),
            timestamp: env.ledger().timestamp(),
            details: None,
        });
        Ok(())
    }

    /// Revoke multiple attestations in a single atomic call (issuer only).
    ///
    /// Authorization is checked once for the issuer. If any attestation does
    /// not belong to the caller or is already revoked the entire batch is
    /// rolled back — no partial writes occur.
    ///
    /// Max batch size is 50. Passing more IDs returns [`Error::BatchTooLarge`].
    ///
    /// Emits one `revoked` event per attestation.
    ///
    /// # Parameters
    /// - `issuer` — authorized issuer (must authorize).
    /// - `attestation_ids` — list of IDs to revoke (max 50).
    /// - `reason` — optional human-readable reason stored in the event data.
    ///
    /// # Returns
    /// Count of revoked attestations.
    ///
    /// # Errors
    /// - [`Error::BatchTooLarge`] — more than 50 IDs supplied.
    /// - [`Error::Unauthorized`] — issuer is not registered or does not own an attestation.
    /// - [`Error::NotFound`] — an ID does not exist.
    /// - [`Error::AlreadyRevoked`] — an attestation is already revoked.
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

        // Validate all attestations first (atomic — no partial writes)
        for id in attestation_ids.iter() {
            let attestation = Storage::get_attestation(&env, &id)?;
            if attestation.issuer != issuer {
                return Err(Error::Unauthorized);
            }
            if attestation.revoked {
                return Err(Error::AlreadyRevoked);
            }
        }

        // All checks passed — apply writes
        let mut count: u32 = 0;
        for id in attestation_ids.iter() {
            let mut attestation = Storage::get_attestation(&env, &id)?;
            attestation.revoked = true;
            attestation.revocation_reason = reason.clone();
            Storage::set_attestation(&env, &attestation);
            Storage::remove_subject_attestation(&env, &attestation.subject, &id);
            Storage::remove_issuer_attestation(&env, &issuer, &id);
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

        Storage::increment_total_revocations(&env, count as u64);
        Ok(count)
    }

    pub fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();

        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if attestation.deleted || attestation.claim_type != claim_type {
                    continue;
                }
                match attestation.get_status(current_time) {
                    AttestationStatus::Valid => {
                        if let Some(expiration) = attestation.expiration {
                            maybe_trigger_expiration_hook(
                                &env,
                                &subject,
                                &attestation_id,
                                expiration,
                                current_time,
                            );
                        }
                        return true;
                    }
                    AttestationStatus::Expired => {
                        Events::attestation_expired(&env, &attestation_id, &subject);
                    }
                    AttestationStatus::Revoked | AttestationStatus::Pending => {}
                }
            }
        }
        false
    }

    pub fn has_valid_claim_from_issuer(
        env: Env,
        subject: Address,
        claim_type: String,
        issuer: Address,
    ) -> bool {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if !attestation.deleted
                    && attestation.claim_type == claim_type
                    && attestation.get_status(current_time) == AttestationStatus::Valid
                {
                    return true;
                }
            }
        }
        false
    }

    pub fn has_valid_claim_from_issuer(env: Env, subject: Address, claim_type: String, issuer: Address) -> bool {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if attestation.deleted { continue; }
                if attestation.claim_type == claim_type && attestation.issuer == issuer {
                    match attestation.get_status(current_time) {
                        AttestationStatus::Valid => return true,
                        AttestationStatus::Expired => {
                            Events::attestation_expired(&env, &attestation_id, &subject);
                        }
                        _ => {}
                    }
                }
            }
        }
        false
    }

    #[must_use]
    pub fn has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool {
        if claim_types.is_empty() {
            return false;
        }
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        for claim_type in claim_types.iter() {
            for attestation_id in attestation_ids.iter() {
                if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                    if !attestation.deleted
                        && attestation.claim_type == claim_type
                        && attestation.get_status(current_time) == AttestationStatus::Valid
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Returns `true` only if the subject holds a valid attestation for **every**
    /// claim type in `claim_types`.
    ///
    /// # Behaviour (verified against spec in issue #309)
    /// - AND-logic: short-circuits and returns `false` on the first unsatisfied claim.
    /// - Empty `claim_types` list returns `true` (vacuous truth).
    /// - Revoked, expired, pending, and deleted attestations are excluded.
    pub fn has_all_claims(env: Env, subject: Address, claim_types: Vec<String>) -> bool {
        if claim_types.is_empty() { return true; }
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        'claims: for claim_type in claim_types.iter() {
            for attestation_id in attestation_ids.iter() {
                if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                    if !attestation.deleted
                        && attestation.claim_type == claim_type
                        && attestation.get_status(current_time) == AttestationStatus::Valid
                    {
                        continue 'claims;
                    }
                }
            }
            return false;
        }
        true
    }

    #[must_use]
    pub fn get_attestation(env: Env, attestation_id: String) -> Result<Attestation, Error> {
        let attestation = Storage::get_attestation(&env, &attestation_id)?;
        if attestation.deleted {
            return Err(Error::NotFound);
        }
        Ok(attestation)
    }

    /// Request GDPR deletion of an attestation.
    ///
    /// Only the subject of the attestation may call this. The attestation is
    /// marked as `deleted` (soft-delete) and removed from the subject index so
    /// it no longer appears in any query result. The record itself is retained
    /// in storage for audit purposes, but is invisible to all public queries.
    ///
    /// A `DeletionRequested` event is emitted for off-chain compliance audit trails.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — attestation does not exist.
    /// - [`Error::Unauthorized`] — caller is not the subject of the attestation.
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
    }

    /// Return the full audit log for `attestation_id`.
    ///
    /// The log is append-only and contains one entry per state change
    /// (create, revoke, renew, update). Returns an empty list if the
    /// attestation has no recorded history.
    #[must_use]
    pub fn get_audit_log(env: Env, attestation_id: String) -> Vec<AuditEntry> {
        Storage::get_audit_log(&env, &attestation_id)
    }

    #[must_use]
    pub fn get_attestation_status(
        env: Env,
        attestation_id: String,
    ) -> Result<AttestationStatus, Error> {
        let attestation = Storage::get_attestation(&env, &attestation_id)?;
        if attestation.deleted {
            return Err(Error::NotFound);
        }
        let status = attestation.get_status(env.ledger().timestamp());
        if status == AttestationStatus::Expired {
            Events::attestation_expired(&env, &attestation_id, &attestation.subject);
        }
        Ok(status)
    }

    #[must_use]
    pub fn get_subject_attestations(
        env: Env,
        subject: Address,
        start: u32,
        limit: u32,
    ) -> Vec<String> {
        let ids = Storage::get_subject_attestations(&env, &subject);
        let mut filtered = Vec::new(&env);
        for id in ids.iter() {
            if let Ok(a) = Storage::get_attestation(&env, &id) {
                if !a.deleted {
                    filtered.push_back(id);
                }
            }
        }
        paginate_strings(&env, filtered, start, limit)
    }

    #[must_use]
    pub fn get_attestations_in_range(
        env: Env,
        subject: Address,
        from_ts: u64,
        to_ts: u64,
        start: u32,
        limit: u32,
    ) -> Vec<Attestation> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let mut filtered_ids = Vec::new(&env);

    pub fn get_issuer_attestations(env: Env, issuer: Address, start: u32, limit: u32) -> Vec<String> {
        crate::storage::paginate(&env, &Storage::get_issuer_attestations(&env, &issuer), start, limit)
    }

    pub fn get_attestations_in_range(env: Env, subject: Address, from_ts: u64, to_ts: u64, start: u32, limit: u32) -> Vec<Attestation> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let mut filtered = Vec::new(&env);
        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if !attestation.deleted && attestation.timestamp >= from_ts && attestation.timestamp <= to_ts {
                    filtered.push_back(id);
                }
            }
        }

        let paginated_ids = crate::storage::paginate(&env, &filtered_ids, start, limit);
        let mut result = Vec::new(&env);
        for id in paginated.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                result.push_back(attestation);
            }
        }
        result
    }

    #[must_use]
    pub fn get_attestations_by_tag(env: Env, subject: Address, tag: String) -> Vec<String> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let mut result = Vec::new(&env);
        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if attestation.deleted { continue; }
                if let Some(tags) = attestation.tags {
                    for t in tags.iter() {
                        if t == tag {
                            result.push_back(id.clone());
                            break;
                        }
                    }
                }
            }
        }
        result
    }

    #[must_use]
    pub fn get_attestations_by_jurisdiction(
        env: Env,
        subject: Address,
        jurisdiction: String,
    ) -> Vec<String> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let mut result = Vec::new(&env);
        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if attestation.deleted { continue; }
                if let Some(j) = attestation.jurisdiction {
                    if j == jurisdiction {
                        result.push_back(id.clone());
                    }
                }
            }
        }
        result
    }

    #[must_use]
    pub fn get_issuer_attestations(
        env: Env,
        issuer: Address,
        start: u32,
        limit: u32,
    ) -> Vec<String> {
        let ids = Storage::get_issuer_attestations(&env, &issuer);
        let mut filtered = Vec::new(&env);
        for id in ids.iter() {
            if let Ok(a) = Storage::get_attestation(&env, &id) {
                if !a.deleted {
                    filtered.push_back(id);
                }
            }
        }
        paginate_strings(&env, filtered, start, limit)
    }

    /// Returns the total number of attestations created by `issuer` from the issuer index.
    pub fn get_issuer_attestation_count(env: Env, issuer: Address) -> u32 {
        Storage::get_issuer_attestations(&env, &issuer).len()
    }

    pub fn get_valid_claims(env: Env, subject: Address) -> Vec<String> {
        let current_time = env.ledger().timestamp();
        let mut result = Vec::new(&env);
        for attestation_id in Storage::get_subject_attestations(&env, &subject).iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if !attestation.deleted && attestation.get_status(current_time) == AttestationStatus::Valid {
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

    /// Returns the count of non-revoked, non-expired, non-deleted attestations for `subject`.
    pub fn get_valid_claim_count(env: Env, subject: Address) -> u32 {
        let current_time = env.ledger().timestamp();
        let mut count: u32 = 0;
        for attestation_id in Storage::get_subject_attestations(&env, &subject).iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if !attestation.deleted
                    && attestation.get_status(current_time) == AttestationStatus::Valid
                {
                    count += 1;
                }
            }
        }
        count
    }

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
                let attestation = Storage::get_attestation(&env, &attestation_id)?;
                if !attestation.deleted
                    && attestation.claim_type == claim_type
                    && attestation.get_status(current_time) == AttestationStatus::Valid
                {
                    return Ok(attestation);
                }
            }
        }

        None
    }

    #[must_use]
    pub fn is_issuer(env: Env, address: Address) -> bool {
        Storage::is_issuer(&env, &address)
    }

    #[must_use]
    pub fn get_issuer_stats(env: Env, issuer: Address) -> IssuerStats {
        Storage::get_issuer_stats(&env, &issuer)
    }

    #[must_use]
    pub fn is_bridge(env: Env, address: Address) -> bool {
        Storage::is_bridge(&env, &address)
    }

    pub fn request_deletion(env: Env, subject: Address, attestation_id: String) -> Result<(), Error> {
        subject.require_auth();
        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;
        if attestation.subject != subject {
            return Err(Error::Unauthorized);
        }
        attestation.deleted = true;
        Storage::set_attestation(&env, &attestation);
        Storage::remove_subject_attestation(&env, &subject, &attestation_id);
        Events::deletion_requested(&env, &subject, &attestation_id, env.ledger().timestamp());
        Ok(())
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
    }

    #[must_use]
    pub fn get_issuer_metadata(env: Env, issuer: Address) -> Option<IssuerMetadata> {
        Storage::get_issuer_metadata(&env, &issuer)
    }

    #[must_use]
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        Storage::get_admin(&env)
    }

    #[must_use]
    pub fn get_fee_config(env: Env) -> Result<FeeConfig, Error> {
        load_fee_config(&env)
    }

    pub fn register_claim_type(env: Env, admin: Address, claim_type: String, description: String) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        validate_claim_type(&claim_type)?;
        let info = ClaimTypeInfo { claim_type: claim_type.clone(), description: description.clone() };
        Storage::set_claim_type(&env, &info);
        Events::claim_type_registered(&env, &claim_type, &description);
        Ok(())
    }

    #[must_use]
    pub fn get_claim_type_description(env: Env, claim_type: String) -> Option<String> {
        Storage::get_claim_type(&env, &claim_type).map(|info| info.description)
    }

    #[must_use]
    pub fn list_claim_types(env: Env, start: u32, limit: u32) -> Vec<String> {
        crate::storage::paginate(&env, &Storage::get_claim_type_list(&env), start, limit)
    }

    // -----------------------------------------------------------------------
    // Multi-sig attestation proposals
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
                deleted: false,
            };
            store_attestation(&env, &attestation);
            Events::attestation_created(&env, &attestation);
            Events::multisig_activated(&env, &proposal_id, &attestation_id);
        } else {
            Storage::set_multisig_proposal(&env, &proposal);
        }
        Ok(())
    }

    /// Retrieve a multi-sig proposal by ID.
    #[must_use]
    pub fn get_multisig_proposal(env: Env, proposal_id: String) -> Result<MultiSigProposal, Error> {
        Storage::get_multisig_proposal(&env, &proposal_id)
    }

    // -------------------------------------------------------------------------
    // Attestation request workflow (#304)
    // -------------------------------------------------------------------------

    /// Subject requests an attestation from a registered issuer.
    ///
    /// Creates a pending [`AttestationRequest`] with a 7-day TTL. Returns the
    /// deterministic request ID.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    /// - [`Error::DuplicateRequest`] — an identical pending request already exists.
    pub fn request_attestation(
        env: Env,
        subject: Address,
        issuer: Address,
        claim_type: String,
    ) -> Result<String, Error> {
        subject.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        validate_claim_type(&claim_type)?;

        let timestamp = env.ledger().timestamp();
        let request_id =
            AttestationRequest::generate_id(&env, &subject, &issuer, &claim_type, timestamp);

        // Duplicate check: reject if a pending request with this ID already exists.
        if let Ok(existing) = Storage::get_attestation_request(&env, &request_id) {
            if existing.status == RequestStatus::Pending {
                return Err(Error::DuplicateRequest);
            }
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

        Storage::set_attestation_request(&env, &request);
        Storage::add_issuer_pending_request(&env, &issuer, &request_id);
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

    /// Issuer fulfills a pending attestation request, creating the attestation.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — request does not exist.
    /// - [`Error::Unauthorized`] — caller is not the request's target issuer.
    /// - [`Error::RequestAlreadyProcessed`] — request was already fulfilled or rejected.
    /// - [`Error::RequestExpired`] — request TTL has elapsed.
    pub fn fulfill_request(
        env: Env,
        issuer: Address,
        request_id: String,
        expiration: Option<u64>,
    ) -> Result<String, Error> {
        issuer.require_auth();
        let mut request = Storage::get_attestation_request(&env, &request_id)?;

        if request.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if request.status != RequestStatus::Pending {
            return Err(Error::RequestAlreadyProcessed);
        }
        let current_time = env.ledger().timestamp();
        if current_time >= request.expires_at {
            return Err(Error::RequestExpired);
        }

        validate_native_expiration(&env, expiration)?;

        // Create the attestation.
        let attestation_id = Attestation::generate_id(
            &env,
            &issuer,
            &request.subject,
            &request.claim_type,
            current_time,
        );
        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: issuer.clone(),
            subject: request.subject.clone(),
            claim_type: request.claim_type.clone(),
            timestamp: current_time,
            expiration,
            revoked: false,
            metadata: None,
            jurisdiction: None,
            valid_from: None,
            imported: false,
            bridged: false,
            source_chain: None,
            source_tx: None,
            tags: None,
            revocation_reason: None,
            deleted: false,
        };
        store_attestation(&env, &attestation);
        Events::attestation_created(&env, &attestation);

        // Mark request fulfilled and remove from pending list.
        request.status = RequestStatus::Fulfilled;
        Storage::set_attestation_request(&env, &request);
        Storage::remove_issuer_pending_request(&env, &issuer, &request_id);
        Events::request_fulfilled(&env, &request_id, &issuer, &attestation_id);

        Ok(attestation_id)
    }

    /// Issuer rejects a pending attestation request.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — request does not exist.
    /// - [`Error::Unauthorized`] — caller is not the request's target issuer.
    /// - [`Error::RequestAlreadyProcessed`] — request was already fulfilled or rejected.
    pub fn reject_request(
        env: Env,
        issuer: Address,
        request_id: String,
        reason: Option<String>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        validate_reason(&reason)?;
        let mut request = Storage::get_attestation_request(&env, &request_id)?;

        if request.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if request.status != RequestStatus::Pending {
            return Err(Error::RequestAlreadyProcessed);
        }

        request.status = RequestStatus::Rejected;
        request.rejection_reason = reason.clone();
        Storage::set_attestation_request(&env, &request);
        Storage::remove_issuer_pending_request(&env, &issuer, &request_id);
        Events::request_rejected(&env, &request_id, &issuer, &reason);
        Ok(())
    }

    /// Return a paginated list of pending (non-expired) request IDs for `issuer`.
    pub fn get_pending_requests(
        env: Env,
        issuer: Address,
        start: u32,
        limit: u32,
    ) -> Vec<String> {
        let current_time = env.ledger().timestamp();
        let all_ids = Storage::get_issuer_pending_requests(&env, &issuer);
        // Filter to only truly pending (not expired) requests.
        let mut pending = Vec::new(&env);
        for id in all_ids.iter() {
            if let Ok(req) = Storage::get_attestation_request(&env, &id) {
                if req.status == RequestStatus::Pending && current_time < req.expires_at {
                    pending.push_back(id);
                }
            }
        }
        crate::storage::paginate(&env, &pending, start, limit)
    }

    /// Retrieve a single attestation request by ID.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — request does not exist.
    pub fn get_attestation_request(
        env: Env,
        request_id: String,
    ) -> Result<AttestationRequest, Error> {
        Storage::get_attestation_request(&env, &request_id)
    }

    /// Endorse an existing attestation, adding a layer of social proof.
    ///
    /// Only registered issuers may endorse. An issuer cannot endorse their own
    /// attestation, and cannot endorse a revoked attestation. Each issuer may
    /// endorse a given attestation at most once.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — endorser is not a registered issuer.
    /// - [`Error::NotFound`] — attestation does not exist.
    /// - [`Error::CannotEndorseOwn`] — endorser is the attestation's issuer.
    /// - [`Error::AlreadyRevoked`] — attestation has been revoked.
    /// - [`Error::AlreadyEndorsed`] — endorser has already endorsed this attestation.
    /// Delegate authority to create attestations of `claim_type` to `delegate`.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    /// - [`Error::CannotDelegateToSelf`] — `issuer == delegate`.
    pub fn delegate_claim_type(
        env: Env,
        issuer: Address,
        delegate: Address,
        claim_type: String,
        expiration: Option<u64>,
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

    /// Revoke a previously granted delegation.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    pub fn revoke_delegation(
        env: Env,
        issuer: Address,
        delegate: Address,
        claim_type: String,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::remove_delegation(&env, &issuer, &delegate, &claim_type);
        Events::delegation_revoked(&env, &issuer, &delegate, &claim_type);
        Ok(())
    }

    /// Create an attestation on behalf of `issuer` using a delegation grant.
    ///
    /// # Errors
    /// - [`Error::DelegationNotFound`] — no delegation exists for this triple.
    /// - [`Error::DelegationExpired`] — the delegation has passed its expiration.
    /// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
    pub fn create_attestation_as_delegate(
        env: Env,
        delegate: Address,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
    ) -> Result<String, Error> {
        delegate.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Validation::require_not_paused(&env)?;

        let delegation = Storage::get_delegation(&env, &issuer, &delegate, &claim_type)
            .ok_or(Error::DelegationNotFound)?;

        let current_time = env.ledger().timestamp();
        if let Some(exp) = delegation.expiration {
            if current_time >= exp {
                return Err(Error::DelegationExpired);
            }
        }

        validate_metadata(&metadata)?;
        validate_native_expiration(&env, expiration)?;

        if issuer == subject {
            return Err(Error::Unauthorized);
        }

        let timestamp = current_time;
        let attestation_id =
            Attestation::generate_id(&env, &issuer, &subject, &claim_type, timestamp);

        if Storage::has_attestation(&env, &attestation_id) {
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
            metadata,
            jurisdiction: None,
            valid_from: None,
            imported: false,
            bridged: false,
            source_chain: None,
            source_tx: None,
            tags: None,
            revocation_reason: None,
            deleted: false,
        };

        store_attestation(&env, &attestation);
        Events::attestation_created(&env, &attestation);
        Storage::append_audit_entry(
            &env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Created,
                actor: delegate.clone(),
                timestamp,
                details: None,
            },
        );
        Ok(attestation_id)
    }

    pub fn endorse_attestation(
        env: Env,
        endorser: Address,
        attestation_id: String,
    ) -> Result<(), Error> {
        endorser.require_auth();
        Validation::require_issuer(&env, &endorser)?;
        let attestation = Storage::get_attestation(&env, &attestation_id)?;
        if attestation.issuer == endorser { return Err(Error::CannotEndorseOwn); }
        if attestation.revoked { return Err(Error::AlreadyRevoked); }
        for existing in Storage::get_endorsements(&env, &attestation_id).iter() {
            if existing.endorser == endorser { return Err(Error::AlreadyEndorsed); }
        }
        let timestamp = env.ledger().timestamp();
        let endorsement = Endorsement { attestation_id: attestation_id.clone(), endorser: endorser.clone(), timestamp };
        Storage::add_endorsement(&env, &endorsement);
        Events::attestation_endorsed(&env, &attestation_id, &endorser, timestamp);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Pull-based attestation requests
    // -----------------------------------------------------------------------

    /// Submit a pull-based attestation request to a registered issuer.
    pub fn request_attestation(env: Env, subject: Address, issuer: Address, claim_type: String) -> Result<String, Error> {
        subject.require_auth();
        Validation::require_issuer(&env, &issuer)?;

        let timestamp = env.ledger().timestamp();
        let request_id = AttestationRequest::generate_id(&env, &subject, &issuer, &claim_type, timestamp);

        if Storage::has_attestation_request(&env, &request_id) {
            return Err(Error::DuplicateRequest);
        }

        let request = AttestationRequest {
            id: request_id.clone(),
            subject: subject.clone(),
            issuer: issuer.clone(),
            claim_type: claim_type.clone(),
            timestamp,
            expires_at: timestamp + ATTESTATION_REQUEST_TTL_SECS,
            status: RequestStatus::Pending,
            rejection_reason: None,
        };

        Storage::set_attestation_request(&env, &request);
        Storage::add_pending_request(&env, &issuer, &request_id);
        Events::attestation_requested(&env, &request_id, &subject, &issuer, &claim_type, request.expires_at);
        Ok(request_id)
    }

    /// Fulfill a pending attestation request, creating the attestation.
    pub fn fulfill_request(env: Env, issuer: Address, request_id: String, expiration: Option<u64>) -> Result<String, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;

        let mut request = Storage::get_attestation_request(&env, &request_id)?;

        if request.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if request.status != RequestStatus::Pending {
            return Err(Error::RequestAlreadyProcessed);
        }
        let current_time = env.ledger().timestamp();
        if current_time >= request.expires_at {
            return Err(Error::RequestExpired);
        }

        validate_native_expiration(&env, expiration)?;

        let attestation_id = Attestation::generate_id(&env, &issuer, &request.subject, &request.claim_type, current_time);
        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: issuer.clone(),
            subject: request.subject.clone(),
            claim_type: request.claim_type.clone(),
            timestamp: current_time,
            expiration,
            revoked: false,
            metadata: None,
            jurisdiction: None,
            valid_from: None,
            imported: false,
            bridged: false,
            source_chain: None,
            source_tx: None,
            tags: None,
            revocation_reason: None,
            deleted: false,
        };

        store_attestation(&env, &attestation);
        Events::attestation_created(&env, &attestation);
        Storage::append_audit_entry(&env, &attestation_id, &AuditEntry {
            action: AuditAction::Created,
            actor: issuer.clone(),
            timestamp: current_time,
            details: None,
        });

        request.status = RequestStatus::Fulfilled;
        Storage::set_attestation_request(&env, &request);
        Storage::remove_pending_request(&env, &issuer, &request_id);

        Events::request_fulfilled(&env, &request_id, &issuer, &attestation_id);
        Ok(attestation_id)
    }

    /// Reject a pending attestation request.
    pub fn reject_request(env: Env, issuer: Address, request_id: String, reason: Option<String>) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        validate_reason(&reason)?;

        let mut request = Storage::get_attestation_request(&env, &request_id)?;

        if request.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if request.status != RequestStatus::Pending {
            return Err(Error::RequestAlreadyProcessed);
        }

        request.status = RequestStatus::Rejected;
        request.rejection_reason = reason.clone();
        Storage::set_attestation_request(&env, &request);
        Storage::remove_pending_request(&env, &issuer, &request_id);

        Events::request_rejected(&env, &request_id, &issuer, &reason);
        Ok(())
    }

    /// Return the current storage limits.
    ///
    /// Returns the admin-configured limits, or the defaults
    /// (10,000 per issuer / 100 per subject) if never explicitly set.
    #[must_use]
    pub fn get_limits(env: Env) -> StorageLimits {
        Storage::get_limits(&env)
    }

    /// Return the semver version string set at initialization (e.g. `"1.0.0"`).
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] — contract has not been initialized.
    #[must_use]
    pub fn get_version(env: Env) -> Result<String, Error> {
        Storage::get_version(&env).ok_or(Error::NotInitialized)
    }

    /// Return global contract statistics.
    ///
    /// No authentication required — safe to call from dashboards and analytics tools.
    #[must_use]
    pub fn get_global_stats(env: Env) -> GlobalStats {
        Storage::get_global_stats(&env)
    }

    /// Lightweight health probe for monitoring dashboards and uptime checks.
    ///
    /// No authentication required. Returns `initialized: false` before
    /// `initialize` has been called.
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

    /// Initialize the admin council with a list of members and a quorum threshold.
    ///
    /// Only the contract admin may call this. Can be called once; re-calling
    /// updates the council configuration.
    ///
    /// # Parameters
    /// - `admin` — current administrator (must authorize).
    /// - `members` — addresses eligible to vote on proposals.
    /// - `quorum` — minimum approvals required to execute a proposal.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] / [`Error::NotInitialized`] — admin check fails.
    /// - [`Error::InvalidQuorum`] — quorum is 0 or exceeds member count.
    pub fn init_council(
        env: Env,
        admin: Address,
        members: Vec<Address>,
        quorum: u32,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;

        if quorum == 0 || quorum > members.len() {
            return Err(Error::InvalidQuorum);
        }

        let member_count = members.len();
        let council = AdminCouncil { members, quorum };
        Storage::set_council(&env, &council);
        Events::council_initialized(&env, quorum, member_count);
        Ok(())
    }

    /// Create a new council proposal for a sensitive operation.
    ///
    /// The caller must be a council member. The proposal starts with the
    /// proposer's approval already counted.
    ///
    /// # Parameters
    /// - `proposer` — council member creating the proposal (must authorize).
    /// - `operation` — the [`CouncilOperation`] to execute upon quorum.
    ///
    /// # Returns
    /// The new proposal ID.
    ///
    /// # Errors
    /// - [`Error::CouncilNotInitialized`] — council has not been set up.
    /// - [`Error::Unauthorized`] — caller is not a council member.
    pub fn propose_council_action(
        env: Env,
        proposer: Address,
        operation: CouncilOperation,
    ) -> Result<u32, Error> {
        proposer.require_auth();

        let council = Storage::get_council(&env).ok_or(Error::CouncilNotInitialized)?;

        // Verify proposer is a council member
        let mut is_member = false;
        for m in council.members.iter() {
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

    /// Approve an existing council proposal.
    ///
    /// The caller must be a council member and must not have already approved.
    ///
    /// # Parameters
    /// - `approver` — council member approving (must authorize).
    /// - `proposal_id` — ID of the proposal to approve.
    ///
    /// # Errors
    /// - [`Error::CouncilNotInitialized`] — council has not been set up.
    /// - [`Error::NotFound`] — proposal does not exist.
    /// - [`Error::AlreadyExecuted`] — proposal already executed.
    /// - [`Error::Unauthorized`] — caller is not a council member.
    /// - [`Error::AlreadyApproved`] — caller already approved this proposal.
    pub fn approve_council_action(
        env: Env,
        approver: Address,
        proposal_id: u32,
    ) -> Result<(), Error> {
        approver.require_auth();

        let council = Storage::get_council(&env).ok_or(Error::CouncilNotInitialized)?;
        let mut proposal = Storage::get_proposal(&env, proposal_id).ok_or(Error::NotFound)?;

        if proposal.executed {
            return Err(Error::AlreadyExecuted);
        }

        // Verify approver is a council member
        let mut is_member = false;
        for m in council.members.iter() {
            if m == approver {
                is_member = true;
                break;
            }
        }
        if !is_member {
            return Err(Error::Unauthorized);
        }

        // Check not already approved
        for a in proposal.approvals.iter() {
            if a == approver {
                return Err(Error::AlreadyApproved);
            }
        }

        proposal.approvals.push_back(approver.clone());
        Storage::set_proposal(&env, &proposal);
        Events::proposal_approved(&env, proposal_id, &approver);
        Ok(())
    }

    /// Execute a council proposal once quorum is reached.
    ///
    /// Any council member may trigger execution once enough approvals exist.
    ///
    /// # Parameters
    /// - `executor` — council member triggering execution (must authorize).
    /// - `proposal_id` — ID of the proposal to execute.
    ///
    /// # Errors
    /// - [`Error::CouncilNotInitialized`] — council has not been set up.
    /// - [`Error::NotFound`] — proposal does not exist.
    /// - [`Error::AlreadyExecuted`] — proposal already executed.
    /// - [`Error::Unauthorized`] — caller is not a council member.
    /// - [`Error::QuorumNotReached`] — not enough approvals yet.
    pub fn execute_council_action(
        env: Env,
        executor: Address,
        proposal_id: u32,
    ) -> Result<(), Error> {
        executor.require_auth();

        // Event and audit
        Events::attestation_transferred(&env, &attestation_id, &old_issuer, &new_issuer);
        let timestamp = env.ledger().timestamp();
        Storage::append_audit_entry(
            &env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Transferred,
                actor: admin.clone(),
                timestamp,
                details: None,
            },
        );

        if proposal.approvals.len() < council.quorum {
            return Err(Error::QuorumNotReached);
        }

        // Execute the operation
        match proposal.operation.clone() {
            CouncilOperation::RemoveIssuer(issuer) => {
                Storage::remove_issuer(&env, &issuer);
                // Emit issuer_removed using the first council member as "admin" proxy
                if let Some(first) = council.members.get(0) {
                    Events::issuer_removed(&env, &issuer, &first);
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

    /// Return the current council configuration, or `None` if not initialized.
    pub fn get_council(env: Env) -> Option<AdminCouncil> {
        Storage::get_council(&env)
    }

    /// Return a council proposal by ID, or `None` if not found.
    pub fn get_council_proposal(env: Env, proposal_id: u32) -> Option<CouncilProposal> {
        Storage::get_proposal(&env, proposal_id)
    }
}
