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
    StorageLimits,
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
    /// Per-issuer issuance statistics.
    IssuerStats(Address),
    /// Per-issuer trust tier.
    IssuerTier(Address),
    /// Global contract statistics.
    GlobalStats,
    /// Contract paused flag.
    Paused,
    /// Rate limit configuration.
    RateLimitConfig,
    /// Storage exhaustion limits.
    StorageLimits,
    /// Last issuance timestamp per issuer (for rate limiting).
    LastIssuanceTime(Address),
    /// Expiration notification hook per subject.
    ExpirationHook(Address),
    /// Multisig proposal by ID.
    MultisigProposal(String),
    /// Endorsements list per attestation.
    Endorsements(String),
    /// Audit log per attestation.
    AuditLog(String),
    /// Pull-based attestation request by ID.
    AttestationRequest(String),
    /// Pending request IDs per issuer.
    PendingRequests(Address),
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
                if &admin == address {
                    return true;
                }
            }
        }
        false
    }

    /// Add `admin` to council if not already present.
    pub fn add_admin(env: &Env, admin: &Address) {
        let mut council = Self::get_admin_council(env).unwrap_or_else(|_| Vec::new(env));
        let mut found = false;
        for a in council.iter() {
            if &a == admin {
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
        let mut council = Self::get_admin_council(env).unwrap_or_else(|_| Vec::new(env));
        let mut new_council: AdminCouncil = Vec::new(env);
        let mut found = false;
        for a in council.iter() {
            if &a != admin {
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

    /// Return `true` if whitelist mode is enabled for `issuer`.
    pub fn is_whitelist_mode(env: &Env, issuer: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerWhitelistMode(issuer.clone()))
            .unwrap_or(false)
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

    /// Remove `subject` from `issuer`'s whitelist.
    pub fn remove_from_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        env.storage()
            .persistent()
            .remove(&StorageKey::IssuerWhitelist(issuer.clone(), subject.clone()));
    }

    /// Return `true` if `subject` is on `issuer`'s whitelist.
    pub fn is_whitelisted(env: &Env, issuer: &Address, subject: &Address) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::IssuerWhitelist(issuer.clone(), subject.clone()))
    }

    /// Alias used by lib.rs for whitelist subject check.
    pub fn is_subject_whitelisted(env: &Env, issuer: &Address, subject: &Address) -> bool {
        Self::is_whitelisted(env, issuer, subject)
    }

    /// Add `subject` to `issuer`'s whitelist (alias).
    pub fn add_subject_to_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        Self::add_to_whitelist(env, issuer, subject);
    }

    /// Remove `subject` from `issuer`'s whitelist (alias).
    pub fn remove_subject_from_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        Self::remove_from_whitelist(env, issuer, subject);
    }

    /// Persist issuer stats.
    pub fn set_issuer_stats(env: &Env, issuer: &Address, stats: &IssuerStats) {
        let key = StorageKey::IssuerStats(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, stats);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve issuer stats, defaulting to zero if not set.
    pub fn get_issuer_stats(env: &Env, issuer: &Address) -> IssuerStats {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerStats(issuer.clone()))
            .unwrap_or(IssuerStats { total_issued: 0 })
    }

    /// Persist issuer tier.
    pub fn set_issuer_tier(env: &Env, issuer: &Address, tier: &IssuerTier) {
        let key = StorageKey::IssuerTier(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, tier);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve issuer tier, or `None` if not set.
    pub fn get_issuer_tier(env: &Env, issuer: &Address) -> Option<IssuerTier> {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerTier(issuer.clone()))
    }

    /// Persist global stats.
    pub fn set_global_stats(env: &Env, stats: &GlobalStats) {
        env.storage().instance().set(&StorageKey::GlobalStats, stats);
    }

    /// Retrieve global stats, defaulting to zeros.
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

    /// Increment total attestations counter.
    pub fn increment_total_attestations(env: &Env, count: u64) {
        let mut stats = Self::get_global_stats(env);
        stats.total_attestations += count;
        Self::set_global_stats(env, &stats);
    }

    /// Increment total revocations counter.
    pub fn increment_total_revocations(env: &Env, count: u64) {
        let mut stats = Self::get_global_stats(env);
        stats.total_revocations += count;
        Self::set_global_stats(env, &stats);
    }

    /// Increment total issuers counter.
    pub fn increment_total_issuers(env: &Env) {
        let mut stats = Self::get_global_stats(env);
        stats.total_issuers += 1;
        Self::set_global_stats(env, &stats);
    }

    /// Decrement total issuers counter.
    pub fn decrement_total_issuers(env: &Env) {
        let mut stats = Self::get_global_stats(env);
        stats.total_issuers = stats.total_issuers.saturating_sub(1);
        Self::set_global_stats(env, &stats);
    }

    /// Persist a multisig proposal.
    pub fn set_multisig_proposal(env: &Env, proposal: &MultiSigProposal) {
        let key = StorageKey::MultisigProposal(proposal.id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, proposal);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve a multisig proposal by ID.
    pub fn get_multisig_proposal(env: &Env, id: &String) -> Result<MultiSigProposal, Error> {
        env.storage()
            .persistent()
            .get(&StorageKey::MultisigProposal(id.clone()))
            .ok_or(Error::NotFound)
    }

    /// Append an endorsement to the list for an attestation.
    pub fn add_endorsement(env: &Env, endorsement: &Endorsement) {
        let key = StorageKey::Endorsements(endorsement.attestation_id.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list: Vec<Endorsement> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env));
        list.push_back(endorsement.clone());
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve all endorsements for an attestation.
    pub fn get_endorsements(env: &Env, attestation_id: &String) -> Vec<Endorsement> {
        env.storage()
            .persistent()
            .get(&StorageKey::Endorsements(attestation_id.clone()))
            .unwrap_or(Vec::new(env))
    }

    /// Append an audit entry for an attestation.
    pub fn append_audit_entry(env: &Env, attestation_id: &String, entry: &AuditEntry) {
        let key = StorageKey::AuditLog(attestation_id.clone());
        let ttl = get_ttl_lifetime(env);
        let mut log: Vec<AuditEntry> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env));
        log.push_back(entry.clone());
        env.storage().persistent().set(&key, &log);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve the full audit log for an attestation.
    pub fn get_audit_log(env: &Env, attestation_id: &String) -> Vec<AuditEntry> {
        env.storage()
            .persistent()
            .get(&StorageKey::AuditLog(attestation_id.clone()))
            .unwrap_or(Vec::new(env))
    }

    /// Persist the paused flag.
    pub fn set_paused(env: &Env, paused: bool) {
        env.storage().instance().set(&StorageKey::Paused, &paused);
    }

    /// Return `true` if the contract is paused.
    pub fn is_paused(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&StorageKey::Paused)
            .unwrap_or(false)
    }

    /// Persist rate limit config.
    pub fn set_rate_limit_config(env: &Env, config: &RateLimitConfig) {
        env.storage().instance().set(&StorageKey::RateLimitConfig, config);
    }

    /// Retrieve rate limit config.
    pub fn get_rate_limit_config(env: &Env) -> Option<RateLimitConfig> {
        env.storage().instance().get(&StorageKey::RateLimitConfig)
    }

    /// Persist storage limits.
    pub fn set_limits(env: &Env, limits: &StorageLimits) {
        env.storage().instance().set(&StorageKey::StorageLimits, limits);
    }

    /// Retrieve storage limits, returning defaults if not set.
    pub fn get_limits(env: &Env) -> StorageLimits {
        env.storage()
            .instance()
            .get(&StorageKey::StorageLimits)
            .unwrap_or_default()
    }

    /// Persist the last issuance timestamp for rate limiting.
    pub fn set_last_issuance_time(env: &Env, issuer: &Address, timestamp: u64) {
        let key = StorageKey::LastIssuanceTime(issuer.clone());
        env.storage().persistent().set(&key, &timestamp);
    }

    /// Retrieve the last issuance timestamp for an issuer.
    pub fn get_last_issuance_time(env: &Env, issuer: &Address) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&StorageKey::LastIssuanceTime(issuer.clone()))
    }

    /// Persist an expiration hook for a subject.
    pub fn set_expiration_hook(env: &Env, subject: &Address, hook: &ExpirationHook) {
        let key = StorageKey::ExpirationHook(subject.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, hook);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve the expiration hook for a subject, or `None`.
    pub fn get_expiration_hook(env: &Env, subject: &Address) -> Option<ExpirationHook> {
        env.storage()
            .persistent()
            .get(&StorageKey::ExpirationHook(subject.clone()))
    }

    /// Persist an attestation request.
    pub fn set_attestation_request(env: &Env, request: &AttestationRequest) {
        let key = StorageKey::AttestationRequest(request.id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, request);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve an attestation request by ID.
    pub fn get_attestation_request(env: &Env, id: &String) -> Result<AttestationRequest, Error> {
        env.storage()
            .persistent()
            .get(&StorageKey::AttestationRequest(id.clone()))
            .ok_or(Error::NotFound)
    }

    /// Return `true` if an attestation request with `id` exists.
    pub fn has_attestation_request(env: &Env, id: &String) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::AttestationRequest(id.clone()))
    }

    /// Append a request ID to the issuer's pending list.
    pub fn add_pending_request(env: &Env, issuer: &Address, request_id: &String) {
        let key = StorageKey::PendingRequests(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list: Vec<String> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env));
        list.push_back(request_id.clone());
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Remove a request ID from the issuer's pending list.
    pub fn remove_pending_request(env: &Env, issuer: &Address, request_id: &String) {
        let key = StorageKey::PendingRequests(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let existing: Vec<String> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env));
        let mut updated = Vec::new(env);
        for id in existing.iter() {
            if &id != request_id {
                updated.push_back(id);
            }
        }
        env.storage().persistent().set(&key, &updated);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve the pending request IDs for an issuer.
    pub fn get_pending_requests(env: &Env, issuer: &Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&StorageKey::PendingRequests(issuer.clone()))
            .unwrap_or(Vec::new(env))
    }
}

/// Paginate a `Vec<T>` by `start` offset and `limit` count.
pub fn paginate<T: soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val> + soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val> + Clone>(
    env: &Env,
    list: &Vec<T>,
    start: u32,
    limit: u32,
) -> Vec<T> {
    let total = list.len();
    if start >= total || limit == 0 {
        return Vec::new(env);
    }
    let end = (start + limit).min(total);
    let mut result = Vec::new(env);
    for i in start..end {
        if let Some(item) = list.get(i) {
            result.push_back(item);
        }
    }
    result
}
