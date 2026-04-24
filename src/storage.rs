//! Storage helpers for TrustLink.
//!
//! This module is the single point of contact between the contract logic and
//! on-chain storage. No other module calls `env.storage()` directly.
//!
//! ## Storage tiers
//!
//! | Tier         | Keys stored                          | TTL policy                        |
//! |--------------|--------------------------------------|-----------------------------------|
//! | Instance     | `Admin`, `Version`                   | Refreshed to 30 days on each write|
//! | Persistent   | Everything else (see [`StorageKey`]) | Refreshed to 30 days on each write|
//!
//! ## Key layout (`StorageKey`)
//!
//! - `Admin` — the single contract administrator address.
//! - `Version` — semver string set at initialization (e.g. `"1.0.0"`).
//! - `Issuer(Address)` — presence flag (`bool`) for each registered issuer.
//! - `Attestation(String)` — full [`Attestation`] record keyed by its ID.
//! - `SubjectAttestations(Address)` — ordered `Vec<String>` of attestation IDs
//!   for a given subject; used for pagination and claim lookups.
//! - `IssuerAttestations(Address)` — ordered `Vec<String>` of attestation IDs
//!   created by a given issuer.
//! - `IssuerMetadata(Address)` — optional [`IssuerMetadata`] set by the issuer.
//! - `ClaimType(String)` — [`ClaimTypeInfo`] record for a registered claim type.
//! - `ClaimTypeList` — ordered `Vec<String>` of all registered claim type IDs;
//!   used for pagination via `list_claim_types`.

use soroban_sdk::{contracttype, Address, Env, String, Vec};
use crate::types::{Attestation, ClaimTypeInfo, Error, IssuerMetadata};

/// Keys used to address data in contract storage.
#[contracttype]
pub enum StorageKey {
    /// The contract administrator address.
    Admin,
    /// Semver version string set at initialization.
    Version,
    /// Presence flag for a registered issuer.
    Issuer(Address),
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
}

const DAY_IN_LEDGERS: u32 = 17280;
const INSTANCE_LIFETIME: u32 = DAY_IN_LEDGERS * 30; // 30 days

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
        env.storage().instance().set(&StorageKey::Admin, admin);
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }

    /// Persist `version` in instance storage alongside the admin.
    pub fn set_version(env: &Env, version: &String) {
        env.storage().instance().set(&StorageKey::Version, version);
    }

    /// Retrieve the contract version string.
    ///
    /// Returns `None` if the contract has not been initialized yet.
    pub fn get_version(env: &Env) -> Option<String> {
        env.storage().instance().get(&StorageKey::Version)
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
        env.storage().persistent().has(&StorageKey::Issuer(address.clone()))
    }

    /// Add `issuer` to the registry and refresh its TTL.
    pub fn add_issuer(env: &Env, issuer: &Address) {
        let key = StorageKey::Issuer(issuer.clone());
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }

    /// Remove `issuer` from the registry.
    pub fn remove_issuer(env: &Env, issuer: &Address) {
        env.storage().persistent().remove(&StorageKey::Issuer(issuer.clone()));
    }

    /// Return `true` if an attestation with `id` exists in storage.
    pub fn has_attestation(env: &Env, id: &String) -> bool {
        env.storage().persistent().has(&StorageKey::Attestation(id.clone()))
    }

    /// Persist `attestation` and refresh its TTL.
    pub fn set_attestation(env: &Env, attestation: &Attestation) {
        let key = StorageKey::Attestation(attestation.id.clone());
        env.storage().persistent().set(&key, attestation);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }

    /// Retrieve an attestation by `id`.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — no attestation with that ID exists.
    pub fn get_attestation(env: &Env, id: &String) -> Result<Attestation, Error> {
        env.storage()
            .persistent()
            .get(&StorageKey::Attestation(id.clone()))
            .ok_or(Error::NotFound)
    }

    /// Return the ordered list of attestation IDs for `subject`, or an empty
    /// [`Vec`] if none exist.
    pub fn get_subject_attestations(env: &Env, subject: &Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&StorageKey::SubjectAttestations(subject.clone()))
            .unwrap_or(Vec::new(env))
    }

    /// Append `attestation_id` to `subject`'s attestation index and refresh TTL.
    pub fn add_subject_attestation(env: &Env, subject: &Address, attestation_id: &String) {
        let key = StorageKey::SubjectAttestations(subject.clone());
        let mut attestations = Self::get_subject_attestations(env, subject);
        attestations.push_back(attestation_id.clone());
        env.storage().persistent().set(&key, &attestations);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }

    /// Return the ordered list of attestation IDs created by `issuer`, or an
    /// empty [`Vec`] if none exist.
    pub fn get_issuer_attestations(env: &Env, issuer: &Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerAttestations(issuer.clone()))
            .unwrap_or(Vec::new(env))
    }

    /// Append `attestation_id` to `issuer`'s attestation index and refresh TTL.
    pub fn add_issuer_attestation(env: &Env, issuer: &Address, attestation_id: &String) {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        let mut attestations = Self::get_issuer_attestations(env, issuer);
        attestations.push_back(attestation_id.clone());
        env.storage().persistent().set(&key, &attestations);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }

    /// Persist `metadata` for `issuer` and refresh its TTL.
    pub fn set_issuer_metadata(env: &Env, issuer: &Address, metadata: &IssuerMetadata) {
        let key = StorageKey::IssuerMetadata(issuer.clone());
        env.storage().persistent().set(&key, metadata);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }

    /// Retrieve metadata for `issuer`, or `None` if not set.
    pub fn get_issuer_metadata(env: &Env, issuer: &Address) -> Option<IssuerMetadata> {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerMetadata(issuer.clone()))
    }

    /// Persist a claim type info record and add it to the ordered list if new.
    pub fn set_claim_type(env: &Env, info: &ClaimTypeInfo) {
        let key = StorageKey::ClaimType(info.claim_type.clone());
        let is_new = !env.storage().persistent().has(&key);
        env.storage().persistent().set(&key, info);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
        if is_new {
            let list_key = StorageKey::ClaimTypeList;
            let mut list: Vec<String> = env.storage()
                .persistent()
                .get(&list_key)
                .unwrap_or(Vec::new(env));
            list.push_back(info.claim_type.clone());
            env.storage().persistent().set(&list_key, &list);
            env.storage().persistent().extend_ttl(&list_key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
        }
    }

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
        env.storage().persistent().set(&key, &enabled);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }

    /// Return `true` if whitelist mode is enabled for `issuer`.
    pub fn is_whitelist_mode(env: &Env, issuer: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::IssuerWhitelistMode(issuer.clone()))
            .unwrap_or(false)
    }

    /// Add `subject` to `issuer`'s whitelist.
    pub fn add_to_whitelist(env: &Env, issuer: &Address, subject: &Address) {
        let key = StorageKey::IssuerWhitelist(issuer.clone(), subject.clone());
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
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
}
