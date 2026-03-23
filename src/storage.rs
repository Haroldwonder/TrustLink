//! Storage helpers for TrustLink.
//!
//! All persistent data uses a 30-day TTL that is refreshed on every write.
//! Instance storage (admin) shares a single TTL entry; persistent storage
//! (issuers, attestations, indexes) each have their own TTL entry.

use soroban_sdk::{contracttype, Address, Env, String, Vec};
use crate::types::{Attestation, ClaimTypeInfo, Error};

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
    /// Registered claim type info keyed by claim type string.
    ClaimType(String),
    /// Ordered list of all registered claim type strings.
    ClaimTypeList,
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

    /// Return `true` if a claim type is registered.
    pub fn has_claim_type(env: &Env, claim_type: &String) -> bool {
        env.storage().persistent().has(&StorageKey::ClaimType(claim_type.clone()))
    }

    /// Persist a claim type and its description, and append to the ordered list.
    pub fn set_claim_type(env: &Env, info: &ClaimTypeInfo) {
        let key = StorageKey::ClaimType(info.claim_type.clone());
        let is_new = !env.storage().persistent().has(&key);
        env.storage().persistent().set(&key, info);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);

        if is_new {
            let list_key = StorageKey::ClaimTypeList;
            let mut list: Vec<String> = env.storage().persistent()
                .get(&list_key)
                .unwrap_or(Vec::new(env));
            list.push_back(info.claim_type.clone());
            env.storage().persistent().set(&list_key, &list);
            env.storage().persistent().extend_ttl(&list_key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
        }
    }

    /// Retrieve a claim type's info, or `None` if not registered.
    pub fn get_claim_type(env: &Env, claim_type: &String) -> Option<ClaimTypeInfo> {
        env.storage().persistent().get(&StorageKey::ClaimType(claim_type.clone()))
    }

    /// Return the ordered list of all registered claim type strings.
    pub fn get_claim_type_list(env: &Env) -> Vec<String> {
        env.storage().persistent()
            .get(&StorageKey::ClaimTypeList)
            .unwrap_or(Vec::new(env))
    }
}
