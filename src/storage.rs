//! Storage helpers for TrustLink.
//!
//! Single point of contact between contract logic and on-chain storage.

use crate::constants::{DAY_IN_LEDGERS, DEFAULT_INSTANCE_LIFETIME};
use crate::types::{
    AdminCouncil, Attestation, AttestationRequest, AttestationTemplate, AuditEntry, ClaimTypeInfo,
    CouncilProposal, Delegation, Endorsement, Error, ExpirationHook, FeeConfig, GlobalStats,
    IssuerMetadata, IssuerStats, IssuerTier, MultiSigProposal, PendingAdminTransfer,
    RateLimitConfig, StorageLimits, TtlConfig,
};
use soroban_sdk::{contracttype, Address, Env, String, Vec};

#[contracttype]
pub enum StorageKey {
    Admin,
    AdminCouncil,
    Version,
    FeeConfig,
    TtlConfig,
    Issuer(Address),
    Bridge(Address),
    Attestation(String),
    SubjectAttestations(Address),
    IssuerAttestations(Address),
    IssuerMetadata(Address),
    ClaimType(String),
    ClaimTypeList,
    MultisigTtlDays,
    IssuerTier(Address),
    IssuerStats(Address),
    GlobalStats,
    ExpirationHook(Address),
    Endorsements(String),
    Limits,
    StorageLimits,
    RateLimitConfig,
    LastIssuance(Address),
    LastIssuanceTime(Address),
    IssuerWhitelistEnabled(Address),
    /// Whitelist mode flag (alias for IssuerWhitelistEnabled).
    IssuerWhitelistMode(Address),
    /// Whitelist entry for a (issuer, subject) pair.
    IssuerWhitelist(Address, Address),
    /// Audit log entries for an attestation.
    AuditLog(String),
    /// Multi-sig proposal keyed by proposal ID.
    MultiSigProposal(String),
    /// An attestation request record.
    AttestationRequest(String),
    IssuerPendingRequests(Address),
    PendingRequests(Address),
    /// Last issuance timestamp for rate limiting (alias).
    LastIssuanceTime(Address),
    /// Storage limits.
    StorageLimits,
    /// Contract paused flag.
    Paused,
    /// Council proposal by numeric ID.
    CouncilProposal(u32),
    CouncilProposalStr(String),
    ProposalCounter,
    PendingAdminTransfer,
    AuditLog(String),
    MultiSigProposal(String),
    AttestationTemplate(Address, String),
    AttestationTemplateList(Address),
}

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

pub struct Storage;

impl Storage {
    pub fn has_admin(env: &Env) -> bool {
        if let Ok(council) = Self::get_admin_council(env) {
            !council.is_empty()
        } else {
            false
        }
    }

    pub fn set_admin(env: &Env, admin: &Address) {
        let mut council = Vec::new(env);
        council.push_back(admin.clone());
        Self::set_admin_council(env, &council);
    }

    pub fn get_admin_council(env: &Env) -> Result<AdminCouncil, Error> {
        env.storage()
            .instance()
            .get(&StorageKey::AdminCouncil)
            .ok_or(Error::NotInitialized)
    }

    pub fn set_admin_council(env: &Env, council: &AdminCouncil) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::AdminCouncil, council);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    pub fn is_admin(env: &Env, address: &Address) -> bool {
        if let Ok(council) = Self::get_admin_council(env) {
            for admin in council.iter() {
                if &admin == address { return true; }
            }
        }
        false
    }

    pub fn add_admin(env: &Env, admin: &Address) {
        let mut council = Self::get_admin_council(env).unwrap_or(Vec::new(env));
        for a in council.iter() {
            if &a == admin { return; }
        }
        council.push_back(admin.clone());
        Self::set_admin_council(env, &council);
    }

    pub fn remove_admin(env: &Env, admin: &Address) {
        let council = Self::get_admin_council(env).unwrap_or(Vec::new(env));
        let mut new_council = Vec::new(env);
        for a in council.iter() {
            if &a != admin { new_council.push_back(a); }
        }
        Self::set_admin_council(env, &new_council);
    }

    pub fn get_admin(env: &Env) -> Result<Address, Error> {
        let council = Self::get_admin_council(env)?;
        council.first().ok_or(Error::NotInitialized)
    }

    pub fn get_council(env: &Env) -> Option<AdminCouncil> {
        env.storage().instance().get(&StorageKey::AdminCouncil)
    }

    pub fn set_council(env: &Env, council: &AdminCouncil) {
        Self::set_admin_council(env, council);
    }

    pub fn set_version(env: &Env, version: &String) {
        env.storage().instance().set(&StorageKey::Version, version);
    }

    pub fn get_version(env: &Env) -> Option<String> {
        env.storage().instance().get(&StorageKey::Version)
    }

    pub fn set_fee_config(env: &Env, fee_config: &FeeConfig) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::FeeConfig, fee_config);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    pub fn get_fee_config(env: &Env) -> Option<FeeConfig> {
        env.storage().instance().get(&StorageKey::FeeConfig)
    }

    pub fn set_ttl_config(env: &Env, ttl_config: &TtlConfig) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::TtlConfig, ttl_config);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    pub fn get_ttl_config(env: &Env) -> Option<TtlConfig> {
        env.storage().instance().get(&StorageKey::TtlConfig)
    }

    pub fn is_issuer(env: &Env, address: &Address) -> bool {
        env.storage().persistent().has(&StorageKey::Issuer(address.clone()))
    }

    pub fn add_issuer(env: &Env, issuer: &Address) {
        let key = StorageKey::Issuer(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn remove_issuer(env: &Env, issuer: &Address) {
        env.storage().persistent().remove(&StorageKey::Issuer(issuer.clone()));
    }

    pub fn is_bridge(env: &Env, address: &Address) -> bool {
        env.storage().persistent().has(&StorageKey::Bridge(address.clone()))
    }

    pub fn add_bridge(env: &Env, bridge: &Address) {
        let key = StorageKey::Bridge(bridge.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn has_attestation(env: &Env, id: &String) -> bool {
        env.storage().persistent().has(&StorageKey::Attestation(id.clone()))
    }

    pub fn set_attestation(env: &Env, attestation: &Attestation) {
        let key = StorageKey::Attestation(attestation.id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, attestation);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_attestation(env: &Env, id: &String) -> Result<Attestation, Error> {
        env.storage().persistent().get(&StorageKey::Attestation(id.clone())).ok_or(Error::NotFound)
    }

    pub fn get_subject_attestations(env: &Env, subject: &Address) -> Vec<String> {
        env.storage().persistent().get(&StorageKey::SubjectAttestations(subject.clone())).unwrap_or(Vec::new(env))
    }

    pub fn add_subject_attestation(env: &Env, subject: &Address, attestation_id: &String) {
        let key = StorageKey::SubjectAttestations(subject.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list = Self::get_subject_attestations(env, subject);
        list.push_back(attestation_id.clone());
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn remove_subject_attestation(env: &Env, subject: &Address, attestation_id: &String) {
        let key = StorageKey::SubjectAttestations(subject.clone());
        let ttl = get_ttl_lifetime(env);
        let existing = Self::get_subject_attestations(env, subject);
        let mut updated = Vec::new(env);
        for id in existing.iter() {
            if &id != attestation_id { updated.push_back(id); }
        }
        env.storage().persistent().set(&key, &updated);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_issuer_attestations(env: &Env, issuer: &Address) -> Vec<String> {
        env.storage().persistent().get(&StorageKey::IssuerAttestations(issuer.clone())).unwrap_or(Vec::new(env))
    }

    pub fn add_issuer_attestation(env: &Env, issuer: &Address, attestation_id: &String) {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list = Self::get_issuer_attestations(env, issuer);
        list.push_back(attestation_id.clone());
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Remove an attestation ID from the issuer's attestation index.
    ///
    /// Used when transferring attestation ownership to a new issuer.
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

    /// Remove an attestation ID from the issuer's attestation index.
    ///
    /// Used when transferring attestation ownership to a new issuer.
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

    pub fn get_issuer_metadata(env: &Env, issuer: &Address) -> Option<IssuerMetadata> {
        env.storage().persistent().get(&StorageKey::IssuerMetadata(issuer.clone()))
    }

    pub fn set_claim_type(env: &Env, info: &ClaimTypeInfo) {
        let key = StorageKey::ClaimType(info.claim_type.clone());
        let is_new = !env.storage().persistent().has(&key);
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, info);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
        if is_new {
            let list_key = StorageKey::ClaimTypeList;
            let mut list: Vec<String> = env.storage().persistent().get(&list_key).unwrap_or(Vec::new(env));
            list.push_back(info.claim_type.clone());
            env.storage().persistent().set(&list_key, &list);
            env.storage().persistent().extend_ttl(&list_key, ttl, ttl);
        }
    }

    pub fn get_claim_type(env: &Env, claim_type: &String) -> Option<ClaimTypeInfo> {
        env.storage().persistent().get(&StorageKey::ClaimType(claim_type.clone()))
    }

    pub fn get_claim_type_list(env: &Env) -> Vec<String> {
        env.storage().persistent().get(&StorageKey::ClaimTypeList).unwrap_or(Vec::new(env))
    }

    pub fn set_whitelist_mode(env: &Env, issuer: &Address, enabled: bool) {
        let key = StorageKey::IssuerWhitelistMode(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &enabled);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn is_whitelist_mode(env: &Env, issuer: &Address) -> bool {
        env.storage().persistent().get(&StorageKey::IssuerWhitelistMode(issuer.clone())).unwrap_or(false)
    }

    pub fn set_whitelist_enabled(env: &Env, issuer: &Address, enabled: bool) {
        Self::set_whitelist_mode(env, issuer, enabled);
    }

    pub fn is_whitelist_enabled(env: &Env, issuer: &Address) -> bool {
        Self::is_whitelist_mode(env, issuer)
    }

    pub fn is_whitelisted(env: &Env, issuer: &Address, subject: &Address) -> bool {
        env.storage().persistent().has(&StorageKey::IssuerWhitelist(issuer.clone(), subject.clone()))
    }

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

    /// Persist a council proposal.
    pub fn set_proposal(env: &Env, proposal: &CouncilProposal) {
        let key = StorageKey::CouncilProposal(proposal.id);
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, proposal);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn remove_from_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        env.storage().persistent().remove(&StorageKey::IssuerWhitelist(issuer.clone(), subject.clone()));
    }

    pub fn is_subject_whitelisted(env: &Env, issuer: &Address, subject: &Address) -> bool {
        Self::is_whitelisted(env, issuer, subject)
    }

    pub fn add_subject_to_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        Self::add_to_whitelist(env, issuer, subject);
    }

    pub fn remove_subject_from_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        Self::remove_from_whitelist(env, issuer, subject);
    }

    pub fn set_paused(env: &Env, paused: bool) {
        env.storage().instance().set(&StorageKey::Paused, &paused);
        env.storage().instance().extend_ttl(DEFAULT_INSTANCE_LIFETIME, DEFAULT_INSTANCE_LIFETIME);
    }

    pub fn is_paused(env: &Env) -> bool {
        env.storage().instance().get(&StorageKey::Paused).unwrap_or(false)
    }

    fn set_global_stats_raw(env: &Env, stats: &GlobalStats) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::GlobalStats, stats);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    /// Increment `total_attestations` by `count`.
    pub fn increment_total_attestations(env: &Env, count: u64) {
        let mut stats = Self::get_global_stats(env);
        stats.total_attestations = stats.total_attestations.saturating_add(count);
        Self::set_global_stats(env, &stats);
    }

    pub fn increment_total_revocations(env: &Env, by: u64) {
        let mut s = Self::get_global_stats_raw(env);
        s.total_revocations = s.total_revocations.saturating_add(by);
        Self::set_global_stats_raw(env, &s);
    }

    pub fn increment_total_issuers(env: &Env) {
        let mut s = Self::get_global_stats_raw(env);
        s.total_issuers = s.total_issuers.saturating_add(1);
        Self::set_global_stats_raw(env, &s);
    }

    pub fn decrement_total_issuers(env: &Env) {
        let mut s = Self::get_global_stats_raw(env);
        s.total_issuers = s.total_issuers.saturating_sub(1);
        Self::set_global_stats_raw(env, &s);
    }

    pub fn get_issuer_stats(env: &Env, issuer: &Address) -> IssuerStats {
        env.storage().persistent().get(&StorageKey::IssuerStats(issuer.clone()))
            .unwrap_or(IssuerStats { total_issued: 0 })
    }

    pub fn set_issuer_stats(env: &Env, issuer: &Address, stats: &IssuerStats) {
        let key = StorageKey::IssuerStats(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, stats);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn set_issuer_tier(env: &Env, issuer: &Address, tier: &IssuerTier) {
        let key = StorageKey::IssuerTier(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, tier);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_issuer_tier(env: &Env, issuer: &Address) -> Option<IssuerTier> {
        env.storage().persistent().get(&StorageKey::IssuerTier(issuer.clone()))
    }

    pub fn get_limits(env: &Env) -> StorageLimits {
        env.storage().instance().get(&StorageKey::StorageLimits).unwrap_or_default()
    }

    pub fn set_limits(env: &Env, limits: &StorageLimits) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::StorageLimits, limits);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    pub fn get_rate_limit_config(env: &Env) -> Option<RateLimitConfig> {
        env.storage().instance().get(&StorageKey::RateLimitConfig)
    }

    pub fn set_rate_limit_config(env: &Env, config: &RateLimitConfig) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::RateLimitConfig, config);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    pub fn get_last_issuance_time(env: &Env, issuer: &Address) -> Option<u64> {
        env.storage().persistent().get(&StorageKey::LastIssuanceTime(issuer.clone()))
    }

    pub fn set_last_issuance_time(env: &Env, issuer: &Address, timestamp: u64) {
        let key = StorageKey::LastIssuanceTime(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &timestamp);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_audit_log(env: &Env, attestation_id: &String) -> Vec<AuditEntry> {
        env.storage().persistent().get(&StorageKey::AuditLog(attestation_id.clone())).unwrap_or(Vec::new(env))
    }

    pub fn append_audit_entry(env: &Env, attestation_id: &String, entry: &AuditEntry) {
        let key = StorageKey::AuditLog(attestation_id.clone());
        let ttl = get_ttl_lifetime(env);
        let mut log = Self::get_audit_log(env, attestation_id);
        log.push_back(entry.clone());
        env.storage().persistent().set(&key, &log);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_expiration_hook(env: &Env, subject: &Address) -> Option<ExpirationHook> {
        env.storage().persistent().get(&StorageKey::ExpirationHook(subject.clone()))
    }

    pub fn set_expiration_hook(env: &Env, subject: &Address, hook: &ExpirationHook) {
        let key = StorageKey::ExpirationHook(subject.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, hook);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn remove_expiration_hook(env: &Env, subject: &Address) {
        env.storage().persistent().remove(&StorageKey::ExpirationHook(subject.clone()));
    }

    pub fn get_multisig_proposal(env: &Env, proposal_id: &String) -> Result<MultiSigProposal, Error> {
        env.storage().persistent().get(&StorageKey::MultiSigProposal(proposal_id.clone())).ok_or(Error::NotFound)
    }

    pub fn set_multisig_proposal(env: &Env, proposal: &MultiSigProposal) {
        let key = StorageKey::MultiSigProposal(proposal.id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, proposal);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_multisig_ttl_days(env: &Env) -> u32 {
        env.storage().instance().get(&StorageKey::MultisigTtlDays).unwrap_or(7)
    }

    pub fn get_endorsements(env: &Env, attestation_id: &String) -> Vec<Endorsement> {
        env.storage().persistent().get(&StorageKey::Endorsements(attestation_id.clone())).unwrap_or(Vec::new(env))
    }

    pub fn add_endorsement(env: &Env, attestation_id: &String, endorsement: &Endorsement) {
        let key = StorageKey::Endorsements(attestation_id.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list = Self::get_endorsements(env, attestation_id);
        list.push_back(endorsement.clone());
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_proposal(env: &Env, id: u32) -> Option<CouncilProposal> {
        env.storage().persistent().get(&StorageKey::CouncilProposal(id))
    }

    pub fn set_proposal(env: &Env, proposal: &CouncilProposal) {
        let key = StorageKey::CouncilProposal(proposal.id);
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, proposal);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn next_proposal_id(env: &Env) -> u32 {
        let current: u32 = env.storage().instance().get(&StorageKey::ProposalCounter).unwrap_or(0);
        let next = current + 1;
        env.storage().instance().set(&StorageKey::ProposalCounter, &next);
        next
    }

    pub fn set_last_issuance_time(env: &Env, issuer: &Address, timestamp: u64) {
        let key = StorageKey::LastIssuance(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, &timestamp);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    // -------------------------------------------------------------------------
    // Attestation requests
    // -------------------------------------------------------------------------

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

    pub fn get_issuer_pending_requests(env: &Env, issuer: &Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerPendingRequests(issuer.clone()))
            .unwrap_or(Vec::new(env))
    }

    pub fn add_issuer_pending_request(env: &Env, issuer: &Address, request_id: &String) {
        let key = StorageKey::IssuerPendingRequests(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list = Self::get_issuer_pending_requests(env, issuer);
        list.push_back(request_id.clone());
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn remove_issuer_pending_request(env: &Env, issuer: &Address, request_id: &String) {
        let key = StorageKey::IssuerPendingRequests(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let existing = Self::get_issuer_pending_requests(env, issuer);
        let mut updated = Vec::new(env);
        for id in existing.iter() {
            if &id != request_id {
                updated.push_back(id);
            }
        }
        env.storage().persistent().set(&key, &updated);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    // ── Issuer tier ───────────────────────────────────────────────────────────

    pub fn get_issuer_tier(env: &Env, issuer: &Address) -> Option<IssuerTier> {
        env.storage().persistent().get(&StorageKey::IssuerTier(issuer.clone()))
    }

    // ── Issuer attestation index ──────────────────────────────────────────────

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

    // ── Delegation ────────────────────────────────────────────────────────────

    pub fn set_delegation(env: &Env, delegation: &crate::types::Delegation) {
        let key = StorageKey::Delegation(
            delegation.delegator.clone(),
            delegation.delegate.clone(),
            delegation.claim_type.clone(),
        );
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, delegation);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    // ── Multisig TTL ──────────────────────────────────────────────────────────

    pub fn get_multisig_ttl_days(env: &Env) -> u32 {
        env.storage()
            .instance()
            .get(&StorageKey::MultisigTtlDays)
            .unwrap_or(7u32)
    }

    // ── Attestation requests ──────────────────────────────────────────────────

    pub fn get_request(env: &Env, request_id: &String) -> Result<crate::types::AttestationRequest, crate::types::Error> {
        env.storage()
            .persistent()
            .get(&StorageKey::AttestationRequest(request_id.clone()))
            .ok_or(crate::types::Error::NotFound)
    }

    pub fn set_request(env: &Env, request: &crate::types::AttestationRequest) {
        let key = StorageKey::AttestationRequest(request.id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, request);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_pending_request_ids(env: &Env, issuer: &Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerPendingRequests(issuer.clone()))
            .unwrap_or(Vec::new(env))
    }

    pub fn add_pending_request(env: &Env, issuer: &Address, request_id: &String) {
        let key = StorageKey::IssuerPendingRequests(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list = Self::get_pending_request_ids(env, issuer);
        list.push_back(request_id.clone());
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn remove_pending_request(env: &Env, issuer: &Address, request_id: &String) {
        let key = StorageKey::IssuerPendingRequests(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let existing = Self::get_pending_request_ids(env, issuer);
        let mut updated = Vec::new(env);
        for id in existing.iter() {
            if &id != request_id {
                updated.push_back(id);
            }
        }
        env.storage().persistent().set(&key, &updated);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }
}

pub fn paginate(env: &Env, list: &Vec<String>, start: u32, limit: u32) -> Vec<String> {
    let mut result = Vec::new(env);
    let len = list.len();
    if start >= len {
        return result;
    }
    let end = (start + limit).min(len);
    for i in start..end {
        if let Some(item) = list.get(i) {
            result.push_back(item);
        }
    }
    result
}
