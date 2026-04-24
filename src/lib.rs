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
    AdminCouncil, Attestation, AttestationRequest, AttestationStatus, AuditAction, AuditEntry,
    ClaimTypeInfo, ContractConfig, ContractMetadata, Endorsement, Error, FeeConfig, GlobalStats,
    HealthStatus, IssuerMetadata, IssuerStats, IssuerTier, MultiSigProposal, RateLimitConfig,
    RequestStatus, StorageLimits, TtlConfig, ATTESTATION_REQUEST_TTL_SECS,
    MULTISIG_PROPOSAL_TTL_SECS,
};
use crate::validation::Validation;

const SECS_PER_DAY: u64 = 86_400;

mod callback {
    use soroban_sdk::{contractclient, Address, Env, String};
    #[contractclient(name = "ExpirationCallbackClient")]
    #[allow(dead_code)]
    pub trait ExpirationCallback {
        fn notify_expiring(env: Env, subject: Address, attestation_id: String, expiration: u64);
    }
}
use callback::ExpirationCallbackClient;

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn validate_metadata(metadata: &Option<String>) -> Result<(), Error> {
    if let Some(v) = metadata {
        if v.len() > 256 {
            return Err(Error::MetadataTooLong);
        }
    }
    Ok(())
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

    pub fn is_issuer(env: Env, address: Address) -> bool {
        Storage::is_issuer(&env, &address)
    }

    pub fn get_issuer_stats(env: Env, issuer: Address) -> IssuerStats {
        Storage::get_issuer_stats(&env, &issuer)
    }

    pub fn update_issuer_tier(env: Env, admin: Address, issuer: Address, tier: IssuerTier) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Validation::require_issuer(&env, &issuer)?;
        Storage::set_issuer_tier(&env, &issuer, &tier);
        Events::issuer_tier_updated(&env, &issuer, &tier);
        Ok(())
    }

    pub fn get_issuer_tier(env: Env, issuer: Address) -> Option<IssuerTier> {
        Storage::get_issuer_tier(&env, &issuer)
    }

    pub fn set_issuer_metadata(env: Env, issuer: Address, metadata: IssuerMetadata) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::set_issuer_metadata(&env, &issuer, &metadata);
        Ok(())
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

    pub fn add_to_whitelist(env: Env, issuer: Address, subject: Address) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::add_to_whitelist(&env, &issuer, &subject);
        Events::whitelist_updated(&env, &issuer, &subject, true);
        Ok(())
    }

    pub fn remove_from_whitelist(env: Env, issuer: Address, subject: Address) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::remove_from_whitelist(&env, &issuer, &subject);
        Events::whitelist_updated(&env, &issuer, &subject, false);
        Ok(())
    }

    pub fn is_whitelisted(env: Env, issuer: Address, subject: Address) -> bool {
        Storage::is_whitelisted(&env, &issuer, &subject)
    }

    pub fn is_whitelist_enabled(env: Env, issuer: Address) -> bool {
        Storage::is_whitelist_enabled(&env, &issuer)
    }

    // -----------------------------------------------------------------------
    // Fee & rate-limit configuration
    // -----------------------------------------------------------------------

    pub fn set_fee(env: Env, admin: Address, fee: i128, collector: Address, fee_token: Option<Address>) -> Result<(), Error> {
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
        Validation::require_not_paused(env)?;
        Validation::require_issuer(env, &issuer)?;
        validate_claim_type(&claim_type)?;
        validate_metadata(&metadata)?;
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
            metadata,
            jurisdiction,
            valid_from: None,
            imported: false,
            bridged: false,
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

        let mut ids: Vec<String> = Vec::new(&env);
        for subject in subjects.iter() {
            let attestation_id = Attestation::generate_id(&env, &issuer, &subject, &claim_type, timestamp);
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
                timestamp,
                details: None,
            });
            ids.push_back(attestation_id);
        }
        Storage::set_last_issuance_time(&env, &issuer, timestamp);
        Ok(ids)
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
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Validation::require_issuer(&env, &issuer)?;
        validate_import_timestamps(&env, timestamp, expiration)?;

        let attestation_id = Attestation::generate_id(&env, &issuer, &subject, &claim_type, timestamp);
        if Storage::has_attestation(&env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer,
            subject,
            claim_type,
            timestamp,
            expiration,
            revoked: false,
            metadata: None,
            jurisdiction: None,
            valid_from: None,
            imported: true,
            bridged: false,
            source_chain: None,
            source_tx: None,
            tags: None,
            revocation_reason: None,
            deleted: false,
        };

        store_attestation(&env, &attestation);
        Events::attestation_imported(&env, &attestation);
        Storage::append_audit_entry(&env, &attestation_id, &AuditEntry {
            action: AuditAction::Created,
            actor: admin.clone(),
            timestamp,
            details: None,
        });
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
            metadata: None,
            jurisdiction: None,
            valid_from: None,
            imported: false,
            bridged: true,
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

    // -----------------------------------------------------------------------
    // Revoke / renew / update expiration  (issues #272, #273, #274)
    // -----------------------------------------------------------------------

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
            return Err(Error::BatchTooLarge);
        }

        // Validate all first (atomic — no partial writes).
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
            Events::attestation_revoked(&env, &id, &issuer, &reason);
            Storage::append_audit_entry(&env, &id, &AuditEntry {
                action: AuditAction::Revoked,
                actor: issuer.clone(),
                timestamp: env.ledger().timestamp(),
                details: reason.clone(),
            });
            count += 1;
        }
        Storage::increment_total_revocations(&env, count as u64);
        Ok(count)
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

    /// Update the expiration of an attestation.
    ///
    /// SECURITY (FINDING-003): `require_issuer` ensures the caller is a
    /// currently registered issuer, preventing deregistered issuers from
    /// modifying attestation expiration.
    pub fn update_expiration(
        env: Env,
        issuer: Address,
        attestation_id: String,
        new_expiration: Option<u64>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        // FINDING-003 fix: verify caller is a currently registered issuer.
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
        Events::attestation_updated(&env, &attestation_id, &issuer, new_expiration);
        Storage::append_audit_entry(&env, &attestation_id, &AuditEntry {
            action: AuditAction::Updated,
            actor: issuer.clone(),
            timestamp: env.ledger().timestamp(),
            details: None,
        });
        Ok(())
    }

    pub fn transfer_attestation(
        env: Env,
        admin: Address,
        attestation_id: String,
        new_issuer: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;
        let old_issuer = attestation.issuer.clone();
        Validation::require_issuer(&env, &new_issuer)?;

        if old_issuer == new_issuer {
            return Ok(());
        }

        Storage::remove_issuer_attestation(&env, &old_issuer, &attestation_id);
        let mut old_stats = Storage::get_issuer_stats(&env, &old_issuer);
        old_stats.total_issued = old_stats.total_issued.saturating_sub(1);
        Storage::set_issuer_stats(&env, &old_issuer, &old_stats);

        attestation.issuer = new_issuer.clone();
        Storage::set_attestation(&env, &attestation);

        Storage::add_issuer_attestation(&env, &new_issuer, &attestation_id);
        let mut new_stats = Storage::get_issuer_stats(&env, &new_issuer);
        new_stats.total_issued += 1;
        Storage::set_issuer_stats(&env, &new_issuer, &new_stats);

        Events::attestation_transferred(&env, &attestation_id, &old_issuer, &new_issuer);
        Storage::append_audit_entry(&env, &attestation_id, &AuditEntry {
            action: AuditAction::Transferred,
            actor: admin.clone(),
            timestamp: env.ledger().timestamp(),
            details: None,
        });
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Query functions (read-only)
    // -----------------------------------------------------------------------

    pub fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool {
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

    pub fn has_valid_claim_from_tier(env: Env, subject: Address, claim_type: String, min_tier: IssuerTier) -> bool {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        let min_rank = min_tier.rank();
        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if attestation.deleted || attestation.claim_type != claim_type { continue; }
                if attestation.get_status(current_time) != AttestationStatus::Valid { continue; }
                if let Some(tier) = Storage::get_issuer_tier(&env, &attestation.issuer) {
                    if tier.rank() >= min_rank { return true; }
                }
            }
        }
        false
    }

    pub fn has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool {
        if claim_types.is_empty() { return false; }
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

    pub fn get_attestation(env: Env, attestation_id: String) -> Result<Attestation, Error> {
        Storage::get_attestation(&env, &attestation_id)
    }

    pub fn get_attestation_status(env: Env, attestation_id: String) -> Result<AttestationStatus, Error> {
        let attestation = Storage::get_attestation(&env, &attestation_id)?;
        let status = attestation.get_status(env.ledger().timestamp());
        if status == AttestationStatus::Expired {
            Events::attestation_expired(&env, &attestation_id, &attestation.subject);
        }
        Ok(status)
    }

    pub fn get_subject_attestations(env: Env, subject: Address, start: u32, limit: u32) -> Vec<String> {
        crate::storage::paginate(&env, &Storage::get_subject_attestations(&env, &subject), start, limit)
    }

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
        let paginated = crate::storage::paginate(&env, &filtered, start, limit);
        let mut result = Vec::new(&env);
        for id in paginated.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                result.push_back(attestation);
            }
        }
        result
    }

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

    pub fn get_attestations_by_jurisdiction(env: Env, subject: Address, jurisdiction: String) -> Vec<String> {
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

    pub fn get_attestation_by_type(env: Env, subject: Address, claim_type: String) -> Result<Attestation, Error> {
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
        Err(Error::NotFound)
    }

    pub fn get_expiring_attestations(env: Env, subject: Address, within_seconds: u64) -> Vec<String> {
        let current_time = env.ledger().timestamp();
        let upper = current_time + within_seconds;
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let mut result = Vec::new(&env);
        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if attestation.deleted || attestation.revoked { continue; }
                if let Some(exp) = attestation.expiration {
                    if exp > current_time && exp <= upper {
                        result.push_back(id);
                    }
                }
            }
        }
        result
    }

    pub fn get_issuer_expiring_attestations(env: Env, issuer: Address, within_seconds: u64) -> Vec<String> {
        let current_time = env.ledger().timestamp();
        let upper = current_time + within_seconds;
        let attestation_ids = Storage::get_issuer_attestations(&env, &issuer);
        let mut result = Vec::new(&env);
        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if attestation.deleted || attestation.revoked { continue; }
                if let Some(exp) = attestation.expiration {
                    if exp > current_time && exp <= upper {
                        result.push_back(id);
                    }
                }
            }
        }
        result
    }

    pub fn get_audit_log(env: Env, attestation_id: String) -> Vec<AuditEntry> {
        Storage::get_audit_log(&env, &attestation_id)
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

    // -----------------------------------------------------------------------
    // Claim type registry
    // -----------------------------------------------------------------------

    pub fn register_claim_type(env: Env, admin: Address, claim_type: String, description: String) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        validate_claim_type(&claim_type)?;
        let info = ClaimTypeInfo { claim_type: claim_type.clone(), description: description.clone() };
        Storage::set_claim_type(&env, &info);
        Events::claim_type_registered(&env, &claim_type, &description);
        Ok(())
    }

    pub fn get_claim_type_description(env: Env, claim_type: String) -> Option<String> {
        Storage::get_claim_type(&env, &claim_type).map(|info| info.description)
    }

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
            Events::multisig_activated(&env, &proposal_id, &attestation_id);
        } else {
            Storage::set_multisig_proposal(&env, &proposal);
        }
        Ok(())
    }

    pub fn get_multisig_proposal(env: Env, proposal_id: String) -> Result<MultiSigProposal, Error> {
        Storage::get_multisig_proposal(&env, &proposal_id)
    }

    // -----------------------------------------------------------------------
    // Endorsements
    // -----------------------------------------------------------------------

    pub fn endorse_attestation(env: Env, endorser: Address, attestation_id: String) -> Result<(), Error> {
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

    pub fn get_attestation_request(env: Env, request_id: String) -> Result<AttestationRequest, Error> {
        Storage::get_attestation_request(&env, &request_id)
    }

    pub fn get_pending_requests(env: Env, issuer: Address, start: u32, limit: u32) -> Vec<String> {
        let current_time = env.ledger().timestamp();
        let all = Storage::get_pending_requests(&env, &issuer);
        // Filter out expired and non-pending requests.
        let mut active = Vec::new(&env);
        for id in all.iter() {
            if let Ok(req) = Storage::get_attestation_request(&env, &id) {
                if req.status == RequestStatus::Pending && current_time < req.expires_at {
                    active.push_back(id);
                }
            }
        }
        crate::storage::paginate(&env, &active, start, limit)
    }

    // -----------------------------------------------------------------------
    // Metadata & status
    // -----------------------------------------------------------------------

    pub fn get_version(env: Env) -> Result<String, Error> {
        Storage::get_version(&env).ok_or(Error::NotInitialized)
    }

    pub fn get_global_stats(env: Env) -> GlobalStats {
        Storage::get_global_stats(&env)
    }

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

    pub fn get_config(env: Env) -> ContractConfig {
        let ttl_config = Storage::get_ttl_config(&env).unwrap_or(TtlConfig { ttl_days: 30 });
        let fee_config = Storage::get_fee_config(&env).unwrap_or_else(|| FeeConfig {
            attestation_fee: 0,
            fee_collector: env.current_contract_address(),
            fee_token: None,
        });
        let version = Storage::get_version(&env).unwrap_or_else(|| String::from_str(&env, ""));
        ContractConfig {
            ttl_config,
            fee_config,
            contract_name: String::from_str(&env, "TrustLink"),
            contract_version: version,
            contract_description: String::from_str(&env, "On-chain attestation and verification system for the Stellar blockchain."),
        }
    }
}
