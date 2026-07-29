//! Tests for the simulate_create_attestation function.
//!
//! Verifies that the simulate function returns the same attestation ID and fee
//! that would result from calling create_attestation with the same arguments,
//! without committing any state to the ledger.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use trustlink::{types::Error, TrustLinkContract, TrustLinkContractClient};

fn setup(env: &Env) -> (Address, Address, Address, TrustLinkContractClient<'_>) {
    let id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(env, &id);
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

#[test]
fn test_simulate_returns_same_id_as_real_call() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);
    let expiration = None;
    let metadata = None;
    let tags = None;

    // Get the simulated result
    let (simulated_id, simulated_fee) = client
        .simulate_create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags)
        .unwrap();

    // Now actually create the attestation
    let real_id = client.create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags);

    // Verify they match
    assert_eq!(simulated_id, real_id);
    // Fee should be 0 when not configured
    assert_eq!(simulated_fee, 0);
}

#[test]
fn test_simulate_detects_duplicate() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);
    let expiration = None;
    let metadata = None;
    let tags = None;

    // First call succeeds
    let id1 = client.create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags);

    // Second call (simulate) should fail with DuplicateAttestation
    let result = client.simulate_create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags);
    assert_eq!(result, Err(Ok(Error::DuplicateAttestation)));

    // Third call (simulate) with different claim type should succeed
    let claim_type2 = String::from_str(&env, "CLAIM_2");
    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &claim_type2, &expiration, &metadata, &tags)
        .unwrap();

    // Actually create it
    let real_id = client.create_attestation(&issuer, &subject, &claim_type2, &expiration, &metadata, &tags);
    assert_eq!(simulated_id, real_id);
}

#[test]
fn test_simulate_detects_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _, subject, client) = setup(&env);

    let claim_type = kyc(&env);
    let expiration = None;
    let metadata = None;
    let tags = None;

    // Admin is not a registered issuer
    let result = client.simulate_create_attestation(&admin, &subject, &claim_type, &expiration, &metadata, &tags);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_simulate_detects_rate_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    // Set a rate limit of 1 second
    client.set_rate_limit(&issuer, &1);

    let claim_type = kyc(&env);
    let expiration = None;
    let metadata = None;
    let tags = None;

    // First call succeeds
    client.create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags);

    // Second call (simulate) should fail with RateLimited
    let result = client.simulate_create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags);
    assert_eq!(result, Err(Ok(Error::RateLimited)));
}

#[test]
fn test_simulate_detects_limit_exceeded() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    // Get current limits
    let limits = client.get_limits();

    // Create attestations to hit the issuer limit
    let claim_type = kyc(&env);
    let expiration = None;
    let metadata = None;
    let tags = None;

    for i in 0..limits.max_attestations_per_issuer {
        let claim = String::from_str(&env, &format!("CLAIM_{}", i));
        client.create_attestation(&issuer, &subject, &claim, &expiration, &metadata, &tags);
    }

    // Next simulate should fail with LimitExceeded
    let result = client.simulate_create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags);
    assert_eq!(result, Err(Ok(Error::LimitExceeded)));
}

#[test]
fn test_simulate_with_metadata() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);
    let expiration = None;
    let metadata = Some(String::from_str(&env, "test_metadata"));
    let tags = Some(Vec::new(&env));

    // Get the simulated result
    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags)
        .unwrap();

    // Now actually create the attestation
    let real_id = client.create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags);

    // Verify they match
    assert_eq!(simulated_id, real_id);
}

#[test]
fn test_simulate_with_expiration() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);
    let expiration = Some(1_000_000);
    let metadata = None;
    let tags = None;

    // Get the simulated result
    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags)
        .unwrap();

    // Now actually create the attestation
    let real_id = client.create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags);

    // Verify they match
    assert_eq!(simulated_id, real_id);
}

#[test]
fn test_simulate_with_tags() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);
    let expiration = None;
    let metadata = None;

    let mut tags = Vec::new(&env);
    tags.push_back(String::from_str(&env, "tag1"));
    tags.push_back(String::from_str(&env, "tag2"));

    // Get the simulated result
    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags)
        .unwrap();

    // Now actually create the attestation
    let real_id = client.create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags);

    // Verify they match
    assert_eq!(simulated_id, real_id);
}

#[test]
fn test_simulate_fee_calculation() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, subject, client) = setup(&env);

    // Configure a fee
    let fee_collector = Address::generate(&env);
    let fee_token = env.register_contract_wasm(None, trustlink::CONTRACT_WASM);
    client.set_fee(&admin, &100_000, &fee_collector, &Some(fee_token));

    let claim_type = kyc(&env);
    let expiration = None;
    let metadata = None;
    let tags = None;

    // Simulate should return the fee amount
    let (_simulated_id, simulated_fee) = client
        .simulate_create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags)
        .unwrap();

    assert_eq!(simulated_fee, 100_000);
}

#[test]
fn test_simulate_does_not_commit_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);
    let expiration = None;
    let metadata = None;
    let tags = None;

    // Simulate should succeed
    let (simulated_id, _) = client
        .simulate_create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags)
        .unwrap();

    // But no attestation should exist
    let stats = client.get_global_stats();
    assert_eq!(stats.total_attestations, 0);

    // Actually create it
    let real_id = client.create_attestation(&issuer, &subject, &claim_type, &expiration, &metadata, &tags);

    // Verify it was created
    let stats_after = client.get_global_stats();
    assert_eq!(stats_after.total_attestations, 1);

    // Verify the IDs match
    assert_eq!(simulated_id, real_id);
}
