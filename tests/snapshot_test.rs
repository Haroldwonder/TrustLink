//! Snapshot tests for key contract state transitions.
//!
//! These tests exist specifically to capture and protect the exact ledger state
//! and events produced after each critical operation. The soroban-sdk writes a
//! JSON snapshot to `test_snapshots/tests/` at the end of every test.
//!
//! If a snapshot file changes unexpectedly in CI, it means contract behaviour
//! or storage layout has changed — review the diff before merging.
//!
//! # Updating snapshots
//! See `docs/snapshot-testing.md` for the update process.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};
use trustlink::{TrustLinkContract, TrustLinkContractClient};

fn deploy(env: &Env) -> TrustLinkContractClient {
    let id = env.register_contract(None, TrustLinkContract);
    TrustLinkContractClient::new(env, &id)
}

// ── 1. Initialization ────────────────────────────────────────────────────────

/// Snapshot: contract state immediately after `initialize`.
/// Captures: Admin, FeeConfig, TtlConfig, Version in instance storage.
#[test]
fn snapshot_after_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let client = deploy(&env);
    let admin = Address::generate(&env);

    client.initialize(&admin, &None);

    // Verify the state we want snapshotted is correct.
    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_version(), String::from_str(&env, "1.0.0"));
    let fee = client.get_fee_config();
    assert_eq!(fee.attestation_fee, 0);
    assert_eq!(fee.fee_token, None);
}

// ── 2. Issuer registration ───────────────────────────────────────────────────

/// Snapshot: contract state after registering an issuer.
/// Captures: Issuer key, GlobalStats.total_issuers, iss_reg event.
#[test]
fn snapshot_after_issuer_registration() {
    let env = Env::default();
    env.mock_all_auths();

    let client = deploy(&env);
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);

    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);

    assert!(client.is_issuer(&issuer));
    assert_eq!(client.get_global_stats().total_issuers, 1);
}

// ── 3. Attestation creation ──────────────────────────────────────────────────

/// Snapshot: contract state after creating a single attestation.
/// Captures: Attestation record, SubjectAttestations index, IssuerAttestations
/// index, IssuerStats, GlobalStats.total_attestations, AuditLog, `created` event.
#[test]
fn snapshot_after_attestation_creation() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000);

    let client = deploy(&env);
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    let att = client.get_attestation(&id);
    assert_eq!(att.issuer, issuer);
    assert_eq!(att.subject, subject);
    assert!(!att.revoked);
    assert!(!att.imported);
    assert_eq!(client.get_global_stats().total_attestations, 1);
    assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 1);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 1);
    assert_eq!(client.get_audit_log(&id).len(), 1);
}

// ── 4. Revocation ────────────────────────────────────────────────────────────

/// Snapshot: contract state after revoking an attestation.
/// Captures: Attestation.revoked=true, AuditLog with Revoked entry,
/// GlobalStats.total_revocations, `revoked` event.
#[test]
fn snapshot_after_revocation() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000);

    let client = deploy(&env);
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let reason = Some(String::from_str(&env, "fraud detected"));

    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id, &reason);

    let att = client.get_attestation(&id);
    assert!(att.revoked);
    assert_eq!(att.revocation_reason, reason);
    assert!(!client.has_valid_claim(&subject, &claim_type));
    assert_eq!(client.get_global_stats().total_revocations, 1);
    assert_eq!(client.get_audit_log(&id).len(), 2); // Created + Revoked
}
