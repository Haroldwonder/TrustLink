use soroban_sdk::{symbol_short, Address, Env, String};
use crate::types::Attestation;

pub struct Events;

impl Events {
    /// Emit event when an attestation is created
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
    
    /// Emit event when an attestation is revoked
    pub fn attestation_revoked(env: &Env, attestation_id: &String, issuer: &Address) {
        env.events().publish(
            (symbol_short!("revoked"), issuer.clone()),
            attestation_id.clone(),
        );
    }
}
