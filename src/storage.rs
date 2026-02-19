use soroban_sdk::{contracttype, Address, Env, String, Vec};
use crate::types::{Attestation, Error};

#[contracttype]
pub enum StorageKey {
    Admin,
    Issuer(Address),
    Attestation(String),
    SubjectAttestations(Address),
    IssuerAttestations(Address),
}

const DAY_IN_LEDGERS: u32 = 17280;
const INSTANCE_LIFETIME: u32 = DAY_IN_LEDGERS * 30; // 30 days
const TEMP_LIFETIME: u32 = DAY_IN_LEDGERS * 7; // 7 days

pub struct Storage;

impl Storage {
    // Admin management
    pub fn has_admin(env: &Env) -> bool {
        env.storage().instance().has(&StorageKey::Admin)
    }
    
    pub fn set_admin(env: &Env, admin: &Address) {
        env.storage().instance().set(&StorageKey::Admin, admin);
        env.storage().instance().extend_ttl(INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }
    
    pub fn get_admin(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&StorageKey::Admin)
            .ok_or(Error::NotInitialized)
    }
    
    // Issuer management
    pub fn is_issuer(env: &Env, address: &Address) -> bool {
        let key = StorageKey::Issuer(address.clone());
        env.storage().persistent().has(&key)
    }
    
    pub fn add_issuer(env: &Env, issuer: &Address) {
        let key = StorageKey::Issuer(issuer.clone());
        env.storage().persistent().set(&key, &true);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }
    
    pub fn remove_issuer(env: &Env, issuer: &Address) {
        let key = StorageKey::Issuer(issuer.clone());
        env.storage().persistent().remove(&key);
    }
    
    // Attestation management
    pub fn has_attestation(env: &Env, id: &String) -> bool {
        let key = StorageKey::Attestation(id.clone());
        env.storage().persistent().has(&key)
    }
    
    pub fn set_attestation(env: &Env, attestation: &Attestation) {
        let key = StorageKey::Attestation(attestation.id.clone());
        env.storage().persistent().set(&key, attestation);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }
    
    pub fn get_attestation(env: &Env, id: &String) -> Result<Attestation, Error> {
        let key = StorageKey::Attestation(id.clone());
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::NotFound)
    }
    
    // Subject attestations index
    pub fn get_subject_attestations(env: &Env, subject: &Address) -> Vec<String> {
        let key = StorageKey::SubjectAttestations(subject.clone());
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env))
    }
    
    pub fn add_subject_attestation(env: &Env, subject: &Address, attestation_id: &String) {
        let key = StorageKey::SubjectAttestations(subject.clone());
        let mut attestations = Self::get_subject_attestations(env, subject);
        attestations.push_back(attestation_id.clone());
        env.storage().persistent().set(&key, &attestations);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }
    
    // Issuer attestations index
    pub fn get_issuer_attestations(env: &Env, issuer: &Address) -> Vec<String> {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env))
    }
    
    pub fn add_issuer_attestation(env: &Env, issuer: &Address, attestation_id: &String) {
        let key = StorageKey::IssuerAttestations(issuer.clone());
        let mut attestations = Self::get_issuer_attestations(env, issuer);
        attestations.push_back(attestation_id.clone());
        env.storage().persistent().set(&key, &attestations);
        env.storage().persistent().extend_ttl(&key, INSTANCE_LIFETIME, INSTANCE_LIFETIME);
    }
}
