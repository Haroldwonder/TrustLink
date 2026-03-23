//! Event emission for TrustLink.
//!
//! All state-changing operations and expiration checks emit a Soroban event so
//! that off-chain indexers can react without polling contract storage.

use soroban_sdk::{symbol_short, Address, Env, String};
use crate::types::Attestation;

/// Emits TrustLink contract events.
pub struct Events;

impl Events {
    /// Emit an event when a new attestation is created.
    ///
    /// # Event schema
    /// ```text
    /// topics: ("created", subject: Address)
    /// data:   (attestation_id: String, issuer: Address, claim_type: String, timestamp: u64)
    /// ```
    ///
    /// # Parameters
    /// - `attestation` — the newly created attestation.
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

    /// Emit an event when an attestation is revoked.
    ///
    /// # Event schema
    /// ```text
    /// topics: ("revoked", issuer: Address)
    /// data:   attestation_id: String
    /// ```
    ///
    /// # Parameters
    /// - `attestation_id` — ID of the revoked attestation.
    /// - `issuer` — address that performed the revocation.
    pub fn attestation_revoked(env: &Env, attestation_id: &String, issuer: &Address) {
        env.events().publish(
            (symbol_short!("revoked"), issuer.clone()),
            attestation_id.clone(),
        );
    }

    /// Emit event when an attestation is renewed
    pub fn attestation_renewed(env: &Env, attestation_id: &String, issuer: &Address, new_expiration: Option<u64>) {
        env.events().publish(
            (symbol_short!("renewed"), issuer.clone()),
            (attestation_id.clone(), new_expiration),
    /// Emit an event when an expired attestation is encountered during a check.
    ///
    /// This event is **not** emitted for revoked attestations; revocation takes
    /// precedence over expiration in [`crate::types::Attestation::get_status`].
    ///
    /// # Event schema
    /// ```text
    /// topics: ("expired", subject: Address)
    /// data:   attestation_id: String
    /// ```
    ///
    /// # Parameters
    /// - `attestation_id` — ID of the expired attestation.
    /// - `subject` — address the attestation was issued about.
    pub fn attestation_expired(env: &Env, attestation_id: &String, subject: &Address) {
        env.events().publish(
            (symbol_short!("expired"), subject.clone()),
            attestation_id.clone(),
        );
    }

    /// Emit an event when an attestation's expiration is updated.
    ///
    /// # Event schema
    /// ```text
    /// topics: ("updated", issuer: Address)
    /// data:   (attestation_id: String, new_expiration: Option<u64>)
    /// ```
    ///
    /// # Parameters
    /// - `attestation_id` — ID of the updated attestation.
    /// - `issuer` — address that performed the update.
    /// - `new_expiration` — the new expiration value (None means no expiration).
    pub fn attestation_updated(env: &Env, attestation_id: &String, issuer: &Address, new_expiration: Option<u64>) {
        env.events().publish(
            (symbol_short!("updated"), issuer.clone()),
            (attestation_id.clone(), new_expiration),
        );
    }

    /// Emit event when an issuer is registered
    pub fn issuer_registered(env: &Env, issuer: &Address, admin: &Address) {
        env.events().publish(
            (symbol_short!("iss_reg"), issuer.clone()),
            admin.clone(),
        );
    }

    /// Emit event when an issuer is removed
    pub fn issuer_removed(env: &Env, issuer: &Address, admin: &Address) {
        env.events().publish(
            (symbol_short!("iss_rem"), issuer.clone()),
            admin.clone(),
        );
    }
}
