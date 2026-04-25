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
    AdminCouncil, Attestation, AttestationRequest, AuditEntry, ClaimTypeInfo, Delegation,
    Endorsement, Error, ExpirationHook, FeeConfig, GlobalStats, IssuerMetadata, IssuerStats,
    IssuerTier, MultiSigProposal, RateLimitConfig, StorageLimits, TtlConfig,
};
use soroban_sdk::{contracttype, Address, Env, String, Vec};

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
    /// Whether whitelist mode is enabled for an issuer.
    IssuerWhitelistMode(Address),
    /// Whether a subject is whitelisted for a specific issuer.
    IssuerWhitelist(Address, Address),
    /// Configurable storage exhaustion limits.
    Limits,
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
            .get(&StorageKey::AdminCouncil)
            .ok_or(Error::NotInitialized)
    }

    /// Persist the admin council and refresh TTL.
    pub fn set_admin_council(env: &Env, council: &AdminCouncil) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::AdminCouncil, council);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    /// Return true if `address` is an admin in the council.
    pub fn is_admin(env: &Env, address: &Address) -> bool {
        if let Ok(council) = Self::get_admin_council(env) {
            for admin in council.iter() {
                if admin == *address {
                    return true;
                }
            }
        }
        false
    }

    /// Add `admin` to council if not already present.
    pub fn add_admin(env: &Env, admin: &Address) {
        let mut council = Self::get_admin_council(env).unwrap_or(Vec::new(env));
        let mut found = false;
        for a in council.iter() {
            if a == *admin {
                found = true;
                break;
            }
        }
        if !found {
            council.push_back(admin.clone());
            Self::set_admin_council(env, &council);
        }
    }

    /// Remove `admin` from council if present.
    pub fn remove_admin(env: &Env, admin: &Address) {
        let mut council = Self::get_admin_council(env).unwrap_or(Vec::new(env));
        let mut new_council = Vec::new(env);
        let mut found = false;
        for a in council.iter() {
            if a != *admin {
                new_council.push_back(a.clone());
            } else {
                found = true;
            }
        }
        if found {
            Self::set_admin_council(env, &new_council);
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
        council.first().ok_or(Error::NotInitialized)
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

    /// Return the ordered list of attestation IDs created by `issuer`, or an
    /// empty [`Vec`] if none exist.
    pub fn get_issuer_attestations(env: &Env, issuer: &Address) -> Vec<String> {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env))
    }

    /// Remove `attestation_id` from `issuer`'s attestation index.
    ///
    /// Note: this does not delete the attestation record; it only removes the ID
    /// from the issuer's listing index so pagination results shrink.
    pub fn add_issuer_attestation(env: &Env, issuer: &Address, attestation_id: &String) {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut attestations = Self::get_issuer_attestations(env, issuer);
        attestations.push_back(attestation_id.clone());
        env.storage().persistent().set(&key, &attestations);
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

    /// Enable or disable whitelist mode for an issuer.
    pub fn set_whitelist_mode(env: &Env, issuer: &Address, enabled: bool) {
        let key = StorageKey::IssuerWhitelistMode(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &enabled);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve the admin council, or `None` if not initialized.
    pub fn get_council(env: &Env) -> Option<AdminCouncil> {
        env.storage().instance().get(&StorageKey::AdminCouncil)
    }

    /// Enable or disable whitelist mode (alias used by lib.rs).
    pub fn set_whitelist_enabled(env: &Env, issuer: &Address, enabled: bool) {
        Self::set_whitelist_mode(env, issuer, enabled);
    }

    /// Return `true` if whitelist mode is enabled (alias used by lib.rs).
    pub fn is_whitelist_enabled(env: &Env, issuer: &Address) -> bool {
        Self::is_whitelist_mode(env, issuer)
    }

    /// Add `subject` to `issuer`'s whitelist.
    pub fn add_to_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        let key = StorageKey::IssuerWhitelist(issuer.clone(), subject.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve a council proposal by ID.
    pub fn get_proposal(env: &Env, id: u32) -> Option<CouncilProposal> {
        env.storage().persistent().get(&StorageKey::CouncilProposal(id))
    }

    /// Increment and return the next proposal ID.
    pub fn next_proposal_id(env: &Env) -> u32 {
        let current: u32 = env.storage().instance().get(&StorageKey::ProposalCounter).unwrap_or(0);
        let next = current + 1;
        env.storage().instance().set(&StorageKey::ProposalCounter, &next);
        next
    }

    /// Set the contract paused flag.
    pub fn set_paused(env: &Env, paused: bool) {
        env.storage().instance().set(&StorageKey::Paused, &paused);
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }

    /// Return `true` if the contract is paused.
    pub fn is_paused(env: &Env) -> bool {
        env.storage().instance().get(&StorageKey::Paused).unwrap_or(false)
    }

    // ── Whitelist aliases used by lib.rs ──────────────────────────────────────

    pub fn set_whitelist_enabled(env: &Env, issuer: &Address, enabled: bool) {
        Self::set_whitelist_mode(env, issuer, enabled);
    }

    pub fn is_whitelist_enabled(env: &Env, issuer: &Address) -> bool {
        Self::is_whitelist_mode(env, issuer)
    }

    pub fn add_subject_to_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        Self::add_to_whitelist(env, issuer, subject);
    }

    pub fn remove_subject_from_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        Self::remove_from_whitelist(env, issuer, subject);
    }

    pub fn is_subject_whitelisted(env: &Env, issuer: &Address, subject: &Address) -> bool {
        Self::is_whitelisted(env, issuer, subject)
    }

    // ── Global stats ──────────────────────────────────────────────────────────

    pub fn get_global_stats(env: &Env) -> GlobalStats {
        env.storage()
            .instance()
            .get(&StorageKey::GlobalStats)
            .unwrap_or(GlobalStats {
                total_attestations: 0,
                total_revocations: 0,
                total_issuers: 0,
            })
    }

    fn set_global_stats(env: &Env, stats: &GlobalStats) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::GlobalStats, stats);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    pub fn increment_total_attestations(env: &Env, count: u64) {
        let mut stats = Self::get_global_stats(env);
        stats.total_attestations = stats.total_attestations.saturating_add(count);
        Self::set_global_stats(env, &stats);
    }

    pub fn increment_total_revocations(env: &Env, count: u64) {
        let mut stats = Self::get_global_stats(env);
        stats.total_revocations = stats.total_revocations.saturating_add(count);
        Self::set_global_stats(env, &stats);
    }

    pub fn increment_total_issuers(env: &Env) {
        let mut stats = Self::get_global_stats(env);
        stats.total_issuers = stats.total_issuers.saturating_add(1);
        Self::set_global_stats(env, &stats);
    }

    pub fn decrement_total_issuers(env: &Env) {
        let mut stats = Self::get_global_stats(env);
        stats.total_issuers = stats.total_issuers.saturating_sub(1);
        Self::set_global_stats(env, &stats);
    }

    // ── Per-issuer stats ──────────────────────────────────────────────────────

    pub fn get_issuer_stats(env: &Env, issuer: &Address) -> IssuerStats {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerStats(issuer.clone()))
            .unwrap_or(IssuerStats { total_issued: 0 })
    }

    pub fn set_issuer_stats(env: &Env, issuer: &Address, stats: &IssuerStats) {
        let key = StorageKey::IssuerStats(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, stats);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    // ── Issuer tier ───────────────────────────────────────────────────────────

    pub fn get_issuer_tier(env: &Env, issuer: &Address) -> Option<IssuerTier> {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerTier(issuer.clone()))
    }

    pub fn set_issuer_tier(env: &Env, issuer: &Address, tier: &IssuerTier) {
        let key = StorageKey::IssuerTier(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, tier);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    // ── Paused flag ───────────────────────────────────────────────────────────

    pub fn is_paused(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&StorageKey::Paused)
            .unwrap_or(false)
    }

    pub fn set_paused(env: &Env, paused: bool) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::Paused, &paused);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    // ── Storage limits ────────────────────────────────────────────────────────

    pub fn get_limits(env: &Env) -> StorageLimits {
        env.storage()
            .instance()
            .get(&StorageKey::StorageLimits)
            .unwrap_or_default()
    }

    pub fn set_limits(env: &Env, limits: &StorageLimits) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::StorageLimits, limits);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    // ── Audit log ─────────────────────────────────────────────────────────────

    pub fn get_audit_log(env: &Env, attestation_id: &String) -> Vec<AuditEntry> {
        env.storage()
            .persistent()
            .get(&StorageKey::AuditLog(attestation_id.clone()))
            .unwrap_or(Vec::new(env))
    }

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
            .persistent()
            .get(&StorageKey::Endorsements(attestation_id.clone()))
            .unwrap_or(Vec::new(env))
    }

    pub fn add_endorsement(env: &Env, endorsement: &Endorsement) {
        let key = StorageKey::Endorsements(endorsement.attestation_id.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list = Self::get_endorsements(env, &endorsement.attestation_id);
        list.push_back(endorsement.clone());
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    // ── Expiration hooks ──────────────────────────────────────────────────────

    pub fn get_expiration_hook(env: &Env, subject: &Address) -> Option<ExpirationHook> {
        env.storage()
            .persistent()
            .get(&StorageKey::ExpirationHook(subject.clone()))
    }

    pub fn set_expiration_hook(env: &Env, subject: &Address, hook: &ExpirationHook) {
        let key = StorageKey::ExpirationHook(subject.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, hook);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn remove_expiration_hook(env: &Env, subject: &Address) {
        env.storage()
            .persistent()
            .remove(&StorageKey::ExpirationHook(subject.clone()));
    }

    // ── Multi-sig proposals ───────────────────────────────────────────────────

    pub fn get_multisig_proposal(env: &Env, proposal_id: &String) -> Result<MultiSigProposal, Error> {
        env.storage()
            .persistent()
            .get(&StorageKey::MultiSigProposal(proposal_id.clone()))
            .ok_or(Error::NotFound)
    }

    pub fn set_multisig_proposal(env: &Env, proposal: &MultiSigProposal) {
        let key = StorageKey::MultiSigProposal(proposal.id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, proposal);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    // ── Attestation requests ──────────────────────────────────────────────────

    pub fn get_attestation_request(env: &Env, request_id: &String) -> Result<AttestationRequest, Error> {
        env.storage()
            .persistent()
            .get(&StorageKey::AttestationRequest(request_id.clone()))
            .ok_or(Error::NotFound)
    }

    pub fn set_attestation_request(env: &Env, request: &AttestationRequest) {
        let key = StorageKey::AttestationRequest(request.id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, request);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_pending_requests(env: &Env, issuer: &Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&StorageKey::PendingRequests(issuer.clone()))
            .unwrap_or(Vec::new(env))
    }

    pub fn add_pending_request(env: &Env, issuer: &Address, request_id: &String) {
        let key = StorageKey::PendingRequests(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list = Self::get_pending_requests(env, issuer);
        list.push_back(request_id.clone());
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn remove_pending_request(env: &Env, issuer: &Address, request_id: &String) {
        let key = StorageKey::PendingRequests(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let existing = Self::get_pending_requests(env, issuer);
        let mut updated = Vec::new(env);
        for id in existing.iter() {
            if &id != request_id {
                updated.push_back(id);
            }
        }
        env.storage().persistent().set(&key, &updated);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    // ── Rate limiting ─────────────────────────────────────────────────────────

    pub fn get_rate_limit_config(env: &Env) -> Option<RateLimitConfig> {
        env.storage()
            .instance()
            .get(&StorageKey::RateLimitConfig)
    }

    pub fn set_rate_limit_config(env: &Env, config: &RateLimitConfig) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::RateLimitConfig, config);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    pub fn get_last_issuance_time(env: &Env, issuer: &Address) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&StorageKey::LastIssuanceTime(issuer.clone()))
    }

    pub fn set_last_issuance_time(env: &Env, issuer: &Address, timestamp: u64) {
        let key = StorageKey::LastIssuanceTime(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &timestamp);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    // ── Delegation ────────────────────────────────────────────────────────────

    pub fn get_delegation(
        env: &Env,
        delegator: &Address,
        delegate: &Address,
        claim_type: &String,
    ) -> Option<Delegation> {
        env.storage()
            .persistent()
            .get(&StorageKey::Delegation(delegator.clone(), delegate.clone(), claim_type.clone()))
    }

    pub fn set_delegation(env: &Env, delegation: &Delegation) {
        let key = StorageKey::Delegation(
            delegation.delegator.clone(),
            delegation.delegate.clone(),
            delegation.claim_type.clone(),
        );
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, delegation);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn remove_delegation(
        env: &Env,
        delegator: &Address,
        delegate: &Address,
        claim_type: &String,
    ) {
        env.storage()
            .persistent()
            .remove(&StorageKey::Delegation(delegator.clone(), delegate.clone(), claim_type.clone()));
    }
}

/// Generic pagination helper: returns a slice of `items` starting at `start`
/// with at most `limit` elements. Returns an empty Vec when `start >= items.len()`
/// or `limit == 0`.
pub fn paginate<T>(env: &Env, items: &Vec<T>, start: u32, limit: u32) -> Vec<T>
where
    T: soroban_sdk::TryFromVal<Env, soroban_sdk::Val>
        + soroban_sdk::IntoVal<Env, soroban_sdk::Val>
        + Clone,
{
    let mut result = Vec::new(env);
    if limit == 0 {
        return result;
    }
    let len = items.len();
    if start >= len {
        return result;
    }
    let end = len.min(start.saturating_add(limit));
    for i in start..end {
        if let Some(item) = items.get(i) {
            result.push_back(item);
        }
    }

    /// Persist storage exhaustion limits.
    pub fn set_limits(env: &Env, limits: &StorageLimits) {
        env.storage().instance().set(&StorageKey::Limits, limits);
    }

    /// Return the current storage limits, falling back to [`StorageLimits::default`] if not set.
    pub fn get_limits(env: &Env) -> StorageLimits {
        env.storage()
            .instance()
            .get(&StorageKey::Limits)
            .unwrap_or_default()
    }
}
