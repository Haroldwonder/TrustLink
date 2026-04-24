//! Storage helpers for TrustLink.
//!
//! This module is the single point of contact between the contract logic and
//! on-chain storage. No other module calls `env.storage()` directly.
//!
//! ## Storage tiers
//!
//! | Tier         | Keys stored                          | TTL policy                        |
//! |--------------|--------------------------------------|-----------------------------------|
//! | Instance     | `Admin`, `Version`, `FeeConfig`, `GlobalStats` | Refreshed to 30 days on each write|
//! | Persistent   | Everything else (see [`StorageKey`]) | Refreshed to 30 days on each write|
//!
//! ## Key layout (`StorageKey`)
//!
//! - `Admin` — the single contract administrator address.
//! - `Version` — semver string set at initialization (e.g. `"1.0.0"`).
//! - `Issuer(Address)` — presence flag (`bool`) for each registered issuer.
//! - `Bridge(Address)` — presence flag (`bool`) for each registered bridge contract.
//! - `Attestation(String)` — full [`Attestation`] record keyed by its ID.
//! - `SubjectAttestations(Address)` — ordered `Vec<String>` of attestation IDs
//!   for a given subject; used for pagination and claim lookups.
//! - `IssuerAttestations(Address)` — ordered `Vec<String>` of attestation IDs
//!   created by a given issuer.
//! - `IssuerMetadata(Address)` — optional [`IssuerMetadata`] set by the issuer.
//! - `ClaimType(String)` — [`ClaimTypeInfo`] record for a registered claim type.
//! - `ClaimTypeList` — ordered `Vec<String>` of all registered claim type IDs;
//!   used for pagination via `list_claim_types`.
//! - `FeeConfig` — global attestation fee settings.
//! - `GlobalStats` — running counters for total attestations, revocations, and issuers.

use crate::types::{
    AdminCouncil, Attestation, AttestationRequest, AuditEntry, ClaimTypeInfo, Endorsement, Error, ExpirationHook,
    FeeConfig, GlobalStats, IssuerMetadata, IssuerStats, IssuerTier, MultiSigProposal, TtlConfig, Delegation, RateLimitConfig,
};
use soroban_sdk::{contracttype, Address, Env, String, Vec};
use crate::types::{Attestation, ClaimTypeInfo, Error, IssuerMetadata, StorageLimits, Delegation};

/// Keys used to address data in contract storage.
#[contracttype]
pub enum StorageKey {
    /// The contract administrator address (legacy - now using AdminCouncil).
    Admin,
    /// List of admin addresses (multi-admin council).
    AdminCouncil,
    /// Semver version string set at initialization.
    Version,
    /// Global attestation fee settings.
    FeeConfig,
    /// TTL configuration (days).
    TtlConfig,
    /// Presence flag for a registered issuer.
    Issuer(Address),
    /// Presence flag for a registered bridge contract.
    Bridge(Address),
    /// Full [`Attestation`] record keyed by its ID.
    Attestation(String),
    /// Ordered list of attestation IDs for a subject address.
    SubjectAttestations(Address),
    /// Ordered list of attestation IDs created by an issuer address.
    IssuerAttestations(Address),
    /// Optional metadata associated with a registered issuer.
    IssuerMetadata(Address),
    /// Info for a registered claim type.
    ClaimType(String),
    /// Ordered list of registered claim type identifiers.
    ClaimTypeList,
    /// Configurable storage limits (admin-settable).
    Limits,
    /// A multi-sig attestation proposal keyed by its ID.
    MultiSigProposal(String),
    /// Ordered list of endorsements for an attestation, keyed by attestation ID.
    Endorsements(String),
    /// Global contract statistics (total attestations, revocations, issuers).
    GlobalStats,
    /// Trust tier for a registered issuer.
    IssuerTier(Address),
    /// Per-issuer statistics keyed by issuer address.
    IssuerStats(Address),
    /// Expiration notification hook for a subject address.
    ExpirationHook(Address),
    /// Append-only audit log for an attestation, keyed by attestation ID.
    AuditLog(String),
    /// Global pause flag — when present and true, write operations are disabled.
    Paused,
    /// Whitelist enabled flag per issuer — when true, only whitelisted subjects are accepted.
    WhitelistEnabled(Address),
    /// Presence flag for a whitelisted subject under a specific issuer.
    SubjectWhitelist(Address, Address),
    /// Delegation from delegator to delegate for specific claim_type.
    Delegation((Address, Address, String)),
    /// Rate limit configuration (global).
    RateLimit,
    /// Last attestation issuance timestamp for an issuer.
    LastIssuanceTime(Address),
}

const DAY_IN_LEDGERS: u32 = 17280;
const DEFAULT_TTL_DAYS: u32 = 30;
const DEFAULT_INSTANCE_LIFETIME: u32 = DAY_IN_LEDGERS * DEFAULT_TTL_DAYS;
// Only extend TTL on read if remaining TTL drops below this threshold (7 days)
#[allow(dead_code)]
const MIN_TTL_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;

/// Get the TTL in ledgers for the configured number of days.
fn get_ttl_lifetime(env: &Env) -> u32 {
    if let Some(config) = env
        .storage()
        .instance()
        .get::<StorageKey, TtlConfig>(&StorageKey::TtlConfig)
    {
        DAY_IN_LEDGERS * config.ttl_days
    } else {
        DEFAULT_INSTANCE_LIFETIME
    }
}

/// Low-level storage operations for TrustLink state.
///
/// All methods take `&Env` and operate on the appropriate storage tier
/// (instance for admin, persistent for everything else).
pub struct Storage;

impl Storage {
    /// Return `true` if admin council is initialized (has >=1 admins).
    pub fn has_admin(env: &Env) -> bool {
        if let Ok(council) = Self::get_admin_council(env) {
            !council.is_empty()
        } else {
            false
        }
    }

    /// Legacy: Persist single `admin` (deprecated, use AdminCouncil).
    pub fn set_admin(env: &Env, admin: &Address) {
        let ttl = get_ttl_lifetime(env);
        let mut council = Vec::new(env);
        council.push_back(admin.clone());
        Self::set_admin_council(env, &council);
    }

    /// Retrieve the admin council (Vec<Address>).
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] if council key absent.
    pub fn get_admin_council(env: &Env) -> Result<AdminCouncil, Error> {
        env.storage()
            .instance()
            .get(&amp;StorageKey::AdminCouncil)
            .ok_or(Error::NotInitialized)
    }

    /// Persist the admin council and refresh TTL.
    pub fn set_admin_council(env: &amp;Env, council: &amp;AdminCouncil) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&amp;StorageKey::AdminCouncil, council);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    /// Return true if `address` is an admin in the council.
    pub fn is_admin(env: &amp;Env, address: &amp;Address) -> bool {
        if let Ok(council) = Self::get_admin_council(env) {
            for admin in council.iter() {
                if admin == address {
                    return true;
                }
            }
        }
        false
    }

    /// Add `admin` to council if not already present.
    pub fn add_admin(env: &amp;Env, admin: &amp;Address) {
        let mut council = Self::get_admin_council(env).unwrap_or(Vec::new(env));
        let mut found = false;
        for a in council.iter() {
            if a == admin {
                found = true;
                break;
            }
        }
        if !found {
            council.push_back(admin.clone());
            Self::set_admin_council(env, &amp;council);
        }
    }

    /// Remove `admin` from council if present.
    pub fn remove_admin(env: &amp;Env, admin: &amp;Address) {
        let mut council = Self::get_admin_council(env).unwrap_or(Vec::new(env));
        let mut new_council = Vec::new(env);
        let mut found = false;
        for a in council.iter() {
            if a != admin {
                new_council.push_back(a.clone());
            } else {
                found = true;
            }
        }
        if found {
            Self::set_admin_council(env, &amp;new_council);
        }
    }

    /// Persist `version` in instance storage alongside the admin.
    pub fn set_version(env: &Env, version: &String) {
        env.storage().instance().set(&StorageKey::Version, version);
    }

    /// Persist the attestation fee configuration.
    pub fn set_fee_config(env: &Env, fee_config: &FeeConfig) {
        let ttl = get_ttl_lifetime(env);
        env.storage()
            .instance()
            .set(&StorageKey::FeeConfig, fee_config);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    /// Persist the TTL configuration.
    pub fn set_ttl_config(env: &Env, ttl_config: &TtlConfig) {
        let ttl = get_ttl_lifetime(env);
        env.storage()
            .instance()
            .set(&StorageKey::TtlConfig, ttl_config);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    /// Retrieve the contract version string.
    ///
    /// Returns `None` if the contract has not been initialized yet.
    pub fn get_version(env: &Env) -> Option<String> {
        env.storage().instance().get(&StorageKey::Version)
    }

    /// Retrieve the current attestation fee configuration.
    pub fn get_fee_config(env: &Env) -> Option<FeeConfig> {
        env.storage().instance().get(&StorageKey::FeeConfig)
    }

    /// Retrieve the current TTL configuration.
    pub fn get_ttl_config(env: &Env) -> Option<TtlConfig> {
        env.storage().instance().get(&StorageKey::TtlConfig)
    }

    /// Retrieve the primary admin address (council[0]).
    ///
    /// Backward compatible with single-admin. Returns Error if council empty.
    /// # Errors
    /// - [`Error::NotInitialized`] — council empty.
    pub fn get_admin(env: &Env) -> Result<Address, Error> {
        let council = Self::get_admin_council(env)?;
        council.first().cloned().ok_or(Error::NotInitialized)
    }

    /// Return `true` if `address` is in the issuer registry.
    pub fn is_issuer(env: &Env, address: &Address) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::Issuer(address.clone()))
    }

    /// Add `issuer` to the registry and refresh its TTL.
    pub fn add_issuer(env: &Env, issuer: &Address) {
        let key = StorageKey::Issuer(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Return `true` if `address` is in the bridge registry.
    pub fn is_bridge(env: &Env, address: &Address) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::Bridge(address.clone()))
    }

    /// Add `bridge` to the registry and refresh its TTL.
    pub fn add_bridge(env: &Env, bridge: &Address) {
        let key = StorageKey::Bridge(bridge.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Remove `issuer` from the registry.
    pub fn remove_issuer(env: &Env, issuer: &Address) {
        env.storage()
            .persistent()
            .remove(&StorageKey::Issuer(issuer.clone()));
    }

    /// Return `true` if an attestation with `id` exists in storage.
    pub fn has_attestation(env: &Env, id: &String) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::Attestation(id.clone()))
    }

    /// Persist `attestation` and refresh its TTL.
    pub fn set_attestation(env: &Env, attestation: &Attestation) {
        let key = StorageKey::Attestation(attestation.id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, attestation);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve an attestation by `id`. TTL is not extended on read to reduce
    /// compute costs; TTL will be refreshed when the attestation is modified.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — no attestation with that ID exists.
    pub fn get_attestation(env: &Env, id: &String) -> Result<Attestation, Error> {
        let key = StorageKey::Attestation(id.clone());
        env.storage().persistent().get(&key).ok_or(Error::NotFound)
    }

    /// Return the ordered list of attestation IDs for `subject`, or an empty
    /// [`Vec`] if none exist. TTL is only extended on index modification,
    /// not on read, to reduce compute costs for frequent queries.
    pub fn get_subject_attestations(env: &Env, subject: &Address) -> Vec<String> {
        let key = StorageKey::SubjectAttestations(subject.clone());
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env))
    }

    /// Append `attestation_id` to `subject`'s attestation index and refresh TTL.
    pub fn add_subject_attestation(env: &Env, subject: &Address, attestation_id: &String) {
        let key = StorageKey::SubjectAttestations(subject.clone());
        let ttl = get_ttl_lifetime(env);
        let mut attestations = Self::get_subject_attestations(env, subject);
        attestations.push_back(attestation_id.clone());
        env.storage().persistent().set(&key, &attestations);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Remove `attestation_id` from `subject`'s attestation index.
    pub fn remove_subject_attestation(env: &Env, subject: &Address, attestation_id: &String) {
        let key = StorageKey::SubjectAttestations(subject.clone());
        let ttl = get_ttl_lifetime(env);
        let existing = Self::get_subject_attestations(env, subject);
        let mut updated = Vec::new(env);
        for id in existing.iter() {
            if &id != attestation_id {
                updated.push_back(id);
            }
        }
        env.storage().persistent().set(&key, &updated);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Remove `attestation_id` from `issuer`'s attestation index.
    ///
    /// Note: this does not delete the attestation record; it only removes the ID
    /// from the issuer's listing index so pagination results shrink.
    pub fn remove_issuer_attestation(env: &Env, issuer: &Address, attestation_id: &String) {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let existing = Self::get_issuer_attestations(env, issuer);
        let mut updated = Vec::new(env);
        for id in existing.iter() {
            if &id != attestation_id {
                updated.push_back(id);
            }
        }
        env.storage().persistent().set(&key, &updated);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Return the ordered list of attestation IDs created by `issuer`, or an
    /// empty [`Vec`] if none exist. TTL is only extended on index modification,
    /// not on read, to reduce compute costs for frequent queries.
    pub fn get_issuer_attestations(env: &Env, issuer: &Address) -> Vec<String> {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env))
    }

    /// Append `attestation_id` to `issuer`'s attestation index and refresh TTL.
    pub fn add_issuer_attestation(env: &Env, issuer: &Address, attestation_id: &String) {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut attestations = Self::get_issuer_attestations(env, issuer);
        attestations.push_back(attestation_id.clone());
        env.storage().persistent().set(&key, &attestations);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Remove `attestation_id` from `issuer`'s attestation index.
    pub fn remove_issuer_attestation(env: &Env, issuer: &Address, attestation_id: &String) {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let existing = Self::get_issuer_attestations(env, issuer);
        let mut updated = Vec::new(env);
        for id in existing.iter() {
            if &id != attestation_id {
                updated.push_back(id);
            }
        }
        env.storage().persistent().set(&key, &updated);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Persist `metadata` for `issuer` and refresh its TTL.
    pub fn set_issuer_metadata(env: &Env, issuer: &Address, metadata: &IssuerMetadata) {
        let key = StorageKey::IssuerMetadata(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, metadata);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve metadata for `issuer`, or `None` if not set.
    pub fn get_issuer_metadata(env: &Env, issuer: &Address) -> Option<IssuerMetadata> {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerMetadata(issuer.clone()))
    }

    /// Persist a [`ClaimTypeInfo`] and add its identifier to the ordered list.
    /// Persist a claim type info record and add it to the ordered list if new.
    pub fn set_claim_type(env: &Env, info: &ClaimTypeInfo) {
        let key = StorageKey::ClaimType(info.claim_type.clone());
        let is_new = !env.storage().persistent().has(&key);
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, info);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
        if is_new {
            let list_key = StorageKey::ClaimTypeList;
            let mut list: Vec<String> = env
                .storage()
                .persistent()
                .get(&list_key)
                .unwrap_or(Vec::new(env));
            list.push_back(info.claim_type.clone());
            env.storage().persistent().set(&list_key, &list);
            env.storage().persistent().extend_ttl(&list_key, ttl, ttl);
        }
    }

    /// Retrieve a [`ClaimTypeInfo`] by identifier, or `None` if not registered
    /// Retrieve a claim type info record, or `None` if not registered.
    pub fn get_claim_type(env: &Env, claim_type: &String) -> Option<ClaimTypeInfo> {
        env.storage()
            .persistent()
            .get(&StorageKey::ClaimType(claim_type.clone()))
    }

    /// Return the ordered list of registered claim type identifiers.
    pub fn get_claim_type_list(env: &Env) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&StorageKey::ClaimTypeList)
            .unwrap_or(Vec::new(env))
    }

    /// Persist storage limits in instance storage.
    pub fn set_limits(env: &Env, limits: &StorageLimits) {
        env.storage().instance().set(&StorageKey::Limits, limits);
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }

    /// Retrieve storage limits, returning defaults if never set.
    pub fn get_limits(env: &Env) -> StorageLimits {
        env.storage()
            .instance()
            .get(&StorageKey::Limits)
            .unwrap_or_default()
    }

    /// Persist updated global stats to instance storage and refresh TTL.
    fn set_global_stats(env: &Env, stats: &GlobalStats) {
        let ttl = get_ttl_lifetime(env);
        env.storage()
            .instance()
            .set(&StorageKey::GlobalStats, stats);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    /// Increment `total_attestations` by `count`.
    pub fn increment_total_attestations(env: &Env, count: u64) {
        let mut stats = Self::get_global_stats(env);
        stats.total_attestations += count;
        Self::set_global_stats(env, &stats);
    }

    /// Increment `total_revocations` by `count`.
    pub fn increment_total_revocations(env: &Env, count: u64) {
        let mut stats = Self::get_global_stats(env);
        stats.total_revocations += count;
        Self::set_global_stats(env, &stats);
    }

    /// Increment `total_issuers` by 1 when a new issuer is registered.
    pub fn increment_total_issuers(env: &Env) {
        let mut stats = Self::get_global_stats(env);
        stats.total_issuers += 1;
        Self::set_global_stats(env, &stats);
    }

    /// Decrement `total_issuers` by 1 when an issuer is removed (saturating at 0).
    pub fn decrement_total_issuers(env: &Env) {
        let mut stats = Self::get_global_stats(env);
        stats.total_issuers = stats.total_issuers.saturating_sub(1);
        Self::set_global_stats(env, &stats);
    }

    /// Persist the trust tier for `issuer`.
    pub fn set_issuer_tier(env: &Env, issuer: &Address, tier: &IssuerTier) {
        let key = StorageKey::IssuerTier(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, tier);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve the trust tier for `issuer`, or `None` if not set.
    pub fn get_issuer_tier(env: &Env, issuer: &Address) -> Option<IssuerTier> {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerTier(issuer.clone()))
    }

    /// Retrieve per-issuer stats, returning zeroed defaults if not yet set.
    pub fn get_issuer_stats(env: &Env, issuer: &Address) -> IssuerStats {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerStats(issuer.clone()))
            .unwrap_or(IssuerStats { total_issued: 0 })
    }

    /// Persist per-issuer stats.
    pub fn set_issuer_stats(env: &Env, issuer: &Address, stats: &IssuerStats) {
        let key = StorageKey::IssuerStats(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, stats);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve the expiration hook for `subject`, or `None` if not registered.
    pub fn get_expiration_hook(env: &Env, subject: &Address) -> Option<ExpirationHook> {
        env.storage()
            .persistent()
            .get(&StorageKey::ExpirationHook(subject.clone()))
    }

    /// Persist an expiration hook for `subject`.
    #[allow(dead_code)]
    pub fn set_expiration_hook(env: &Env, subject: &Address, hook: &ExpirationHook) {
        let key = StorageKey::ExpirationHook(subject.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, hook);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Return the audit log for `attestation_id`, or an empty [`Vec`] if none exist.
    pub fn get_audit_log(env: &Env, attestation_id: &String) -> Vec<AuditEntry> {
        env.storage()
            .persistent()
            .get(&StorageKey::AuditLog(attestation_id.clone()))
            .unwrap_or(Vec::new(env))
    }

    /// Append `entry` to the audit log for `attestation_id` (append-only).
    pub fn append_audit_entry(env: &Env, attestation_id: &String, entry: &AuditEntry) {
        let key = StorageKey::AuditLog(attestation_id.clone());
        let ttl = get_ttl_lifetime(env);
        let mut log = Self::get_audit_log(env, attestation_id);
        log.push_back(entry.clone());
        env.storage().persistent().set(&key, &log);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Return the ordered list of endorsements for `attestation_id`, or an empty [`Vec`] if none.
    pub fn get_endorsements(env: &Env, attestation_id: &String) -> Vec<Endorsement> {
        let key = StorageKey::Endorsements(attestation_id.clone());
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env))
    }

    /// Append `endorsement` to the endorsements list for its attestation and refresh TTL.
    pub fn add_endorsement(env: &Env, endorsement: &Endorsement) {
        let key = StorageKey::Endorsements(endorsement.attestation_id.clone());
        let ttl = get_ttl_lifetime(env);
        let mut endorsements = Self::get_endorsements(env, &endorsement.attestation_id);
        endorsements.push_back(endorsement.clone());
        env.storage().persistent().set(&key, &endorsements);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Return `true` if the contract is currently paused.
    ///
    /// Defaults to `false` (not paused) when the key is absent.
    pub fn is_paused(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&StorageKey::Paused)
            .unwrap_or(false)
    }

    /// Set the contract pause state and refresh the instance TTL.
    pub fn set_paused(env: &Env, paused: bool) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::Paused, &paused);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    /// Return `true` if the issuer has whitelist mode enabled.
    ///
    /// Defaults to `false` (disabled) when the key is absent.
    pub fn is_whitelist_enabled(env: &Env, issuer: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::WhitelistEnabled(issuer.clone()))
            .unwrap_or(false)
    }

    /// Enable or disable whitelist mode for `issuer`.
    pub fn set_whitelist_enabled(env: &Env, issuer: &Address, enabled: bool) {
        let key = StorageKey::WhitelistEnabled(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &enabled);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Return `true` if `subject` is whitelisted under `issuer`.
    pub fn is_subject_whitelisted(env: &Env, issuer: &Address, subject: &Address) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::SubjectWhitelist(issuer.clone(), subject.clone()))
    }

    /// Add `subject` to `issuer`'s whitelist.
    pub fn add_subject_to_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        let key = StorageKey::SubjectWhitelist(issuer.clone(), subject.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Remove `subject` from `issuer`'s whitelist.
    pub fn remove_subject_from_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        env.storage()
            .persistent()
            .remove(&StorageKey::SubjectWhitelist(issuer.clone(), subject.clone()));
    }

    /// Persist delegation from `delegator` to `delegate` for `claim_type`.
    pub fn set_delegation(
        env: &Env,
        delegator: &Address,
        delegate: &Address,
        claim_type: &String,
        delegation: &Delegation,
    ) {
        let key = StorageKey::Delegation((delegator.clone(), delegate.clone(), claim_type.clone()));
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, delegation);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve delegation, or None if not found.
    pub fn get_delegation(
        env: &Env,
        delegator: &Address,
        delegate: &Address,
        claim_type: &String,
    ) -> Option<Delegation> {
        let key = StorageKey::Delegation((delegator.clone(), delegate.clone(), claim_type.clone()));
        env.storage().persistent().get(&key)
    }

    /// Remove delegation.
    pub fn remove_delegation(
        env: &Env,
        delegator: &Address,
        delegate: &Address,
        claim_type: &String,
    ) {
        let key = StorageKey::Delegation((delegator.clone(), delegate.clone(), claim_type.clone()));
        env.storage().persistent().remove(&key);
    }

    /// Persist rate limit configuration in instance storage.
    pub fn set_rate_limit_config(env: &Env, config: &RateLimitConfig) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::RateLimit, config);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    /// Retrieve rate limit configuration, or `None` if not set.
    pub fn get_rate_limit_config(env: &Env) -> Option<RateLimitConfig> {
        env.storage().instance().get(&StorageKey::RateLimit)
    }

    /// Record the current timestamp as the last issuance time for an issuer.
    pub fn set_last_issuance_time(env: &Env, issuer: &Address, timestamp: u64) {
        let key = StorageKey::LastIssuanceTime(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &timestamp);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve the last attestation issuance timestamp for an issuer, or `None` if never issued.
    pub fn get_last_issuance_time(env: &Env, issuer: &Address) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&StorageKey::LastIssuanceTime(issuer.clone()))
    }
}

/// Return a paginated window of `values` starting at index `start` for up to
/// `limit` items. Returns an empty vec if `start >= values.len()`.
pub(crate) fn paginate<T: Clone>(env: &Env, values: &Vec<T>, start: u32, limit: u32) -> Vec<T> {
    let total = values.len();
    if start >= total {
        return Vec::new(env);
    }
    let end = (start + limit).min(total);
    let mut result = Vec::new(env);
    for index in start..end {
        if let Some(value) = values.get(index) {
            result.push_back(value.clone());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    fn make_vec(env: &Env, items: &[&str]) -> Vec<String> {
        let mut v = Vec::new(env);
        for s in items {
            v.push_back(String::from_str(env, s));
        }
        v
    }

    #[test]
    fn paginate_normal_slice() {
        let env = Env::default();
        let input = make_vec(&env, &["a", "b", "c", "d", "e"]);
        let result = paginate(&env, &input, 1, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result.get(0).unwrap(), String::from_str(&env, "b"));
        assert_eq!(result.get(1).unwrap(), String::from_str(&env, "c"));
        assert_eq!(result.get(2).unwrap(), String::from_str(&env, "d"));
    }

    #[test]
    fn paginate_empty_input() {
        let env = Env::default();
        let input: Vec<String> = Vec::new(&env);
        let result = paginate(&env, &input, 0, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn paginate_start_beyond_length() {
        let env = Env::default();
        let input = make_vec(&env, &["a", "b"]);
        let result = paginate(&env, &input, 10, 5);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn paginate_limit_overflow() {
        let env = Env::default();
        let input = make_vec(&env, &["a", "b", "c"]);
        let result = paginate(&env, &input, 1, 100);
        assert_eq!(result.len(), 2);
        assert_eq!(result.get(0).unwrap(), String::from_str(&env, "b"));
        assert_eq!(result.get(1).unwrap(), String::from_str(&env, "c"));
    }

    #[test]
    fn paginate_start_zero_full_limit() {
        let env = Env::default();
        let input = make_vec(&env, &["x", "y", "z"]);
        let result = paginate(&env, &input, 0, 3);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn paginate_start_equals_length() {
        let env = Env::default();
        let input = make_vec(&env, &["a", "b", "c"]);
        let result = paginate(&env, &input, 3, 5);
        assert_eq!(result.len(), 0);
    }

    /// Return `true` if the contract is currently paused.
    pub fn is_paused(env: &Env) -> bool {
        env.storage()
            .instance()
            .get::<StorageKey, bool>(&StorageKey::Paused)
            .unwrap_or(false)
    }

    /// Set the paused state of the contract.
    pub fn set_paused(env: &Env, paused: bool) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::Paused, &paused);
        env.storage().instance().extend_ttl(ttl, ttl);
    }
}
