//! Tests for get_issuer_list() paginated query.
//!
//! Acceptance criteria:
//!   - Empty list before any issuers are registered.
//!   - Single issuer appears after registration and is removed after removal.
//!   - Multi-page pagination returns correct slices.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use trustlink::{TrustLinkContract, TrustLinkContractClient};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn setup(env: &Env) -> (Address, TrustLinkContractClient<'_>) {
    let id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin, &None);
    (admin, client)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_empty_list_before_any_registration() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, client) = setup(&env);

    let list = client.get_issuer_list(&0, &10);
    assert_eq!(list.len(), 0);
}

#[test]
fn test_single_issuer_appears_after_registration() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup(&env);
    let issuer = Address::generate(&env);

    client.register_issuer(&admin, &issuer);

    let list = client.get_issuer_list(&0, &10);
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), issuer);
}

#[test]
fn test_issuer_removed_from_list_after_removal() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup(&env);
    let issuer = Address::generate(&env);

    client.register_issuer(&admin, &issuer);
    assert_eq!(client.get_issuer_list(&0, &10).len(), 1);

    client.remove_issuer(&admin, &issuer);
    assert_eq!(client.get_issuer_list(&0, &10).len(), 0);
}

#[test]
fn test_multi_page_pagination() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup(&env);

    // Register 5 issuers
    let mut issuers = Vec::new(&env);
    for _ in 0..5 {
        let issuer = Address::generate(&env);
        client.register_issuer(&admin, &issuer);
        issuers.push_back(issuer);
    }

    // Page 1: first 3
    let page1 = client.get_issuer_list(&0, &3);
    assert_eq!(page1.len(), 3);
    assert_eq!(page1.get(0).unwrap(), issuers.get(0).unwrap());
    assert_eq!(page1.get(1).unwrap(), issuers.get(1).unwrap());
    assert_eq!(page1.get(2).unwrap(), issuers.get(2).unwrap());

    // Page 2: remaining 2
    let page2 = client.get_issuer_list(&3, &3);
    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap(), issuers.get(3).unwrap());
    assert_eq!(page2.get(1).unwrap(), issuers.get(4).unwrap());
}

#[test]
fn test_start_beyond_list_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup(&env);
    let issuer = Address::generate(&env);

    client.register_issuer(&admin, &issuer);

    let list = client.get_issuer_list(&99, &10);
    assert_eq!(list.len(), 0);
}

// ---------------------------------------------------------------------------
// Removed issuers invalidate their attestations (strict behaviour)
// ---------------------------------------------------------------------------

#[test]
fn test_removed_issuer_attestations_become_invalid() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let kyc = String::from_str(&env, "KYC");

    client.register_issuer(&admin, &issuer);
    client.create_attestation(&issuer, &subject, &kyc, &None, &None, &None);

    // While the issuer is registered, the claim is valid.
    assert!(client.has_valid_claim(&subject, &kyc));
    assert!(client.has_valid_claim_from_issuer(&subject, &kyc, &issuer));
    assert!(client.has_any_claim(&subject));
    assert!(client.has_all_claims(&subject, &Vec::from_array(&env, [kyc.clone()])));
    assert_eq!(client.get_valid_claims(&subject).len(), 1);
    assert!(client.get_attestation_by_type(&subject, &kyc).is_some());

    // Removing the issuer invalidates all of its attestations.
    client.remove_issuer(&admin, &issuer);

    assert!(!client.has_valid_claim(&subject, &kyc));
    assert!(!client.has_valid_claim_from_issuer(&subject, &kyc, &issuer));
    assert!(!client.has_any_claim(&subject));
    assert!(!client.has_all_claims(&subject, &Vec::from_array(&env, [kyc.clone()])));
    assert_eq!(client.get_valid_claims(&subject).len(), 0);
    assert!(client.get_attestation_by_type(&subject, &kyc).is_none());
}

#[test]
fn test_reregistering_issuer_restores_attestation_validity() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let kyc = String::from_str(&env, "KYC");

    client.register_issuer(&admin, &issuer);
    client.create_attestation(&issuer, &subject, &kyc, &None, &None, &None);

    client.remove_issuer(&admin, &issuer);
    assert!(!client.has_valid_claim(&subject, &kyc));

    // Re-registering the same issuer makes its (non-revoked) attestations valid again.
    client.register_issuer(&admin, &issuer);
    assert!(client.has_valid_claim(&subject, &kyc));
    assert!(client.has_valid_claim_from_issuer(&subject, &kyc, &issuer));
    assert!(client.has_any_claim(&subject));
    assert_eq!(client.get_valid_claims(&subject).len(), 1);
}
