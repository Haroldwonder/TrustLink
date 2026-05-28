//! Storage helpers for TrustLink.
//!
//! Single point of contact between contract logic and on-chain storage.

use crate::constants::{DAY_IN_LEDGERS, DEFAULT_INSTANCE_LIFETIME};
use crate::types::{
    AdminCouncil, Attestation, AttestationRequest, AttestationTemplate, AuditEntry, ClaimTypeInfo,
    Endorsement, Error, ExpirationHook, FeeConfig, GlobalStats, IssuerMetadata, IssuerStats,
    IssuerTier, MultiSigProposal, PendingAdminTransfer, RateLimitConfig, StorageLimits, TtlConfig,
    CouncilProposal,
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
    /// Contract paused flag.
    Paused,
    /// Council proposal by numeric ID.
    CouncilProposal(u32),
    CouncilProposalStr(String),
    ProposalCounter,
    PendingAdminTransfer,
    AttestationTemplate(Address, String),
    AttestationTemplateList(Address),
    Delegation(Address, Address, String),
    /// Chunked subject index: (subject, chunk_index) → Vec<String> of up to CHUNK_SIZE IDs.
    SubjectAttestationsChunk(Address, u32),
    /// Total number of IDs stored across all subject chunks.
    SubjectAttestationsCount(Address),
    /// Chunked issuer index: (issuer, chunk_index) → Vec<String> of up to CHUNK_SIZE IDs.
    IssuerAttestationsChunk(Address, u32),
    /// Total number of IDs stored across all issuer chunks.
    IssuerAttestationsCount(Address),
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
        let _ttl = get_ttl_lifetime(env);
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

    /// Append multiple attestation IDs to the issuer index in a single write.
    ///
    /// Used by `create_attestations_batch` to replace N per-item writes with
    /// one read + one write regardless of batch size.
    pub fn add_issuer_attestations_bulk(env: &Env, issuer: &Address, attestation_ids: &Vec<String>) {
        if attestation_ids.is_empty() {
            return;
        }
        let key = StorageKey::IssuerAttestations(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list = Self::get_issuer_attestations(env, issuer);
        for id in attestation_ids.iter() {
            list.push_back(id);
        }
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Increment the issuer's `total_issued` counter by `count` in a single write.
    ///
    /// Used by `create_attestations_batch` to replace N per-item stat writes.
    pub fn increment_issuer_stats(env: &Env, issuer: &Address, count: u64) {
        let mut stats = Self::get_issuer_stats(env, issuer);
        stats.total_issued = stats.total_issued.saturating_add(count);
        Self::set_issuer_stats(env, issuer, &stats);
    }

    /// Append multiple attestation IDs to the issuer index in a single write.
    ///
    /// Used by `create_attestations_batch` to replace the N per-item writes
    /// with one read + one write regardless of batch size.
    pub fn add_issuer_attestations_bulk(env: &Env, issuer: &Address, attestation_ids: &Vec<String>) {
        if attestation_ids.is_empty() {
            return;
        }
        let key = StorageKey::IssuerAttestations(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list = Self::get_issuer_attestations(env, issuer);
        for id in attestation_ids.iter() {
            list.push_back(id);
        }
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Increment the issuer's `total_issued` counter by `count` in a single write.
    ///
    /// Used by `create_attestations_batch` to replace N per-item stat writes.
    pub fn increment_issuer_stats(env: &Env, issuer: &Address, count: u64) {
        let mut stats = Self::get_issuer_stats(env, issuer);
        stats.total_issued = stats.total_issued.saturating_add(count);
        Self::set_issuer_stats(env, issuer, &stats);
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

    pub fn get_global_stats(env: &Env) -> GlobalStats {
        env.storage().instance()
            .get(&StorageKey::GlobalStats)
            .unwrap_or(GlobalStats { total_attestations: 0, total_revocations: 0, total_issuers: 0 })
    }

    pub fn set_global_stats(env: &Env, stats: &GlobalStats) {
        Self::set_global_stats_raw(env, stats)
    }

    pub fn get_global_stats_raw(env: &Env) -> GlobalStats {
        Self::get_global_stats(env)
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

    pub fn next_proposal_id(env: &Env) -> u32 {
        let current: u32 = env.storage().instance().get(&StorageKey::ProposalCounter).unwrap_or(0);
        let next = current + 1;
        env.storage().instance().set(&StorageKey::ProposalCounter, &next);
        next
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

    pub fn get_delegation(
        env: &Env,
        delegator: &Address,
        delegate: &Address,
        claim_type: &String,
    ) -> Option<crate::types::Delegation> {
        let key = StorageKey::Delegation(delegator.clone(), delegate.clone(), claim_type.clone());
        env.storage().persistent().get(&key)
    }

    pub fn remove_delegation(
        env: &Env,
        delegator: &Address,
        delegate: &Address,
        claim_type: &String,
    ) {
        let key = StorageKey::Delegation(delegator.clone(), delegate.clone(), claim_type.clone());
        env.storage().persistent().remove(&key);
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

    // ── Pending admin transfer ────────────────────────────────────────────────

    pub fn set_pending_admin_transfer(env: &Env, transfer: &PendingAdminTransfer) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::PendingAdminTransfer, transfer);
        env.storage().instance().extend_ttl(ttl, ttl);
    }

    pub fn get_pending_admin_transfer(env: &Env) -> Option<PendingAdminTransfer> {
        env.storage().instance().get(&StorageKey::PendingAdminTransfer)
    }

    pub fn remove_pending_admin_transfer(env: &Env) {
        env.storage().instance().remove(&StorageKey::PendingAdminTransfer);
    }

    // ── Attestation templates ─────────────────────────────────────────────────

    pub fn set_template(env: &Env, issuer: &Address, template_id: &String, template: &AttestationTemplate) {
        let key = StorageKey::AttestationTemplate(issuer.clone(), template_id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, template);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_template(env: &Env, issuer: &Address, template_id: &String) -> Option<AttestationTemplate> {
        env.storage().persistent().get(&StorageKey::AttestationTemplate(issuer.clone(), template_id.clone()))
    }

    pub fn add_to_template_registry(env: &Env, issuer: &Address, template_id: &String) {
        let key = StorageKey::AttestationTemplateList(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut list: Vec<String> = env.storage().persistent().get(&key).unwrap_or(Vec::new(env));
        list.push_back(template_id.clone());
        env.storage().persistent().set(&key, &list);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    pub fn get_template_registry(env: &Env, issuer: &Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&StorageKey::AttestationTemplateList(issuer.clone()))
            .unwrap_or(Vec::new(env))
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

// =============================================================================
// Chunked index storage
//
// Soroban's storage model does not support partial/lazy reads of a single
// ledger entry — every `get` deserialises the entire value. Splitting a large
// Vec<String> index across multiple fixed-size ledger entries (chunks) means
// that a paginated query only needs to load the specific chunk(s) that overlap
// the requested page, rather than the entire index.
//
// Layout
// ------
//   SubjectAttestationsChunk(addr, 0)  → Vec<String>  (IDs 0..CHUNK_SIZE-1)
//   SubjectAttestationsChunk(addr, 1)  → Vec<String>  (IDs CHUNK_SIZE..2*CHUNK_SIZE-1)
//   SubjectAttestationsCount(addr)     → u32          (total IDs across all chunks)
//
//   IssuerAttestationsChunk(addr, 0)   → Vec<String>
//   IssuerAttestationsChunk(addr, 1)   → Vec<String>
//   IssuerAttestationsCount(addr)      → u32
//
// Chunk size is 50 IDs. Each ID is a 64-byte hex string (~64 bytes XDR).
// One chunk ≈ 50 × 64 = 3,200 bytes — well within the 64 KB entry limit.
//
// Query cost
// ----------
// To serve page (start, limit) the caller loads at most
//   ceil(limit / CHUNK_SIZE) + 1  chunks
// instead of the entire index. For a 10-item page against a 10,000-item index
// this reduces the data read from ~640 KB to ~6.4 KB (~100× improvement).
// =============================================================================

/// Number of attestation IDs stored per index chunk.
pub const CHUNK_SIZE: u32 = 50;

pub struct ChunkedIndex;

impl ChunkedIndex {
    // ── Subject index ─────────────────────────────────────────────────────────

    fn subject_count_key(subject: &Address) -> StorageKey {
        StorageKey::SubjectAttestationsCount(subject.clone())
    }

    fn subject_chunk_key(subject: &Address, chunk: u32) -> StorageKey {
        StorageKey::SubjectAttestationsChunk(subject.clone(), chunk)
    }

    /// Total number of IDs in the subject's chunked index.
    pub fn subject_count(env: &Env, subject: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&Self::subject_count_key(subject))
            .unwrap_or(0u32)
    }

    /// Append one ID to the subject's chunked index.
    pub fn add_subject(env: &Env, subject: &Address, id: &String) {
        let ttl = get_ttl_lifetime(env);
        let count = Self::subject_count(env, subject);
        let chunk_idx = count / CHUNK_SIZE;
        let chunk_key = Self::subject_chunk_key(subject, chunk_idx);

        let mut chunk: Vec<String> = env
            .storage()
            .persistent()
            .get(&chunk_key)
            .unwrap_or(Vec::new(env));
        chunk.push_back(id.clone());
        env.storage().persistent().set(&chunk_key, &chunk);
        env.storage().persistent().extend_ttl(&chunk_key, ttl, ttl);

        let new_count = count + 1;
        let count_key = Self::subject_count_key(subject);
        env.storage().persistent().set(&count_key, &new_count);
        env.storage().persistent().extend_ttl(&count_key, ttl, ttl);
    }

    /// Remove one ID from the subject's chunked index.
    ///
    /// Finds the ID by scanning from the last chunk backwards, swaps it with
    /// the last element (to avoid shifting), and decrements the count.
    pub fn remove_subject(env: &Env, subject: &Address, id: &String) {
        let ttl = get_ttl_lifetime(env);
        let count = Self::subject_count(env, subject);
        if count == 0 {
            return;
        }

        // Scan chunks from last to first to find the target ID.
        let total_chunks = (count + CHUNK_SIZE - 1) / CHUNK_SIZE;
        let mut found_chunk_idx: Option<u32> = None;
        let mut found_pos: Option<u32> = None;

        'outer: for ci in (0..total_chunks).rev() {
            let chunk_key = Self::subject_chunk_key(subject, ci);
            let chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&chunk_key)
                .unwrap_or(Vec::new(env));
            for (pos, entry) in chunk.iter().enumerate() {
                if &entry == id {
                    found_chunk_idx = Some(ci);
                    found_pos = Some(pos as u32);
                    break 'outer;
                }
            }
        }

        let (target_chunk, target_pos) = match (found_chunk_idx, found_pos) {
            (Some(c), Some(p)) => (c, p),
            _ => return, // ID not found — nothing to do.
        };

        // Determine the last element's location.
        let last_idx = count - 1;
        let last_chunk_idx = last_idx / CHUNK_SIZE;
        let last_pos = last_idx % CHUNK_SIZE;

        if target_chunk == last_chunk_idx && target_pos == last_pos {
            // Target IS the last element — just pop it.
            let chunk_key = Self::subject_chunk_key(subject, target_chunk);
            let mut chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&chunk_key)
                .unwrap_or(Vec::new(env));
            let mut new_chunk = Vec::new(env);
            for (i, v) in chunk.iter().enumerate() {
                if i as u32 != target_pos {
                    new_chunk.push_back(v);
                }
            }
            env.storage().persistent().set(&chunk_key, &new_chunk);
            env.storage().persistent().extend_ttl(&chunk_key, ttl, ttl);
        } else {
            // Swap target with last element, then pop last.
            let last_chunk_key = Self::subject_chunk_key(subject, last_chunk_idx);
            let mut last_chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&last_chunk_key)
                .unwrap_or(Vec::new(env));
            let last_val = match last_chunk.get(last_pos) {
                Some(v) => v,
                None => return,
            };

            // Write last_val into target position.
            let target_chunk_key = Self::subject_chunk_key(subject, target_chunk);
            let mut target_c: Vec<String> = env
                .storage()
                .persistent()
                .get(&target_chunk_key)
                .unwrap_or(Vec::new(env));
            let mut new_target = Vec::new(env);
            for (i, v) in target_c.iter().enumerate() {
                if i as u32 == target_pos {
                    new_target.push_back(last_val.clone());
                } else {
                    new_target.push_back(v);
                }
            }
            env.storage().persistent().set(&target_chunk_key, &new_target);
            env.storage().persistent().extend_ttl(&target_chunk_key, ttl, ttl);

            // Pop last element from last chunk.
            let mut new_last = Vec::new(env);
            for (i, v) in last_chunk.iter().enumerate() {
                if i as u32 != last_pos {
                    new_last.push_back(v);
                }
            }
            env.storage().persistent().set(&last_chunk_key, &new_last);
            env.storage().persistent().extend_ttl(&last_chunk_key, ttl, ttl);
        }

        // Decrement count.
        let count_key = Self::subject_count_key(subject);
        let new_count = count - 1;
        env.storage().persistent().set(&count_key, &new_count);
        env.storage().persistent().extend_ttl(&count_key, ttl, ttl);
    }

    /// Load only the chunks needed to serve page (start, limit).
    ///
    /// Returns a flat Vec<String> of IDs from position `start` up to
    /// `start + limit`, loading at most `ceil(limit / CHUNK_SIZE) + 1` chunks.
    pub fn get_subject_page(env: &Env, subject: &Address, start: u32, limit: u32) -> Vec<String> {
        let count = Self::subject_count(env, subject);
        let mut result = Vec::new(env);
        if start >= count || limit == 0 {
            return result;
        }
        let end = (start + limit).min(count);
        let first_chunk = start / CHUNK_SIZE;
        let last_chunk = (end - 1) / CHUNK_SIZE;

        for ci in first_chunk..=last_chunk {
            let chunk_key = Self::subject_chunk_key(subject, ci);
            let chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&chunk_key)
                .unwrap_or(Vec::new(env));
            let chunk_start = ci * CHUNK_SIZE; // absolute index of chunk[0]
            for (local_pos, id) in chunk.iter().enumerate() {
                let abs_pos = chunk_start + local_pos as u32;
                if abs_pos >= start && abs_pos < end {
                    result.push_back(id);
                }
            }
        }
        result
    }

    /// Load all IDs for a subject by iterating every chunk.
    ///
    /// Prefer `get_subject_page` for paginated queries. This is provided for
    /// operations that genuinely need the full index (e.g. count queries).
    pub fn get_subject_all(env: &Env, subject: &Address) -> Vec<String> {
        let count = Self::subject_count(env, subject);
        let mut result = Vec::new(env);
        if count == 0 {
            return result;
        }
        let total_chunks = (count + CHUNK_SIZE - 1) / CHUNK_SIZE;
        for ci in 0..total_chunks {
            let chunk_key = Self::subject_chunk_key(subject, ci);
            let chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&chunk_key)
                .unwrap_or(Vec::new(env));
            for id in chunk.iter() {
                result.push_back(id);
            }
        }
        result
    }

    // ── Issuer index ──────────────────────────────────────────────────────────

    fn issuer_count_key(issuer: &Address) -> StorageKey {
        StorageKey::IssuerAttestationsCount(issuer.clone())
    }

    fn issuer_chunk_key(issuer: &Address, chunk: u32) -> StorageKey {
        StorageKey::IssuerAttestationsChunk(issuer.clone(), chunk)
    }

    /// Total number of IDs in the issuer's chunked index.
    pub fn issuer_count(env: &Env, issuer: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&Self::issuer_count_key(issuer))
            .unwrap_or(0u32)
    }

    /// Append one ID to the issuer's chunked index.
    pub fn add_issuer(env: &Env, issuer: &Address, id: &String) {
        let ttl = get_ttl_lifetime(env);
        let count = Self::issuer_count(env, issuer);
        let chunk_idx = count / CHUNK_SIZE;
        let chunk_key = Self::issuer_chunk_key(issuer, chunk_idx);

        let mut chunk: Vec<String> = env
            .storage()
            .persistent()
            .get(&chunk_key)
            .unwrap_or(Vec::new(env));
        chunk.push_back(id.clone());
        env.storage().persistent().set(&chunk_key, &chunk);
        env.storage().persistent().extend_ttl(&chunk_key, ttl, ttl);

        let new_count = count + 1;
        let count_key = Self::issuer_count_key(issuer);
        env.storage().persistent().set(&count_key, &new_count);
        env.storage().persistent().extend_ttl(&count_key, ttl, ttl);
    }

    /// Append multiple IDs to the issuer's chunked index in as few writes as possible.
    ///
    /// Used by `create_attestations_batch` — fills the current tail chunk before
    /// opening new ones, so a batch of N IDs touches at most ceil(N/CHUNK_SIZE)+1 chunks.
    pub fn add_issuer_bulk(env: &Env, issuer: &Address, ids: &Vec<String>) {
        if ids.is_empty() {
            return;
        }
        let ttl = get_ttl_lifetime(env);
        let mut count = Self::issuer_count(env, issuer);

        let mut pending: Vec<String> = Vec::new(env);
        for id in ids.iter() {
            pending.push_back(id);
        }

        let mut pi = 0u32;
        while pi < pending.len() {
            let chunk_idx = count / CHUNK_SIZE;
            let chunk_key = Self::issuer_chunk_key(issuer, chunk_idx);
            let mut chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&chunk_key)
                .unwrap_or(Vec::new(env));

            let space = CHUNK_SIZE - (count % CHUNK_SIZE);
            let take = space.min(pending.len() - pi);
            for i in 0..take {
                if let Some(id) = pending.get(pi + i) {
                    chunk.push_back(id);
                    count += 1;
                }
            }
            pi += take;

            env.storage().persistent().set(&chunk_key, &chunk);
            env.storage().persistent().extend_ttl(&chunk_key, ttl, ttl);
        }

        let count_key = Self::issuer_count_key(issuer);
        env.storage().persistent().set(&count_key, &count);
        env.storage().persistent().extend_ttl(&count_key, ttl, ttl);
    }

    /// Remove one ID from the issuer's chunked index (swap-with-last strategy).
    pub fn remove_issuer(env: &Env, issuer: &Address, id: &String) {
        let ttl = get_ttl_lifetime(env);
        let count = Self::issuer_count(env, issuer);
        if count == 0 {
            return;
        }

        let total_chunks = (count + CHUNK_SIZE - 1) / CHUNK_SIZE;
        let mut found_chunk_idx: Option<u32> = None;
        let mut found_pos: Option<u32> = None;

        'outer: for ci in (0..total_chunks).rev() {
            let chunk_key = Self::issuer_chunk_key(issuer, ci);
            let chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&chunk_key)
                .unwrap_or(Vec::new(env));
            for (pos, entry) in chunk.iter().enumerate() {
                if &entry == id {
                    found_chunk_idx = Some(ci);
                    found_pos = Some(pos as u32);
                    break 'outer;
                }
            }
        }

        let (target_chunk, target_pos) = match (found_chunk_idx, found_pos) {
            (Some(c), Some(p)) => (c, p),
            _ => return,
        };

        let last_idx = count - 1;
        let last_chunk_idx = last_idx / CHUNK_SIZE;
        let last_pos = last_idx % CHUNK_SIZE;

        if target_chunk == last_chunk_idx && target_pos == last_pos {
            let chunk_key = Self::issuer_chunk_key(issuer, target_chunk);
            let mut chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&chunk_key)
                .unwrap_or(Vec::new(env));
            let mut new_chunk = Vec::new(env);
            for (i, v) in chunk.iter().enumerate() {
                if i as u32 != target_pos {
                    new_chunk.push_back(v);
                }
            }
            env.storage().persistent().set(&chunk_key, &new_chunk);
            env.storage().persistent().extend_ttl(&chunk_key, ttl, ttl);
        } else {
            let last_chunk_key = Self::issuer_chunk_key(issuer, last_chunk_idx);
            let mut last_chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&last_chunk_key)
                .unwrap_or(Vec::new(env));
            let last_val = match last_chunk.get(last_pos) {
                Some(v) => v,
                None => return,
            };

            let target_chunk_key = Self::issuer_chunk_key(issuer, target_chunk);
            let mut target_c: Vec<String> = env
                .storage()
                .persistent()
                .get(&target_chunk_key)
                .unwrap_or(Vec::new(env));
            let mut new_target = Vec::new(env);
            for (i, v) in target_c.iter().enumerate() {
                if i as u32 == target_pos {
                    new_target.push_back(last_val.clone());
                } else {
                    new_target.push_back(v);
                }
            }
            env.storage().persistent().set(&target_chunk_key, &new_target);
            env.storage().persistent().extend_ttl(&target_chunk_key, ttl, ttl);

            let mut new_last = Vec::new(env);
            for (i, v) in last_chunk.iter().enumerate() {
                if i as u32 != last_pos {
                    new_last.push_back(v);
                }
            }
            env.storage().persistent().set(&last_chunk_key, &new_last);
            env.storage().persistent().extend_ttl(&last_chunk_key, ttl, ttl);
        }

        let count_key = Self::issuer_count_key(issuer);
        let new_count = count - 1;
        env.storage().persistent().set(&count_key, &new_count);
        env.storage().persistent().extend_ttl(&count_key, ttl, ttl);
    }

    /// Load only the chunks needed to serve page (start, limit) for an issuer.
    pub fn get_issuer_page(env: &Env, issuer: &Address, start: u32, limit: u32) -> Vec<String> {
        let count = Self::issuer_count(env, issuer);
        let mut result = Vec::new(env);
        if start >= count || limit == 0 {
            return result;
        }
        let end = (start + limit).min(count);
        let first_chunk = start / CHUNK_SIZE;
        let last_chunk = (end - 1) / CHUNK_SIZE;

        for ci in first_chunk..=last_chunk {
            let chunk_key = Self::issuer_chunk_key(issuer, ci);
            let chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&chunk_key)
                .unwrap_or(Vec::new(env));
            let chunk_start = ci * CHUNK_SIZE;
            for (local_pos, id) in chunk.iter().enumerate() {
                let abs_pos = chunk_start + local_pos as u32;
                if abs_pos >= start && abs_pos < end {
                    result.push_back(id);
                }
            }
        }
        result
    }

    /// Load all IDs for an issuer by iterating every chunk.
    pub fn get_issuer_all(env: &Env, issuer: &Address) -> Vec<String> {
        let count = Self::issuer_count(env, issuer);
        let mut result = Vec::new(env);
        if count == 0 {
            return result;
        }
        let total_chunks = (count + CHUNK_SIZE - 1) / CHUNK_SIZE;
        for ci in 0..total_chunks {
            let chunk_key = Self::issuer_chunk_key(issuer, ci);
            let chunk: Vec<String> = env
                .storage()
                .persistent()
                .get(&chunk_key)
                .unwrap_or(Vec::new(env));
            for id in chunk.iter() {
                result.push_back(id);
            }
        }
        result
    }
}
