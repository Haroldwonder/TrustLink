//! Tests for the simulate_create_attestation function.
//!
//! Verifies that the simulate function:
//!   - Returns the same attestation ID and fee that create_attestation would produce
//!   - Detects every error condition that create_attestation would reject
//!   - Never writes any state to the ledger

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use trustlink::{types::Error, TrustLinkContract, TrustLinkContractClient};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup(env: &Env) -> (Address, Address, Address, TrustLinkContractClient<'_>) {
    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let issuer = Address::generate(env);
    let subject = Address::generate(env);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);
    (admin, issuer, subject, client)
}

fn kyc(env: &Env) -> String {
    String::from_str(env, "KYC_PASSED")
}

fn register_test_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract_v2(admin.clone())
        .address()
}

// ---------------------------------------------------------------------------
// Happy path: returned ID matches what create_attestation produces
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_returns_same_id_as_real_call() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let (simulated_id, simulated_fee) = client
        .simulate_create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None)
        .unwrap();

    let real_id = client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);

    assert_eq!(simulated_id, real_id);
    // No fee configured → fee must be 0
    assert_eq!(simulated_fee, 0);
}

// ---------------------------------------------------------------------------
// Simulate does not commit any state
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_does_not_commit_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None)
        .unwrap();

    // Global stats must still be zero — nothing was written
    assert_eq!(client.get_global_stats().total_attestations, 0);

    // Actually creating it now must succeed and produce the same ID
    let real_id = client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);
    assert_eq!(simulated_id, real_id);
    assert_eq!(client.get_global_stats().total_attestations, 1);
}

// ---------------------------------------------------------------------------
// Error: caller is not a registered issuer
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_unregistered_caller_returns_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _, subject, client) = setup(&env);

    // admin was never registered as an issuer
    let result = client.simulate_create_attestation(
        &admin, &subject, &kyc(&env), &None, &None, &None,
    );
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

// ---------------------------------------------------------------------------
// Error: issuer == subject
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_issuer_equals_subject_returns_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, _, client) = setup(&env);

    // Pass the issuer address as both issuer and subject
    let result = client.simulate_create_attestation(
        &issuer, &issuer, &kyc(&env), &None, &None, &None,
    );
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

// ---------------------------------------------------------------------------
// Error: contract is paused
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_paused_contract_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, subject, client) = setup(&env);

    client.pause(&admin);

    let result = client.simulate_create_attestation(
        &issuer, &subject, &kyc(&env), &None, &None, &None,
    );
    // Any error is acceptable; the important thing is that it does not succeed
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Error: whitelist mode active and subject is not on the list
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_subject_not_whitelisted_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    // Enable whitelist mode without adding subject
    client.enable_whitelist_mode(&issuer);

    let result = client.simulate_create_attestation(
        &issuer, &subject, &kyc(&env), &None, &None, &None,
    );
    assert_eq!(result, Err(Ok(Error::SubjectNotWhitelisted)));
}

#[test]
fn test_simulate_whitelisted_subject_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    client.enable_whitelist_mode(&issuer);
    client.add_to_whitelist(&issuer, &subject);

    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None)
        .unwrap();

    let real_id = client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);
    assert_eq!(simulated_id, real_id);
}

// ---------------------------------------------------------------------------
// Error: rate limited
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_rate_limited_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, subject, client) = setup(&env);

    // Set a global rate limit of 1000 seconds
    client.set_rate_limit(&admin, &1_000);

    env.ledger().with_mut(|l| l.timestamp = 1_000);
    client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);

    // Same ledger timestamp — still within the rate-limit window
    let result = client.simulate_create_attestation(
        &issuer, &subject, &kyc(&env), &None, &None, &None,
    );
    assert_eq!(result, Err(Ok(Error::RateLimited)));
}

#[test]
fn test_simulate_after_rate_limit_window_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, subject, client) = setup(&env);

    client.set_rate_limit(&admin, &1_000);

    env.ledger().with_mut(|l| l.timestamp = 1_000);
    client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);

    // Advance past the rate-limit window
    env.ledger().with_mut(|l| l.timestamp = 2_001);
    let claim2 = String::from_str(&env, "AML_CHECKED");
    let result = client.simulate_create_attestation(
        &issuer, &subject, &claim2, &None, &None, &None,
    );
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// Error: issuer attestation count at limit
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_issuer_limit_exceeded_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, subject, client) = setup(&env);

    // Cap the issuer at 2 attestations
    client.set_limits(&admin, &2, &10_000);

    env.ledger().with_mut(|l| l.timestamp = 1_000);
    client.create_attestation(
        &issuer, &subject, &String::from_str(&env, "CLAIM_1"), &None, &None, &None,
    );
    env.ledger().with_mut(|l| l.timestamp = 2_000);
    client.create_attestation(
        &issuer, &subject, &String::from_str(&env, "CLAIM_2"), &None, &None, &None,
    );

    // Third attempt should fail
    let result = client.simulate_create_attestation(
        &issuer, &subject, &kyc(&env), &None, &None, &None,
    );
    assert_eq!(result, Err(Ok(Error::LimitExceeded)));
}

// ---------------------------------------------------------------------------
// Error: subject attestation count at limit
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_subject_limit_exceeded_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, subject, client) = setup(&env);

    // Cap the subject at 1 attestation
    client.set_limits(&admin, &10_000, &1);

    client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);

    let result = client.simulate_create_attestation(
        &issuer, &subject, &String::from_str(&env, "AML_CHECKED"), &None, &None, &None,
    );
    assert_eq!(result, Err(Ok(Error::LimitExceeded)));
}

// ---------------------------------------------------------------------------
// Error: attestation already exists at the same timestamp (duplicate)
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_duplicate_attestation_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    // Create a real attestation first
    client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);

    // Simulate at the same timestamp → same ID would be generated → duplicate
    let result = client.simulate_create_attestation(
        &issuer, &subject, &kyc(&env), &None, &None, &None,
    );
    assert_eq!(result, Err(Ok(Error::DuplicateAttestation)));
}

#[test]
fn test_simulate_different_claim_type_after_first_creation_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);

    // Different claim type at a later timestamp — must not collide
    env.ledger().with_mut(|l| l.timestamp = l.timestamp + 1);
    let aml = String::from_str(&env, "AML_CHECKED");
    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &aml, &None, &None, &None)
        .unwrap();

    let real_id = client.create_attestation(&issuer, &subject, &aml, &None, &None, &None);
    assert_eq!(simulated_id, real_id);
}

// ---------------------------------------------------------------------------
// Fee: simulate returns the configured fee amount without charging it
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_returns_configured_fee() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, subject, client) = setup(&env);

    let fee_collector = Address::generate(&env);
    let fee_token = register_test_token(&env, &admin);
    client.set_fee(&admin, &100_000, &fee_collector, &Some(fee_token));

    let (_, simulated_fee) = client
        .simulate_create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None)
        .unwrap();

    assert_eq!(simulated_fee, 100_000);
    // Still no attestation was written
    assert_eq!(client.get_global_stats().total_attestations, 0);
}

#[test]
fn test_simulate_zero_fee_when_no_fee_configured() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let (_, simulated_fee) = client
        .simulate_create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None)
        .unwrap();

    assert_eq!(simulated_fee, 0);
}

// ---------------------------------------------------------------------------
// ID consistency across optional parameter combinations
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_with_expiration_matches_real_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let expiration = Some(9_999_999_u64);
    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &kyc(&env), &expiration, &None, &None)
        .unwrap();

    let real_id = client.create_attestation(
        &issuer, &subject, &kyc(&env), &expiration, &None, &None,
    );
    assert_eq!(simulated_id, real_id);
}

#[test]
fn test_simulate_with_metadata_matches_real_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let metadata = Some(String::from_str(&env, "extra_info"));
    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &kyc(&env), &None, &metadata, &None)
        .unwrap();

    let real_id = client.create_attestation(
        &issuer, &subject, &kyc(&env), &None, &metadata, &None,
    );
    assert_eq!(simulated_id, real_id);
}

#[test]
fn test_simulate_with_tags_matches_real_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let mut tags: Vec<String> = Vec::new(&env);
    tags.push_back(String::from_str(&env, "verified"));
    tags.push_back(String::from_str(&env, "tier1"));

    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &Some(tags.clone()))
        .unwrap();

    let real_id = client.create_attestation(
        &issuer, &subject, &kyc(&env), &None, &None, &Some(tags),
    );
    assert_eq!(simulated_id, real_id);
}

#[test]
fn test_simulate_all_optional_params_matches_real_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let expiration = Some(5_000_000_u64);
    let metadata = Some(String::from_str(&env, "full_param_test"));
    let mut tags: Vec<String> = Vec::new(&env);
    tags.push_back(String::from_str(&env, "tag_a"));

    let (simulated_id, _) = client
        .simulate_create_attestation(
            &issuer, &subject, &kyc(&env),
            &expiration, &metadata, &Some(tags.clone()),
        )
        .unwrap();

    let real_id = client.create_attestation(
        &issuer, &subject, &kyc(&env),
        &expiration, &metadata, &Some(tags),
    );
    assert_eq!(simulated_id, real_id);
}

// ---------------------------------------------------------------------------
// Multiple sequential simulates are independent (no cross-contamination)
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_multiple_calls_return_distinct_results_for_different_subjects() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, _, client) = setup(&env);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);

    let (id1, _) = client
        .simulate_create_attestation(&issuer, &s1, &kyc(&env), &None, &None, &None)
        .unwrap();

    let (id2, _) = client
        .simulate_create_attestation(&issuer, &s2, &kyc(&env), &None, &None, &None)
        .unwrap();

    // IDs must differ because the subject is part of the ID derivation
    assert_ne!(id1, id2);
    // Neither call should have written state
    assert_eq!(client.get_global_stats().total_attestations, 0);
}
