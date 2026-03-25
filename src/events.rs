use soroban_sdk::{symbol_short, Address, Env, String};

use crate::types::{Attestation, IssuerTier};

pub struct Events;

impl Events {
    pub fn admin_initialized(env: &Env, admin: &Address, timestamp: u64) {
        env.events()
            .publish((symbol_short!("adm_init"),), (admin.clone(), timestamp));
    }

    pub fn attestation_created(env: &Env, attestation: &Attestation) {
        env.events().publish(
            (symbol_short!("created"), attestation.subject.clone()),
            (
                attestation.id.clone(),
                attestation.issuer.clone(),
                attestation.claim_type.clone(),
                attestation.timestamp,
                attestation.metadata.clone(),
            ),
        );
    }

    pub fn attestation_imported(env: &Env, attestation: &Attestation) {
        env.events().publish(
            (symbol_short!("imported"), attestation.subject.clone()),
            (
                attestation.id.clone(),
                attestation.issuer.clone(),
                attestation.claim_type.clone(),
                attestation.timestamp,
                attestation.expiration,
            ),
        );
    }

    pub fn attestation_bridged(env: &Env, attestation: &Attestation) {
        env.events().publish(
            (symbol_short!("bridged"), attestation.subject.clone()),
            (
                attestation.id.clone(),
                attestation.issuer.clone(),
                attestation.claim_type.clone(),
                attestation
                    .source_chain
                    .clone()
                    .unwrap_or(String::from_str(env, "")),
                attestation
                    .source_tx
                    .clone()
                    .unwrap_or(String::from_str(env, "")),
            ),
        );
    }

    pub fn attestation_revoked(env: &Env, attestation_id: &String, issuer: &Address) {
        env.events().publish(
            (symbol_short!("revoked"), issuer.clone()),
            attestation_id.clone(),
        );
    }

    pub fn attestation_renewed(
        env: &Env,
        attestation_id: &String,
        issuer: &Address,
        new_expiration: Option<u64>,
    ) {
        env.events().publish(
            (symbol_short!("renewed"), issuer.clone()),
            (attestation_id.clone(), new_expiration),
        );
    }

    pub fn attestation_updated(
        env: &Env,
        attestation_id: &String,
        issuer: &Address,
        new_expiration: Option<u64>,
    ) {
        env.events().publish(
            (symbol_short!("updated"), issuer.clone()),
            (attestation_id.clone(), new_expiration),
        );
    }

    pub fn attestation_expired(env: &Env, attestation_id: &String, subject: &Address) {
        env.events().publish(
            (symbol_short!("expired"), subject.clone()),
            attestation_id.clone(),
        );
    }

    pub fn issuer_registered(env: &Env, issuer: &Address, admin: &Address, timestamp: u64) {
        env.events().publish(
            (symbol_short!("iss_reg"), issuer.clone()),
            (admin.clone(), timestamp),
        );
    }

    /// Emitted when an issuer's tier is set or updated by the admin.
    pub fn issuer_tier_updated(env: &Env, issuer: &Address, tier: &IssuerTier) {
        env.events().publish(
            (symbol_short!("iss_tier"), issuer.clone()),
            tier.clone(),
        );
    }

    pub fn issuer_removed(env: &Env, issuer: &Address, admin: &Address, timestamp: u64) {
        env.events().publish(
            (symbol_short!("iss_rem"), issuer.clone()),
            (admin.clone(), timestamp),
        );
    }

    pub fn claim_type_registered(env: &Env, claim_type: &String, description: &String) {
        env.events().publish(
            (symbol_short!("clmtype"), claim_type.clone()),
            description.clone(),
        );
    }

    /// Emitted when a new multi-sig proposal is created.
    pub fn multisig_proposed(
        env: &Env,
        proposal_id: &String,
        proposer: &Address,
        subject: &Address,
        threshold: u32,
    ) {
        env.events().publish(
            (symbol_short!("ms_prop"), subject.clone()),
            (proposal_id.clone(), proposer.clone(), threshold),
        );
    }

    /// Emitted when an issuer co-signs a multi-sig proposal.
    pub fn multisig_cosigned(
        env: &Env,
        proposal_id: &String,
        signer: &Address,
        signatures_so_far: u32,
        threshold: u32,
    ) {
        env.events().publish(
            (symbol_short!("ms_sign"), signer.clone()),
            (proposal_id.clone(), signatures_so_far, threshold),
        );
    }

    /// Emitted when a multi-sig proposal reaches threshold and the attestation is activated.
    pub fn multisig_activated(env: &Env, proposal_id: &String, attestation_id: &String) {
        env.events().publish(
            (symbol_short!("ms_actv"),),
            (proposal_id.clone(), attestation_id.clone()),
        );
    }

    /// Emitted when a new attestation template is created by an issuer.
    pub fn template_created(env: &Env, issuer: &Address, template_id: &String) {
        env.events().publish(
            (symbol_short!("tmpl_crt"), issuer.clone()),
            template_id.clone(),
        );
    }
}
