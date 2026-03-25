//! Storage helpers for TrustLink.
//!
//! This module is the single point of contact between the contract logic and
//! on-chain storage. No other module calls `env.storage()` directly.
//!
//! ## Storage tiers
//!
//! | Tier         | Keys stored                          | TTL policy                        |
//! |--------------|--------------------------------------|-----------------------------------|
//! | Instance     | `Admin`, `Version`, `FeeConfig`      | Refreshed to 30 days on each write|
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
//! - `Template(Address, String)` — full [`AttestationTemplate`] record keyed by `(issuer, template_id)`.
//! - `TemplateRegistry(Address)` — ordered `Vec<String>` of template IDs for an issuer (insertion order).

use crate::types::{Attestation, AttestationTemplate, ClaimTypeInfo, Error, FeeConfig, IssuerMetadata, MultiSigProposal, TtlConfig};
use soroban_sdk::{contracttype, Address, Env, String, Vec};

/// Keys used to address data in contract storage.
#[contracttype]
pub enum StorageKey {
    /// The contract administrator address.
    Admin,
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
    /// A multi-sig attestation proposal keyed by its ID.
    MultiSigProposal(String),
    /// Full [`AttestationTemplate`] record keyed by `(issuer, template_id)`.
    Template(Address, String),
    /// Ordered `Vec<String>` of template IDs for an issuer (insertion order).
    TemplateRegistry(Address),
}

const DAY_IN_LEDGERS: u32 = 17280;
const DEFAULT_TTL_DAYS: u32 = 30;
const DEFAULT_INSTANCE_LIFETIME: u32 = DAY_IN_LEDGERS * DEFAULT_TTL_DAYS;

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
    /// Return `true` if the admin key exists in instance storage.
    pub fn has_admin(env: &Env) -> bool {
        env.storage().instance().has(&StorageKey::Admin)
    }

    /// Persist `admin` in instance storage and refresh the instance TTL.
    pub fn set_admin(env: &Env, admin: &Address) {
        let ttl = get_ttl_lifetime(env);
        env.storage().instance().set(&StorageKey::Admin, admin);
        env.storage().instance().extend_ttl(ttl, ttl);
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

    /// Retrieve the admin address.
    ///
    /// # Errors
    /// - [`Error::NotInitialized`] — admin key is absent.
    pub fn get_admin(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&StorageKey::Admin)
            .ok_or(Error::NotInitialized)
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

    /// Retrieve an attestation by `id`.
    ///
    /// Extends the entry's TTL on every successful read so that frequently
    /// accessed attestations are not evicted while still in active use.
    /// The threshold and extension amount match the write-path constants,
    /// meaning a read is never more expensive than a write in terms of
    /// storage rent.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — no attestation with that ID exists.
    pub fn get_attestation(env: &Env, id: &String) -> Result<Attestation, Error> {
        let key = StorageKey::Attestation(id.clone());
        let attestation = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotFound)?;
        // Extend TTL on read so hot attestations stay alive without a write.
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
        Ok(attestation)
    }

    /// Return the ordered list of attestation IDs for `subject`, or an empty
    /// [`Vec`] if none exist.
    ///
    /// Extends the index TTL on read using the same threshold and amount as
    /// write operations — no additional storage cost beyond what a write incurs.
    pub fn get_subject_attestations(env: &Env, subject: &Address) -> Vec<String> {
        let key = StorageKey::SubjectAttestations(subject.clone());
        let list = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env));
        if env.storage().persistent().has(&key) {
            let ttl = get_ttl_lifetime(env);
            env.storage().persistent().extend_ttl(&key, ttl, ttl);
        }
        list
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

    /// Return the ordered list of attestation IDs created by `issuer`, or an
    /// empty [`Vec`] if none exist.
    ///
    /// Extends the index TTL on read using the same threshold and amount as
    /// write operations — no additional storage cost beyond what a write incurs.
    pub fn get_issuer_attestations(env: &Env, issuer: &Address) -> Vec<String> {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        let list = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env));
        if env.storage().persistent().has(&key) {
            let ttl = get_ttl_lifetime(env);
            env.storage().persistent().extend_ttl(&key, ttl, ttl);
        }
        list
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

    /// Persist a [`MultiSigProposal`] and refresh its TTL.
    pub fn set_multisig_proposal(env: &Env, proposal: &MultiSigProposal) {
        let key = StorageKey::MultiSigProposal(proposal.id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, proposal);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve a [`MultiSigProposal`] by ID.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — no proposal with that ID exists.
    pub fn get_multisig_proposal(env: &Env, id: &String) -> Result<MultiSigProposal, Error> {
        env.storage()
            .persistent()
            .get(&StorageKey::MultiSigProposal(id.clone()))
            .ok_or(Error::NotFound)
    }

    /// Return `true` if a proposal with `id` exists.
    pub fn has_multisig_proposal(env: &Env, id: &String) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::MultiSigProposal(id.clone()))
    }

    /// Persist `template` keyed by `(issuer, template_id)` and refresh its TTL.
    pub fn set_template(env: &Env, issuer: &Address, template_id: &String, template: &AttestationTemplate) {
        let key = StorageKey::Template(issuer.clone(), template_id.clone());
        let ttl = get_ttl_lifetime(env);
        env.storage().persistent().set(&key, template);
        env.storage().persistent().extend_ttl(&key, ttl, ttl);
    }

    /// Retrieve a template by `(issuer, template_id)`, or `None` if absent.
    pub fn get_template(env: &Env, issuer: &Address, template_id: &String) -> Option<AttestationTemplate> {
        env.storage()
            .persistent()
            .get(&StorageKey::Template(issuer.clone(), template_id.clone()))
    }

    /// Return `true` if a template keyed by `(issuer, template_id)` exists.
    pub fn has_template(env: &Env, issuer: &Address, template_id: &String) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::Template(issuer.clone(), template_id.clone()))
    }

    /// Return the ordered list of template IDs for `issuer`, or an empty
    /// [`Vec`] if none exist.
    pub fn get_template_registry(env: &Env, issuer: &Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&StorageKey::TemplateRegistry(issuer.clone()))
            .unwrap_or(Vec::new(env))
    }

    /// Append `template_id` to `issuer`'s template registry ONLY if not already
    /// present (preserves insertion order, no duplicates), then refresh TTL.
    pub fn add_to_template_registry(env: &Env, issuer: &Address, template_id: &String) {
        let key = StorageKey::TemplateRegistry(issuer.clone());
        let ttl = get_ttl_lifetime(env);
        let mut registry = Self::get_template_registry(env, issuer);
        // Only append if the ID is not already in the registry.
        let already_present = registry.iter().any(|id| id == *template_id);
        if !already_present {
            registry.push_back(template_id.clone());
            env.storage().persistent().set(&key, &registry);
            env.storage().persistent().extend_ttl(&key, ttl, ttl);
        }
    }
}
