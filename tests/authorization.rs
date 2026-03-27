//! Authorization boundary tests for TrustLink (#71).
//!
//! Verifies that every privileged operation rejects callers that lack the
//! required role, and that failed calls leave global state unchanged.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String};
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
