#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Events as _, Ledger}, Address, Env, String};
use soroban_sdk::{testutils::{Address as _, Events as _, Ledger}, Address, BytesN, Env, String};

fn create_test_contract(env: &Env) -> (Address, TrustLinkContractClient) {
    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(env, &contract_id);
    (contract_id, client)
}

#[test]
fn test_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    
    client.initialize(&admin);
    
    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_double_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    
    client.initialize(&admin);
    client.initialize(&admin); // Should panic
}

#[test]
fn test_register_and_check_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    
    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);
    
    assert!(client.is_issuer(&issuer));
}

#[test]
fn test_remove_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    
    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);
    assert!(client.is_issuer(&issuer));
    
    client.remove_issuer(&admin, &issuer);
    assert!(!client.is_issuer(&issuer));
}

#[test]
fn test_create_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    
    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None);
    
    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.issuer, issuer);
    assert_eq!(attestation.subject, subject);
    assert_eq!(attestation.claim_type, claim_type);
    assert!(!attestation.revoked);
}

#[test]
fn test_has_valid_claim() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    
    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None);
    
    assert!(client.has_valid_claim(&subject, &claim_type));
    
    let other_claim = String::from_str(&env, "ACCREDITED");
    assert!(!client.has_valid_claim(&subject, &other_claim));
}

#[test]
fn test_revoke_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    
    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None);
    
    assert!(client.has_valid_claim(&subject, &claim_type));
    
    client.revoke_attestation(&issuer, &attestation_id);
    
    assert!(!client.has_valid_claim(&subject, &claim_type));
    
    let attestation = client.get_attestation(&attestation_id);
    assert!(attestation.revoked);
}

#[test]
fn test_expired_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    
    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let current_time = env.ledger().timestamp();
    let expiration = Some(current_time + 100);
    
    let attestation_id = client.create_attestation(&issuer, &subject, &claim_type, &expiration, &None);
    
    // Should be valid initially
    assert!(client.has_valid_claim(&subject, &claim_type));
    
    // Fast forward time past expiration
    env.ledger().with_mut(|li| {
        li.timestamp = current_time + 200;
    });
    
    // Should now be invalid
    assert!(!client.has_valid_claim(&subject, &claim_type));
    
    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Expired);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_expired_event_emitted_on_has_valid_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (contract_id, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let current_time = env.ledger().timestamp();
    client.create_attestation(&issuer, &subject, &claim_type, &Some(current_time + 100));

    env.ledger().with_mut(|li| li.timestamp = current_time + 200);
    assert!(!client.has_valid_claim(&subject, &claim_type));

    // Verify at least one "expired" event was emitted by this contract
    let expired_sym = soroban_sdk::symbol_short!("expired");
    let found = env.events().all().iter().any(|(id, topics, _)| {
        id == contract_id && topics.get(0).map(|v| v.shallow_eq(&expired_sym.to_val())).unwrap_or(false)
    });
    assert!(found, "expected an expired event to be emitted");
}

#[test]
fn test_expired_event_emitted_on_get_attestation_status() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (contract_id, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let current_time = env.ledger().timestamp();
    let attestation_id = client.create_attestation(
        &issuer, &subject, &claim_type, &Some(current_time + 100),
    );

    env.ledger().with_mut(|li| li.timestamp = current_time + 200);

    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Expired);

    let expired_sym = soroban_sdk::symbol_short!("expired");
    let found = env.events().all().iter().any(|(id, topics, _)| {
        id == contract_id && topics.get(0).map(|v| v.shallow_eq(&expired_sym.to_val())).unwrap_or(false)
    });
    assert!(found, "expected an expired event to be emitted");
}

#[test]
fn test_no_expired_event_for_revoked_attestation() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (contract_id, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let current_time = env.ledger().timestamp();
    let attestation_id = client.create_attestation(
        &issuer, &subject, &claim_type, &Some(current_time + 100),
    );
    client.revoke_attestation(&issuer, &attestation_id);

    env.ledger().with_mut(|li| li.timestamp = current_time + 200);

    // Revoked takes precedence — status is Revoked, not Expired
    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Revoked);

    // No expired event should have been emitted
    let expired_sym = soroban_sdk::symbol_short!("expired");
    let found = env.events().all().iter().any(|(id, topics, _)| {
        id == contract_id && topics.get(0).map(|v| v.shallow_eq(&expired_sym.to_val())).unwrap_or(false)
    });
    assert!(!found, "expired event must not be emitted for revoked attestation");
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_duplicate_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    
    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    
    // Mock the timestamp to be consistent
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });
    
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None);
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None); // Should panic
}

#[test]
fn test_pagination() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    
    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);
    
    // Create multiple attestations
    let claims = ["CLAIM_0", "CLAIM_1", "CLAIM_2", "CLAIM_3", "CLAIM_4"];
    for claim_str in claims.iter() {
        let claim = String::from_str(&env, claim_str);
        client.create_attestation(&issuer, &subject, &claim, &None, &None);
    }
    
    let page1 = client.get_subject_attestations(&subject, &0, &2);
    assert_eq!(page1.len(), 2);
    
    let page2 = client.get_subject_attestations(&subject, &2, &2);
    assert_eq!(page2.len(), 2);
    
    let page3 = client.get_subject_attestations(&subject, &4, &2);
    assert_eq!(page3.len(), 1);
}

// ── Task 5.1 ──────────────────────────────────────────────────────────────────
// Requirements: 3.2, 4.1
#[test]
fn test_create_attestation_with_valid_from() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time = env.ledger().timestamp();
    let future_time = current_time + 1000;
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &Some(future_time));

    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.valid_from, Some(future_time));

    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Pending);
}

// ── Task 5.2 ──────────────────────────────────────────────────────────────────
// Requirements: 2.3, 2.4, 4.1, 4.2
#[test]
fn test_get_status_pending_transitions_to_valid() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 1_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let future_time = current_time + 500;
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &Some(future_time));

    // Before valid_from: status must be Pending
    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Pending);

    // Advance ledger time past valid_from
    env.ledger().with_mut(|l| l.timestamp = future_time + 1);

    // After valid_from: status must be Valid
    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Valid);
}

// ── Task 5.3 ──────────────────────────────────────────────────────────────────
// Requirements: 5.1, 5.3
#[test]
fn test_has_valid_claim_pending_then_valid() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 1_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let future_time = current_time + 500;
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");

    client.create_attestation(&issuer, &subject, &claim_type, &None, &Some(future_time));

    // Before valid_from: has_valid_claim must be false
    assert!(!client.has_valid_claim(&subject, &claim_type));

    // Advance ledger time past valid_from
    env.ledger().with_mut(|l| l.timestamp = future_time + 1);

    // After valid_from: has_valid_claim must be true
    assert!(client.has_valid_claim(&subject, &claim_type));
}

// ── Task 5.4 ──────────────────────────────────────────────────────────────────
// Requirements: 6.1, 6.2, 6.3
#[test]
fn test_create_attestation_valid_from_none_unchanged() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
// ── Batch revocation tests ────────────────────────────────────────────────────

fn setup_batch_env(env: &Env) -> (Address, Address, TrustLinkContractClient) {
    let admin = Address::generate(env);
    let issuer = Address::generate(env);
    let (_, client) = create_test_contract(env);
    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);
    (admin, issuer, client)
}

#[test]
fn test_batch_revoke_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let id1 = client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None);
    let id2 = client.create_attestation(&issuer, &subject, &String::from_str(&env, "ACCREDITED_INVESTOR"), &None);
    let id3 = client.create_attestation(&issuer, &subject, &String::from_str(&env, "MERCHANT_VERIFIED"), &None);

    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id1.clone());
    ids.push_back(id2.clone());
    ids.push_back(id3.clone());

    let count = client.revoke_attestations_batch(&issuer, &ids);
    assert_eq!(count, 3);

    assert!(client.get_attestation(&id1).revoked);
    assert!(client.get_attestation(&id2).revoked);
    assert!(client.get_attestation(&id3).revoked);
}

#[test]
fn test_batch_revoke_returns_count() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let id1 = client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None);
    let id2 = client.create_attestation(&issuer, &subject, &String::from_str(&env, "ACCREDITED_INVESTOR"), &None);

    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id1);
    ids.push_back(id2);

    let count = client.revoke_attestations_batch(&issuer, &ids);
    assert_eq!(count, 2);
}

#[test]
fn test_batch_revoke_emits_events_for_each() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, client) = create_test_contract(&env);
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let claim_type = String::from_str(&env, "KYC_PASSED");

    // Create with valid_from = None — backward-compatible path
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &None);

    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.valid_from, None);

    // Status must be Valid (not Pending)
    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Valid);

    // has_valid_claim must return true
    assert!(client.has_valid_claim(&subject, &claim_type));
}

// ── Task 5.5 ──────────────────────────────────────────────────────────────────
// Requirements: 3.4
#[test]
fn test_create_attestation_valid_from_past_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 2_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let past_time = current_time - 1;
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let result = client.try_create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &Some(past_time),
    );
    assert_eq!(
        result,
        Err(Ok(types::Error::InvalidValidFrom))
    );
}

#[test]
fn test_create_attestation_valid_from_equal_current_time_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 2_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let claim_type = String::from_str(&env, "KYC_PASSED");

    // valid_from == current_time must also be rejected
    let result = client.try_create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &Some(current_time),
    );
    assert_eq!(
        result,
        Err(Ok(types::Error::InvalidValidFrom))
    );
}

// ── Task 5.6 ──────────────────────────────────────────────────────────────────
// Requirements: 2.3, 2.4
#[test]
fn test_revoke_pending_attestation() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 1_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let future_time = current_time + 500;
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &Some(future_time));

    // Revoke while still pending
    client.revoke_attestation(&issuer, &attestation_id);

    // Time-lock is dominant: status is still Pending before valid_from
    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Pending);

    // Advance ledger time past valid_from
    env.ledger().with_mut(|l| l.timestamp = future_time + 1);

    // Now the revocation takes effect: status is Revoked
    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Revoked);
}

// ── Attestation Renewal Unit Tests (Task 5.1) ─────────────────────────────────
// Requirements: 1.2, 1.3, 2.2, 2.3, 3.1, 4.1, 4.2, 5.1, 5.3, 6.2


#[test]
fn test_renew_valid_attestation() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 1_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let initial_expiration = Some(current_time + 500);
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &initial_expiration, &None);

    let new_expiration = Some(current_time + 2_000);
    client.renew_attestation(&issuer, &attestation_id, &new_expiration);

    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.expiration, new_expiration);

    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Valid);
}

#[test]
fn test_renew_expired_attestation() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 1_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let near_expiration = Some(current_time + 100);
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &near_expiration, &None);

    // Advance ledger past expiration
    env.ledger().with_mut(|l| l.timestamp = current_time + 200);

    // Attestation is now expired
    assert_eq!(
        client.get_attestation_status(&attestation_id),
        types::AttestationStatus::Expired
    );

    // Renew with a future expiration
    let new_expiration = Some(current_time + 5_000);
    client.renew_attestation(&issuer, &attestation_id, &new_expiration);

    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Valid);

    assert!(client.has_valid_claim(&subject, &claim_type));
}

#[test]
fn test_renew_with_none_expiration() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 1_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let initial_expiration = Some(current_time + 500);
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &initial_expiration, &None);

    // Renew with None → non-expiring
    client.renew_attestation(&issuer, &attestation_id, &None);

    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.expiration, None);

    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Valid);
}

#[test]
fn test_renew_revoked_attestation_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &None);

    client.revoke_attestation(&issuer, &attestation_id);

    let new_expiration = Some(env.ledger().timestamp() + 1_000);
    let result = client.try_renew_attestation(&issuer, &attestation_id, &new_expiration);
    assert_eq!(result, Err(Ok(types::Error::AlreadyRevoked)));
}

#[test]
fn test_renew_wrong_issuer_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer_a);
    client.register_issuer(&admin, &issuer_b);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id =
        client.create_attestation(&issuer_a, &subject, &claim_type, &None, &None);

    let new_expiration = Some(env.ledger().timestamp() + 1_000);
    // issuer_b tries to renew issuer_a's attestation
    let result = client.try_renew_attestation(&issuer_b, &attestation_id, &new_expiration);
    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
}

#[test]
fn test_renew_unregistered_issuer_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let unregistered = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &None);

    let new_expiration = Some(env.ledger().timestamp() + 1_000);
    // unregistered address attempts renewal
    let result = client.try_renew_attestation(&unregistered, &attestation_id, &new_expiration);
    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
}

#[test]
fn test_renew_missing_attestation_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let nonexistent_id = String::from_str(&env, "does-not-exist");
    let new_expiration = Some(env.ledger().timestamp() + 1_000);
    let result = client.try_renew_attestation(&issuer, &nonexistent_id, &new_expiration);
    assert_eq!(result, Err(Ok(types::Error::NotFound)));
}

#[test]
fn test_renew_past_expiration_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 2_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &None);

    // new_expiration is in the past
    let past_time = current_time - 1;
    let result = client.try_renew_attestation(&issuer, &attestation_id, &Some(past_time));
    assert_eq!(result, Err(Ok(types::Error::InvalidExpiration)));
}

#[test]
fn test_renew_expiration_equal_current_time_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 2_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &None);

    // new_expiration == current_time must also be rejected
    let result = client.try_renew_attestation(&issuer, &attestation_id, &Some(current_time));
    assert_eq!(result, Err(Ok(types::Error::InvalidExpiration)));
}

#[test]
fn test_renewal_preserves_original_fields() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 1_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let valid_from = Some(current_time + 1); // just above current so it's accepted
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &valid_from);

    let before = client.get_attestation(&attestation_id);

    // Advance time past valid_from so renewal is allowed
    env.ledger().with_mut(|l| l.timestamp = current_time + 100);

    let new_expiration = Some(current_time + 5_000);
    client.renew_attestation(&issuer, &attestation_id, &new_expiration);

    let after = client.get_attestation(&attestation_id);

    // Only expiration should change
    assert_eq!(after.issuer, before.issuer);
    assert_eq!(after.subject, before.subject);
    assert_eq!(after.claim_type, before.claim_type);
    assert_eq!(after.timestamp, before.timestamp);
    assert_eq!(after.valid_from, before.valid_from);
    // expiration is updated
    assert_eq!(after.expiration, new_expiration);
}

#[test]
fn test_no_event_on_renewal_error() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &None);

    client.revoke_attestation(&issuer, &attestation_id);

    // Capture event count before the failing renewal
    let events_before = env.events().all().len();

    let new_expiration = Some(env.ledger().timestamp() + 1_000);
    let _ = client.try_renew_attestation(&issuer, &attestation_id, &new_expiration);

    // No new events should have been emitted
    let events_after = env.events().all().len();
    assert_eq!(events_before, events_after);
    let id1 = client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None);
    let id2 = client.create_attestation(&issuer, &subject, &String::from_str(&env, "ACCREDITED_INVESTOR"), &None);

    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id1);
    ids.push_back(id2);

    client.revoke_attestations_batch(&issuer, &ids);

    let revoked_sym = soroban_sdk::symbol_short!("revoked");
    let revoked_count = env.events().all().iter().filter(|(id, topics, _)| {
        *id == contract_id && topics.get(0).map(|v| v.shallow_eq(&revoked_sym.to_val())).unwrap_or(false)
    }).count();

    assert_eq!(revoked_count, 2, "expected one revoked event per attestation");
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_batch_revoke_unauthorized_issuer_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup_batch_env(&env);
    let other_issuer = Address::generate(&env);
    client.register_issuer(&admin, &other_issuer);

    let subject = Address::generate(&env);
    // issuer creates an attestation
    let id = client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None);

    // other_issuer tries to revoke issuer's attestation — must panic Unauthorized
    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id);
    client.revoke_attestations_batch(&other_issuer, &ids);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_batch_revoke_already_revoked_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let id = client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None);
    client.revoke_attestation(&issuer, &id);

    // Attempting to batch-revoke an already-revoked attestation must panic AlreadyRevoked
    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id);
    client.revoke_attestations_batch(&issuer, &ids);
}

#[test]
fn test_batch_revoke_single_auth_check() {
    // Verifies the function works end-to-end with mock_all_auths (single auth path).
    // If auth were checked per-attestation the mock would still pass, but this
    // confirms the happy-path with one auth invocation for the whole batch.
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let mut ids = soroban_sdk::Vec::new(&env);
    for claim in ["C1", "C2", "C3", "C4", "C5"].iter() {
        let id = client.create_attestation(
            &issuer, &subject, &String::from_str(&env, claim), &None,
        );
        ids.push_back(id);
    }

    let count = client.revoke_attestations_batch(&issuer, &ids);
    assert_eq!(count, 5);
}

#[test]
fn test_batch_revoke_empty_vec() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup_batch_env(&env);

    let ids: soroban_sdk::Vec<String> = soroban_sdk::Vec::new(&env);
    let count = client.revoke_attestations_batch(&issuer, &ids);
    assert_eq!(count, 0);
}

// ── Claim type registry tests ─────────────────────────────────────────────────

#[test]
fn test_register_and_get_claim_type() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin);

    let ct = String::from_str(&env, "KYC_PASSED");
    let desc = String::from_str(&env, "Subject has passed KYC verification");
    client.register_claim_type(&admin, &ct, &desc);

    let result = client.get_claim_type_description(&ct);
    assert_eq!(result, Some(desc));
}

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let current_time: u64 = 1_000;
    env.ledger().with_mut(|l| l.timestamp = current_time);

    let future_time = current_time + 500;
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim_type, &None, &Some(future_time));

    // Revoke while still pending
    client.revoke_attestation(&issuer, &attestation_id);

    // Time-lock is dominant: status is still Pending before valid_from
    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Pending);

    // Advance ledger time past valid_from
    env.ledger().with_mut(|l| l.timestamp = future_time + 1);

    // Now the revocation takes effect: status is Revoked
    let status = client.get_attestation_status(&attestation_id);
    assert_eq!(status, types::AttestationStatus::Revoked);
    let non_admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin);
#[test]
fn test_get_claim_type_description_unknown_returns_none() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin);

    let result = client.get_claim_type_description(&String::from_str(&env, "UNKNOWN"));
    assert_eq!(result, None);
}

#[test]
fn test_register_claim_type_updates_description() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin);

    let ct = String::from_str(&env, "KYC_PASSED");
    client.register_claim_type(&admin, &ct, &String::from_str(&env, "v1 description"));
    client.register_claim_type(&admin, &ct, &String::from_str(&env, "v2 description"));

    let result = client.get_claim_type_description(&ct);
    assert_eq!(result, Some(String::from_str(&env, "v2 description")));
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_register_claim_type_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let not_admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin);

    client.register_claim_type(
        &not_admin,
        &String::from_str(&env, "KYC_PASSED"),
        &String::from_str(&env, "desc"),
    );
}

#[test]
fn test_list_claim_types_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin);

    let types = [
        ("KYC_PASSED",          "Passed KYC"),
        ("ACCREDITED_INVESTOR", "Accredited investor status"),
        ("MERCHANT_VERIFIED",   "Verified merchant"),
        ("AML_CLEARED",         "AML screening passed"),
        ("SANCTIONS_CHECKED",   "Sanctions list checked"),
    ];

    for (ct, desc) in types.iter() {
        client.register_claim_type(
            &admin,
            &String::from_str(&env, ct),
            &String::from_str(&env, desc),
        );
    }

    let page1 = client.list_claim_types(&0, &2);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap(), String::from_str(&env, "KYC_PASSED"));

    let page2 = client.list_claim_types(&2, &2);
    assert_eq!(page2.len(), 2);

    let page3 = client.list_claim_types(&4, &2);
    assert_eq!(page3.len(), 1);
    assert_eq!(page3.get(0).unwrap(), String::from_str(&env, "SANCTIONS_CHECKED"));
}

#[test]
fn test_list_claim_types_empty() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin);

    let result = client.list_claim_types(&0, &10);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_register_claim_type_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (contract_id, client) = create_test_contract(&env);
    client.initialize(&admin);

    let ct = String::from_str(&env, "KYC_PASSED");
    client.register_claim_type(&admin, &ct, &String::from_str(&env, "KYC verified"));

    let clmtype_sym = soroban_sdk::symbol_short!("clmtype");
    let found = env.events().all().iter().any(|(id, topics, _)| {
        id == contract_id
            && topics.get(0).map(|v| v.shallow_eq(&clmtype_sym.to_val())).unwrap_or(false)
    });
    assert!(found, "expected a clmtype event to be emitted");
}

// ── update_expiration tests ───────────────────────────────────────────────────

#[test]
fn test_update_expiration_extend() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let current_time = env.ledger().timestamp();
    let id = client.create_attestation(
        &issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &Some(current_time + 100),
    );

    // Extend expiration
    client.update_expiration(&issuer, &id, &Some(current_time + 1000));

    let attestation = client.get_attestation(&id);
    assert_eq!(attestation.expiration, Some(current_time + 1000));
}

#[test]
fn test_update_expiration_shorten() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let current_time = env.ledger().timestamp();
    let id = client.create_attestation(
        &issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &Some(current_time + 1000),
    );

    client.update_expiration(&issuer, &id, &Some(current_time + 50));

    let attestation = client.get_attestation(&id);
    assert_eq!(attestation.expiration, Some(current_time + 50));
}

#[test]
fn test_update_expiration_remove() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let current_time = env.ledger().timestamp();
    let id = client.create_attestation(
        &issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &Some(current_time + 100),
    );

    // Remove expiration entirely
    client.update_expiration(&issuer, &id, &None);

    let attestation = client.get_attestation(&id);
    assert_eq!(attestation.expiration, None);
}

#[test]
fn test_update_expiration_status_reflects_immediately() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let current_time = env.ledger().timestamp();
    let id = client.create_attestation(
        &issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &Some(current_time + 100),
    );

    // Fast-forward past expiration — should be expired
    env.ledger().with_mut(|li| li.timestamp = current_time + 200);
    assert_eq!(client.get_attestation_status(&id), types::AttestationStatus::Expired);

    // Extend expiration beyond current time — should be valid again
    client.update_expiration(&issuer, &id, &Some(current_time + 500));
    assert_eq!(client.get_attestation_status(&id), types::AttestationStatus::Valid);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_update_expiration_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, client) = setup_batch_env(&env);
    let other_issuer = Address::generate(&env);
    client.register_issuer(&admin, &other_issuer);

    let subject = Address::generate(&env);
    let id = client.create_attestation(
        &issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None,
    );

    // other_issuer cannot update issuer's attestation
    client.update_expiration(&other_issuer, &id, &Some(9999));
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_update_expiration_revoked_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let id = client.create_attestation(
        &issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None,
    );
    client.revoke_attestation(&issuer, &id);

    // Cannot update a revoked attestation
    client.update_expiration(&issuer, &id, &Some(9999));
}

#[test]
fn test_update_expiration_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, client) = create_test_contract(&env);
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let id = client.create_attestation(
        &issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None,
    );

    client.update_expiration(&issuer, &id, &Some(5000));

    let updated_sym = soroban_sdk::symbol_short!("updated");
    let found = env.events().all().iter().any(|(cid, topics, _)| {
        cid == contract_id
            && topics.get(0).map(|v| v.shallow_eq(&updated_sym.to_val())).unwrap_or(false)
    });
    assert!(found, "expected an updated event to be emitted");
}

// ── Version / metadata tests ──────────────────────────────────────────────────

#[test]
fn test_get_version_after_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin);

    let version = client.get_version();
    assert_eq!(version, String::from_str(&env, "1.0.0"));
}

#[test]
fn test_get_contract_metadata_after_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin);

    let meta = client.get_contract_metadata();
    assert_eq!(meta.name, String::from_str(&env, "TrustLink"));
    assert_eq!(meta.version, String::from_str(&env, "1.0.0"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_get_version_before_initialization_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client) = create_test_contract(&env);
    client.get_version(); // NotInitialized
}

#[test]
fn test_get_valid_claims_multiple_types() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None);
    env.ledger().with_mut(|l| l.timestamp += 1);
    client.create_attestation(&issuer, &subject, &String::from_str(&env, "ACCREDITED_INVESTOR"), &None);
    env.ledger().with_mut(|l| l.timestamp += 1);
    client.create_attestation(&issuer, &subject, &String::from_str(&env, "MERCHANT_VERIFIED"), &None);

    let claims = client.get_valid_claims(&subject);
    assert_eq!(claims.len(), 3);
    assert!(claims.contains(&String::from_str(&env, "KYC_PASSED")));
    assert!(claims.contains(&String::from_str(&env, "ACCREDITED_INVESTOR")));
    assert!(claims.contains(&String::from_str(&env, "MERCHANT_VERIFIED")));
}

#[test]
fn test_get_valid_claims_excludes_revoked() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let id = client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None);
    env.ledger().with_mut(|l| l.timestamp += 1);
    client.create_attestation(&issuer, &subject, &String::from_str(&env, "ACCREDITED_INVESTOR"), &None);

    client.revoke_attestation(&issuer, &id);

    let claims = client.get_valid_claims(&subject);
    assert_eq!(claims.len(), 1);
    assert!(claims.contains(&String::from_str(&env, "ACCREDITED_INVESTOR")));
    assert!(!claims.contains(&String::from_str(&env, "KYC_PASSED")));
}

#[test]
fn test_get_valid_claims_excludes_expired() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    // Expires at timestamp 100
    client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &Some(100));
    env.ledger().with_mut(|l| l.timestamp += 1);
    client.create_attestation(&issuer, &subject, &String::from_str(&env, "ACCREDITED_INVESTOR"), &None);

    // Advance past expiration
    env.ledger().with_mut(|l| l.timestamp = 200);

    let claims = client.get_valid_claims(&subject);
    assert_eq!(claims.len(), 1);
    assert!(claims.contains(&String::from_str(&env, "ACCREDITED_INVESTOR")));
    assert!(!claims.contains(&String::from_str(&env, "KYC_PASSED")));
}

#[test]
fn test_get_valid_claims_empty_when_none_valid() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    let id = client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None);
    client.revoke_attestation(&issuer, &id);

    let claims = client.get_valid_claims(&subject);
    assert_eq!(claims.len(), 0);
}

#[test]
fn test_get_valid_claims_empty_for_unknown_subject() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin);

    let unknown = Address::generate(&env);
    let claims = client.get_valid_claims(&unknown);
    assert_eq!(claims.len(), 0);
}

#[test]
fn test_get_valid_claims_deduplicates() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    // Two valid KYC_PASSED attestations from different timestamps
    client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None);
    env.ledger().with_mut(|l| l.timestamp += 1);
    client.create_attestation(&issuer, &subject, &String::from_str(&env, "KYC_PASSED"), &None);

    let claims = client.get_valid_claims(&subject);
    assert_eq!(claims.len(), 1);
    assert!(claims.contains(&String::from_str(&env, "KYC_PASSED")));
}

#[test]
fn test_get_valid_claims_large_attestation_set() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin);
    client.register_issuer(&admin, &issuer);

    // Create 20 attestations: 10 unique valid claim types + 10 revoked
    let claim_types = [
        "CLAIM_A", "CLAIM_B", "CLAIM_C", "CLAIM_D", "CLAIM_E",
        "CLAIM_F", "CLAIM_G", "CLAIM_H", "CLAIM_I", "CLAIM_J",
    ];

    for (i, ct) in claim_types.iter().enumerate() {
        env.ledger().with_mut(|l| l.timestamp = i as u64 + 1);
        client.create_attestation(&issuer, &subject, &String::from_str(&env, ct), &None);
    }

    // Add 10 more that will be revoked
    let mut revoke_ids = soroban_sdk::Vec::new(&env);
    for (i, ct) in claim_types.iter().enumerate() {
        env.ledger().with_mut(|l| l.timestamp = i as u64 + 100);
        let id = client.create_attestation(&issuer, &subject, &String::from_str(&env, ct), &None);
        revoke_ids.push_back(id);
    }
    client.revoke_attestations_batch(&issuer, &revoke_ids);

    let claims = client.get_valid_claims(&subject);
    // Should still return exactly 10 unique valid claim types
    assert_eq!(claims.len(), 10);
}
