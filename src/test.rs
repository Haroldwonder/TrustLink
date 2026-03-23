#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Events as _, Ledger}, Address, Env, String};

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
    let attestation_id = client.create_attestation(&issuer, &subject, &claim_type, &None);
    
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
    client.create_attestation(&issuer, &subject, &claim_type, &None);
    
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
    let attestation_id = client.create_attestation(&issuer, &subject, &claim_type, &None);
    
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
    
    let attestation_id = client.create_attestation(&issuer, &subject, &claim_type, &expiration);
    
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
    
    client.create_attestation(&issuer, &subject, &claim_type, &None);
    client.create_attestation(&issuer, &subject, &claim_type, &None); // Should panic
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
        client.create_attestation(&issuer, &subject, &claim, &None);
    }
    
    let page1 = client.get_subject_attestations(&subject, &0, &2);
    assert_eq!(page1.len(), 2);
    
    let page2 = client.get_subject_attestations(&subject, &2, &2);
    assert_eq!(page2.len(), 2);
    
    let page3 = client.get_subject_attestations(&subject, &4, &2);
    assert_eq!(page3.len(), 1);
}

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

// ── get_attestation_by_type tests ─────────────────────────────────

#[test]
fn test_get_attestation_by_type_found() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let id = client.create_attestation(&issuer, &subject, &claim_type, &None);

    let attestation = client.get_attestation_by_type(&subject, &claim_type);
    assert_eq!(attestation.id, id);
    assert_eq!(attestation.claim_type, claim_type);
    assert_eq!(attestation.subject, subject);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_get_attestation_by_type_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    // No attestations created — must return NotFound
    client.get_attestation_by_type(&subject, &String::from_str(&env, "KYC_PASSED"));
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_get_attestation_by_type_ignores_revoked() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let id = client.create_attestation(&issuer, &subject, &claim_type, &None);
    client.revoke_attestation(&issuer, &id);

    // Revoked attestation must not be returned
    client.get_attestation_by_type(&subject, &claim_type);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_get_attestation_by_type_ignores_expired() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let claim_type = String::from_str(&env, "KYC_PASSED");
    let current_time = env.ledger().timestamp();
    client.create_attestation(&issuer, &subject, &claim_type, &Some(current_time + 100));

    env.ledger().with_mut(|li| li.timestamp = current_time + 200);

    // Expired attestation must not be returned
    client.get_attestation_by_type(&subject, &claim_type);
}

#[test]
fn test_get_attestation_by_type_multiple_claim_types() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);

    let kyc = String::from_str(&env, "KYC_PASSED");
    let accredited = String::from_str(&env, "ACCREDITED_INVESTOR");

    client.create_attestation(&issuer, &subject, &kyc, &None);
    client.create_attestation(&issuer, &subject, &accredited, &None);

    // Each claim type resolves independently
    let kyc_result = client.get_attestation_by_type(&subject, &kyc);
    assert_eq!(kyc_result.claim_type, kyc);

    let acc_result = client.get_attestation_by_type(&subject, &accredited);
    assert_eq!(acc_result.claim_type, accredited);
}

#[test]
fn test_get_attestation_by_type_returns_most_recent_valid() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup_batch_env(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    // First attestation — will be revoked
    env.ledger().with_mut(|li| li.timestamp = 1000);
    let id_old = client.create_attestation(&issuer, &subject, &claim_type, &None);
    client.revoke_attestation(&issuer, &id_old);

    // Second attestation — valid, created later
    env.ledger().with_mut(|li| li.timestamp = 2000);
    let id_new = client.create_attestation(&issuer, &subject, &claim_type, &None);

    let result = client.get_attestation_by_type(&subject, &claim_type);
    assert_eq!(result.id, id_new);
}
