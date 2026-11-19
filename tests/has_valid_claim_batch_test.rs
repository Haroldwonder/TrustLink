//! Tests for the has_valid_claim_batch function.
//!
//! Verifies that the batch query returns a positionally-aligned Vec<bool>
//! matching the input `subjects` slice, delegating each element to the
//! same logic as `has_valid_claim`.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use trustlink::{TrustLinkContract, TrustLinkContractClient};

fn setup(env: &Env) -> (Address, Address, TrustLinkContractClient<'_>) {
    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let issuer = Address::generate(env);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);
    (admin, issuer, client)
}

fn kyc(env: &Env) -> String {
    String::from_str(env, "KYC_PASSED")
}

fn aml(env: &Env) -> String {
    String::from_str(env, "AML_CHECKED")
}

// ---------------------------------------------------------------------------
// Empty input
// ---------------------------------------------------------------------------

#[test]
fn test_batch_empty_subjects_returns_empty_vec() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, _, client) = setup(&env);

    let subjects: Vec<Address> = Vec::new(&env);
    let results = client.has_valid_claim_batch(&subjects, &kyc(&env));

    assert_eq!(results.len(), 0);
}

// ---------------------------------------------------------------------------
// All subjects have the claim
// ---------------------------------------------------------------------------

#[test]
fn test_batch_all_subjects_have_claim() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let claim = kyc(&env);
    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let s3 = Address::generate(&env);

    client.create_attestation(&issuer, &s1, &claim, &None, &None, &None);
    client.create_attestation(&issuer, &s2, &claim, &None, &None, &None);
    client.create_attestation(&issuer, &s3, &claim, &None, &None, &None);

    let mut subjects = Vec::new(&env);
    subjects.push_back(s1);
    subjects.push_back(s2);
    subjects.push_back(s3);

    let results = client.has_valid_claim_batch(&subjects, &claim);

    assert_eq!(results.len(), 3);
    assert!(results.get(0).unwrap());
    assert!(results.get(1).unwrap());
    assert!(results.get(2).unwrap());
}

// ---------------------------------------------------------------------------
// No subjects have the claim
// ---------------------------------------------------------------------------

#[test]
fn test_batch_no_subjects_have_claim() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, _, client) = setup(&env);

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);

    let mut subjects = Vec::new(&env);
    subjects.push_back(s1);
    subjects.push_back(s2);

    let results = client.has_valid_claim_batch(&subjects, &kyc(&env));

    assert_eq!(results.len(), 2);
    assert!(!results.get(0).unwrap());
    assert!(!results.get(1).unwrap());
}

// ---------------------------------------------------------------------------
// Mixed: some have the claim, some do not
// ---------------------------------------------------------------------------

#[test]
fn test_batch_mixed_results_preserve_order() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let claim = kyc(&env);
    let s_with = Address::generate(&env);
    let s_without = Address::generate(&env);

    // Only s_with gets the attestation
    client.create_attestation(&issuer, &s_with, &claim, &None, &None, &None);

    // Input order: [s_without, s_with, s_without] — result must mirror this
    let mut subjects = Vec::new(&env);
    subjects.push_back(s_without.clone());
    subjects.push_back(s_with.clone());
    subjects.push_back(s_without.clone());

    let results = client.has_valid_claim_batch(&subjects, &claim);

    assert_eq!(results.len(), 3);
    assert!(!results.get(0).unwrap()); // s_without → false
    assert!(results.get(1).unwrap());  // s_with    → true
    assert!(!results.get(2).unwrap()); // s_without → false
}

// ---------------------------------------------------------------------------
// Revoked attestation → false
// ---------------------------------------------------------------------------

#[test]
fn test_batch_revoked_attestation_returns_false() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let claim = kyc(&env);
    let subject = Address::generate(&env);

    let att_id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
    client.revoke_attestation(&issuer, &att_id, &None);

    let mut subjects = Vec::new(&env);
    subjects.push_back(subject);

    let results = client.has_valid_claim_batch(&subjects, &claim);

    assert_eq!(results.len(), 1);
    assert!(!results.get(0).unwrap());
}

// ---------------------------------------------------------------------------
// Expired attestation → false
// ---------------------------------------------------------------------------

#[test]
fn test_batch_expired_attestation_returns_false() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let claim = kyc(&env);
    let subject = Address::generate(&env);

    // Create with an expiration 100 seconds in the future
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let expiration: Option<u64> = Some(1100);
    client.create_attestation(&issuer, &subject, &claim, &expiration, &None, &None);

    // Advance past expiration
    env.ledger().with_mut(|l| l.timestamp = 1101);

    let mut subjects = Vec::new(&env);
    subjects.push_back(subject);

    let results = client.has_valid_claim_batch(&subjects, &claim);

    assert_eq!(results.len(), 1);
    assert!(!results.get(0).unwrap());
}

// ---------------------------------------------------------------------------
// Not-yet-expired attestation → true
// ---------------------------------------------------------------------------

#[test]
fn test_batch_valid_before_expiry_returns_true() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let claim = kyc(&env);
    let subject = Address::generate(&env);

    env.ledger().with_mut(|l| l.timestamp = 1000);
    let expiration: Option<u64> = Some(2000);
    client.create_attestation(&issuer, &subject, &claim, &expiration, &None, &None);

    // Still before expiry
    env.ledger().with_mut(|l| l.timestamp = 1500);

    let mut subjects = Vec::new(&env);
    subjects.push_back(subject);

    let results = client.has_valid_claim_batch(&subjects, &claim);

    assert_eq!(results.len(), 1);
    assert!(results.get(0).unwrap());
}

// ---------------------------------------------------------------------------
// Deleted attestation → false
// ---------------------------------------------------------------------------

#[test]
fn test_batch_deleted_attestation_returns_false() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let claim = kyc(&env);
    let subject = Address::generate(&env);

    let att_id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
    client.request_deletion(&subject, &att_id);

    let mut subjects = Vec::new(&env);
    subjects.push_back(subject);

    let results = client.has_valid_claim_batch(&subjects, &claim);

    assert_eq!(results.len(), 1);
    assert!(!results.get(0).unwrap());
}

// ---------------------------------------------------------------------------
// Only the queried claim type counts — other claims on the same subject
// must not produce a false positive
// ---------------------------------------------------------------------------

#[test]
fn test_batch_different_claim_type_does_not_match() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let subject = Address::generate(&env);

    // Subject has AML but not KYC
    client.create_attestation(&issuer, &subject, &aml(&env), &None, &None, &None);

    let mut subjects = Vec::new(&env);
    subjects.push_back(subject);

    // Query for KYC — should be false
    let results = client.has_valid_claim_batch(&subjects, &kyc(&env));

    assert_eq!(results.len(), 1);
    assert!(!results.get(0).unwrap());
}

#[test]
fn test_batch_subject_with_multiple_claim_types_matches_correct_one() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let subject = Address::generate(&env);

    // Subject has both KYC and AML
    client.create_attestation(&issuer, &subject, &kyc(&env), &None, &None, &None);
    client.create_attestation(&issuer, &subject, &aml(&env), &None, &None, &None);

    let mut subjects = Vec::new(&env);
    subjects.push_back(subject.clone());

    // Query for KYC → true
    let kyc_results = client.has_valid_claim_batch(&subjects, &kyc(&env));
    assert!(kyc_results.get(0).unwrap());

    // Query for AML → true
    let aml_results = client.has_valid_claim_batch(&subjects, &aml(&env));
    assert!(aml_results.get(0).unwrap());

    // Query for a claim the subject does not hold → false
    let other = String::from_str(&env, "ACCREDITED_INVESTOR");
    let other_results = client.has_valid_claim_batch(&subjects, &other);
    assert!(!other_results.get(0).unwrap());
}

// ---------------------------------------------------------------------------
// Multiple issuers — any valid attestation from any issuer satisfies the check
// ---------------------------------------------------------------------------

#[test]
fn test_batch_attestation_from_any_issuer_is_valid() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);
    let subject = Address::generate(&env);

    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer_a);
    client.register_issuer(&admin, &issuer_b);

    let claim = kyc(&env);

    // Only issuer_b attests this subject
    client.create_attestation(&issuer_b, &subject, &claim, &None, &None, &None);

    let mut subjects = Vec::new(&env);
    subjects.push_back(subject);

    let results = client.has_valid_claim_batch(&subjects, &claim);
    assert!(results.get(0).unwrap());
}

// ---------------------------------------------------------------------------
// Single subject — result length is 1
// ---------------------------------------------------------------------------

#[test]
fn test_batch_single_subject_returns_single_element() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let claim = kyc(&env);
    let subject = Address::generate(&env);
    client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

    let mut subjects = Vec::new(&env);
    subjects.push_back(subject);

    let results = client.has_valid_claim_batch(&subjects, &claim);

    assert_eq!(results.len(), 1);
    assert!(results.get(0).unwrap());
}

// ---------------------------------------------------------------------------
// Result length always equals input length
// ---------------------------------------------------------------------------

#[test]
fn test_batch_result_length_matches_input_length() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let claim = kyc(&env);

    // Create 5 subjects, attest only 3 of them
    let subjects_vec: std::vec::Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();

    for s in &subjects_vec[0..3] {
        client.create_attestation(&issuer, s, &claim, &None, &None, &None);
    }

    let mut subjects = Vec::new(&env);
    for s in &subjects_vec {
        subjects.push_back(s.clone());
    }

    let results = client.has_valid_claim_batch(&subjects, &claim);

    assert_eq!(results.len(), 5);
    // First 3 have the claim, last 2 do not
    assert!(results.get(0).unwrap());
    assert!(results.get(1).unwrap());
    assert!(results.get(2).unwrap());
    assert!(!results.get(3).unwrap());
    assert!(!results.get(4).unwrap());
}

// ---------------------------------------------------------------------------
// Duplicate subjects in input — each position evaluated independently
// ---------------------------------------------------------------------------

#[test]
fn test_batch_duplicate_subjects_in_input() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let claim = kyc(&env);
    let subject = Address::generate(&env);
    client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

    // Same subject appears twice
    let mut subjects = Vec::new(&env);
    subjects.push_back(subject.clone());
    subjects.push_back(subject.clone());

    let results = client.has_valid_claim_batch(&subjects, &claim);

    assert_eq!(results.len(), 2);
    assert!(results.get(0).unwrap());
    assert!(results.get(1).unwrap());
}
