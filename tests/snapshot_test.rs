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
use trustlink::types::AttestationOrigin;

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
    assert_eq!(att.origin, trustlink::types::AttestationOrigin::Native);
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
// ── 5. Bridge attestation ────────────────────────────────────────────────────

/// Snapshot: contract state after bridging an attestation from another chain.
/// Captures: Attestation record with source_chain/source_tx, bridged=true,
/// GlobalStats.total_attestations, `bridged` event.
#[test]
fn snapshot_after_bridge_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000);

    let client = deploy(&env);
    let admin = Address::generate(&env);
    let bridge = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let source_chain = String::from_str(&env, "ethereum");
    let source_tx = String::from_str(&env, "0x123abc");

    client.initialize(&admin, &None);
    client.register_bridge(&admin, &bridge);

    let id = client.bridge_attestation(&bridge, &subject, &claim_type, &source_chain, &source_tx);

    let att = client.get_attestation(&id);
    assert_eq!(att.issuer, bridge);
    assert_eq!(att.subject, subject);
    assert_eq!(att.origin, AttestationOrigin::Bridged);
    assert_eq!(att.source_chain, Some(source_chain));
    assert_eq!(att.source_tx, Some(source_tx));
    assert_eq!(client.get_global_stats().total_attestations, 1);
}

// ── 6. Transfer attestation ──────────────────────────────────────────────────

/// Snapshot: contract state after transferring an attestation to a new issuer.
/// Captures: Updated attestation.issuer, issuer stats changes, audit log entry,
/// `att_xfer` event.
#[test]
fn snapshot_after_transfer_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000);

    let client = deploy(&env);
    let admin = Address::generate(&env);
    let old_issuer = Address::generate(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    client.initialize(&admin, &None);
    client.register_issuer(&admin, &old_issuer);
    client.register_issuer(&admin, &new_issuer);

    let id = client.create_attestation(&old_issuer, &subject, &claim_type, &None, &None, &None);
    client.transfer_attestation(&admin, &id, &new_issuer);

    let att = client.get_attestation(&id);
    assert_eq!(att.issuer, new_issuer);
    assert_eq!(client.get_issuer_stats(&old_issuer).total_issued, 0);
    assert_eq!(client.get_issuer_stats(&new_issuer).total_issued, 1);
    assert_eq!(client.get_audit_log(&id).len(), 2); // Created + Transferred
}

// ── 7. Contract pause/unpause ────────────────────────────────────────────────

/// Snapshot: contract state after pausing the contract.
/// Captures: Paused flag, `paused` event.
#[test]
fn snapshot_after_contract_pause() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000);

    let client = deploy(&env);
    let admin = Address::generate(&env);

    client.initialize(&admin, &None);
    client.pause(&admin);

    assert!(client.is_paused());
}

/// Snapshot: contract state after unpausing the contract.
/// Captures: Paused flag reset, `unpaused` event.
#[test]
fn snapshot_after_contract_unpause() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000);

    let client = deploy(&env);
    let admin = Address::generate(&env);

    client.initialize(&admin, &None);
    client.pause(&admin);
    client.unpause(&admin);

    assert!(!client.is_paused());
}

// ── 8. Expiration hook triggered ─────────────────────────────────────────────

/// Snapshot: contract state when an expiration hook is triggered.
/// Captures: ExpirationHook registration, `exp_hook` event fired during has_valid_claim.
#[test]
fn snapshot_after_expiration_hook_triggered() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000);

    let client = deploy(&env);
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let callback_contract = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);

    // Create an attestation that expires in 10 days
    let expiration = 1_000 + (10 * 86_400);
    let id = client.create_attestation(&issuer, &subject, &claim_type, &Some(expiration), &None, &None);

    // Register expiration hook to notify 7 days before expiry
    client.register_expiration_hook(&subject, &callback_contract, &7);

    // Move time to 4 days before expiry (within notification window)
    env.ledger().with_mut(|l| l.timestamp = expiration - (4 * 86_400));

    // This should trigger the expiration hook
    let has_claim = client.has_valid_claim(&subject, &claim_type);
    assert!(has_claim); // Still valid, but hook should fire

    let hook = client.get_expiration_hook(&subject);
    assert!(hook.is_some());
    let hook = hook.unwrap();
    assert_eq!(hook.callback_contract, callback_contract);
    assert_eq!(hook.notify_days_before, 7);
}