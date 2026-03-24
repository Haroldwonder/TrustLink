//! Event emission for TrustLink.

use soroban_sdk::{symbol_short, Address, Env, String};
use crate::types::Attestation;

pub struct Events;

impl Events {
    pub fn attestation_created(env: &Env, attestation: &Attestation) {
        env.events().publish(
            (symbol_short!("created"), attestation.subject.clone()),
            (
                attestation.id.clone(),
                attestation.issuer.clone(),
                attestation.claim_type.clone(),
                attestation.timestamp,
            ),
        );
    }

    pub fn attestation_revoked(env: &Env, attestation_id: &String, issuer: &Address) {
        env.events().publish(
            (symbol_short!("revoked"), issuer.clone()),
            attestation_id.clone(),
        );
    }

    pub fn attestation_renewed(env: &Env, attestation_id: &String, issuer: &Address, new_expiration: Option<u64>) {
        env.events().publish(
            (symbol_short!("renewed"), issuer.clone()),
            (attestation_id.clone(), new_expiration),
        );
    }

    pub fn attestation_expired(env: &Env, attestation_id: &String, subject: &Address) {
        env.events().publish(
            (symbol_short!("expired"), subject.clone()),
            attestation_id.clone(),
        );
    }

    /// Emitted when the contract is successfully initialized.
    ///
    /// topics: ("admin_init",)
    /// data:   (admin: Address, timestamp: u64)
    pub fn admin_initialized(env: &Env, admin: &Address, timestamp: u64) {
        env.events().publish(
            (symbol_short!("admin_init"),),
            (admin.clone(), timestamp),
        );
    }

    pub fn attestation_updated(env: &Env, attestation_id: &String, issuer: &Address, new_expiration: Option<u64>) {
        env.events().publish(
            (symbol_short!("updated"), issuer.clone()),
            (attestation_id.clone(), new_expiration),
        );
    }

    pub fn issuer_registered(env: &Env, issuer: &Address, admin: &Address) {
        env.events().publish(
            (symbol_short!("iss_reg"), issuer.clone()),
            admin.clone(),
        );
    }

    pub fn issuer_removed(env: &Env, issuer: &Address, admin: &Address) {
        env.events().publish(
            (symbol_short!("iss_rem"), issuer.clone()),
            admin.clone(),
        );
    }

    pub fn claim_type_registered(env: &Env, claim_type: &String, description: &String) {
        env.events().publish(
            (symbol_short!("clmtype"),),
            (claim_type.clone(), description.clone()),
        );
    }

    pub fn contract_upgraded(env: &Env, admin: &Address) {
        env.events().publish(
            (symbol_short!("upgraded"),),
            admin.clone(),
        );
    }
}
