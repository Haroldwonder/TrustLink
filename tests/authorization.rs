//! Authorization boundary tests for TrustLink (#71).
//!
//! Verifies that every privileged operation rejects callers that lack the
//! required role, and that failed calls leave global state unchanged.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use trustlink::{types::Error, TrustLinkContract, TrustLinkContractClient};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup(env: &Env) -> (Address, Address, Address, TrustLinkContractClient<'_>) {
    let id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(env, &id);
    let admin = Address::generate(env);
    let issuer = Address::generate(env);
    let unauth = Address::generate(env);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);
    (admin, issuer, unauth, client)
}

fn kyc(env: &Env) -> String {
    String::from_str(env, "KYC_PASSED")
}

fn claim(env: &Env, id: u32) -> String {
    String::from_str(env, &format!("CLAIM_{}", id))
}

#[test]
fn test_global_stats_reflects_mixed_operations() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &None);

    let total_issuers = 3u32;
    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);
    let issuer_c = Address::generate(&env);

    client.register_issuer(&admin, &issuer_a);
    client.register_issuer(&admin, &issuer_b);
    client.register_issuer(&admin, &issuer_c);

    let subject = Address::generate(&env);
    let total_attestations = 5u32;
    let total_revocations = 2u32;

    let mut attestation_ids: Vec<String> = Vec::new(&env);
    for i in 0..total_attestations {
        let claim_type = claim(&env, i);
        let id = client.create_attestation(
            &issuer_a,
            &subject,
            &claim_type,
            &None,
            &None,
            &None,
        );
        attestation_ids.push_back(id);
    }

    for i in 0..total_revocations {
        client.revoke_attestation(
            &issuer_a,
            &attestation_ids.get(i as u32).unwrap(),
            &None,
        );
    }

    let stats = client.get_global_stats();
    assert_eq!(stats.total_attestations, u64::from(total_attestations));
    assert_eq!(stats.total_revocations, u64::from(total_revocations));
    assert_eq!(stats.total_issuers, u64::from(total_issuers));
}

// ---------------------------------------------------------------------------
// 1. Unauthorized admin actions
// ---------------------------------------------------------------------------

#[test]
fn test_unauth_cannot_register_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, _, unauth, client) = setup(&env);
    let new_issuer = Address::generate(&env);

    let stats_before = client.get_global_stats();
    assert_eq!(
        client.try_register_issuer(&unauth, &new_issuer),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(
        client.get_global_stats().total_issuers,
        stats_before.total_issuers
    );
}

#[test]
fn test_unauth_cannot_remove_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, unauth, client) = setup(&env);

    let stats_before = client.get_global_stats();
    assert_eq!(
        client.try_remove_issuer(&unauth, &issuer),
        Err(Ok(Error::Unauthorized))
    );
    assert!(client.is_issuer(&issuer));
    assert_eq!(
        client.get_global_stats().total_issuers,
        stats_before.total_issuers
    );
}

#[test]
fn test_unauth_cannot_remove_bridge() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _, unauth, client) = setup(&env);
    let bridge = Address::generate(&env);
    client.register_bridge(&admin, &bridge);

    let bridges_before = client.get_bridges();
    assert_eq!(
        client.try_remove_bridge(&unauth, &bridge),
        Err(Ok(Error::Unauthorized))
    );
    // The bridge must still be registered after the rejected call.
    assert_eq!(client.get_bridges(), bridges_before);
}

#[test]
fn test_remove_bridge_fails_when_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _, _, client) = setup(&env);
    let bridge = Address::generate(&env);
    client.register_bridge(&admin, &bridge);
    client.pause(&admin);

    let bridges_before = client.get_bridges();
    assert_eq!(
        client.try_remove_bridge(&admin, &bridge),
        Err(Ok(Error::ContractPaused))
    );
    assert_eq!(client.get_bridges(), bridges_before);
}

#[test]
fn test_register_bridge_fails_when_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _, _, client) = setup(&env);
    let bridge = Address::generate(&env);
    client.pause(&admin);

    let bridges_before = client.get_bridges();
    assert_eq!(
        client.try_register_bridge(&admin, &bridge),
        Err(Ok(Error::ContractPaused))
    );
    assert_eq!(client.get_bridges(), bridges_before);
}

// ---------------------------------------------------------------------------
// 2. Unauthorized issuance actions
// ---------------------------------------------------------------------------

#[test]
fn test_unauth_cannot_create_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, _, unauth, client) = setup(&env);
    let subject = Address::generate(&env);

    let stats_before = client.get_global_stats();
    assert_eq!(
        client.try_create_attestation(&unauth, &subject, &kyc(&env), &None, &None, &None),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(
        client.get_global_stats().total_attestations,
        stats_before.total_attestations
    );
}

#[test]
fn test_admin_cannot_create_attestation_when_not_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _, _, client) = setup(&env);
    let subject = Address::generate(&env);

    // Admin is not a registered issuer — must be rejected.
    let stats_before = client.get_global_stats();
    assert_eq!(
        client.try_create_attestation(&admin, &subject, &kyc(&env), &None, &None, &None),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(
        client.get_global_stats().total_attestations,
        stats_before.total_attestations
    );
}

// ---------------------------------------------------------------------------
// 3. Cross-issuer revocation
// ---------------------------------------------------------------------------

#[test]
fn test_issuer_b_cannot_revoke_issuer_a_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer_a, _, client) = setup(&env);
    let issuer_b = Address::generate(&env);
    let subject = Address::generate(&env);
    client.register_issuer(&admin, &issuer_b);

    let attestation_id =
        client.create_attestation(&issuer_a, &subject, &kyc(&env), &None, &None, &None);

    let stats_before = client.get_global_stats();
    assert_eq!(
        client.try_revoke_attestation(&issuer_b, &attestation_id, &None),
        Err(Ok(Error::Unauthorized))
    );
    assert!(!client.get_attestation(&attestation_id).revoked);
    assert_eq!(
        client.get_global_stats().total_revocations,
        stats_before.total_revocations
    );
}

// ---------------------------------------------------------------------------
// 3b. Removed issuers retain revocation rights only
// ---------------------------------------------------------------------------

#[test]
fn test_removed_issuer_can_revoke_own_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, _, client) = setup(&env);
    let subject = Address::generate(&env);

    let attestation_id =
        client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);

    client.remove_issuer(&admin, &issuer);
    assert!(!client.is_issuer(&issuer));

    client.revoke_attestation(&issuer, &attestation_id, &None);
    assert!(client.get_attestation(&attestation_id).revoked);
}

#[test]
fn test_removed_issuer_cannot_create_renew_or_amend() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, _, client) = setup(&env);
    let subject = Address::generate(&env);

    let attestation_id =
        client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);

    client.remove_issuer(&admin, &issuer);
    assert!(!client.is_issuer(&issuer));

    // Cannot create new attestations.
    assert_eq!(
        client.try_create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None),
        Err(Ok(Error::Unauthorized))
    );

    // Cannot renew existing attestations.
    assert_eq!(
        client.try_renew_attestation(&issuer, &attestation_id, &None),
        Err(Ok(Error::Unauthorized))
    );

    // Cannot amend expiration to extend validity.
    assert_eq!(
        client.try_update_expiration(&issuer, &attestation_id, &u64::MAX),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn test_non_owner_cannot_revoke_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer_a, _, client) = setup(&env);
    let issuer_b = Address::generate(&env);
    let subject = Address::generate(&env);
    client.register_issuer(&admin, &issuer_b);

    let attestation_id =
        client.create_attestation(&issuer_a, &subject, &kyc(&env), &None, &None, &None);

    // A registered issuer that does not own the attestation cannot revoke it.
    assert_eq!(
        client.try_revoke_attestation(&issuer_b, &attestation_id, &None),
        Err(Ok(Error::Unauthorized))
    );
    assert!(!client.get_attestation(&attestation_id).revoked);

    // A removed issuer likewise cannot revoke attestations it did not issue.
    client.remove_issuer(&admin, &issuer_b);
    assert_eq!(
        client.try_revoke_attestation(&issuer_b, &attestation_id, &None),
        Err(Ok(Error::Unauthorized))
    );
    assert!(!client.get_attestation(&attestation_id).revoked);
}

// ---------------------------------------------------------------------------
// 4. Unauthorized import (admin-only)
// ---------------------------------------------------------------------------

#[test]
fn test_unauth_cannot_import_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, unauth, client) = setup(&env);
    let subject = Address::generate(&env);

    let stats_before = client.get_global_stats();
    assert_eq!(
        client.try_import_attestation(&unauth, &issuer, &subject, &kyc(&env), &1_000_000, &None),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(
        client.get_global_stats().total_attestations,
        stats_before.total_attestations
    );
}
