//! Tests for the export_revocation_list endpoint.
//!
//! Verifies that the revocation list export endpoint correctly exports
//! revocation status for attestations in a compact, standards-adjacent format.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use trustlink::{types::RevocationListFormat, TrustLinkContract, TrustLinkContractClient};

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

fn aml(env: &Env) -> String {
    String::from_str(env, "AML_CHECKED")
}

#[test]
fn test_export_revocation_list_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, _, client) = setup(&env);

    // Export should return empty list
    let list = client.export_revocation_list(&issuer, &None, &RevocationListFormat::SimpleList);
    let list = list.unwrap();

    assert_eq!(list.issuer, issuer);
    assert_eq!(list.claim_type, None);
    assert_eq!(list.revoked_attestation_ids.len(), 0);
    assert_eq!(list.total_attestation_count, 0);
    assert_eq!(list.revoked_count, 0);
    assert!(list.bitstring.is_none());
}

#[test]
fn test_export_revocation_list_with_revocations() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);

    // Create attestations
    let id1 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    let id2 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    let id3 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Revocate some attestations
    client.revoke_attestation(&issuer, &id2, &None);
    client.revoke_attestation(&issuer, &id3, &None);

    // Export all revocations
    let list = client.export_revocation_list(&issuer, &None, &RevocationListFormat::SimpleList);
    let list = list.unwrap();

    assert_eq!(list.issuer, issuer);
    assert_eq!(list.claim_type, None);
    assert_eq!(list.total_attestation_count, 3);
    assert_eq!(list.revoked_count, 2);

    // Check that revoked IDs are in the list
    let revoked_ids: Vec<String> = list.revoked_attestation_ids;
    assert!(revoked_ids.iter().any(|id| *id == id2));
    assert!(revoked_ids.iter().any(|id| *id == id3));
    assert!(!revoked_ids.iter().any(|id| *id == id1)); // id1 should not be revoked
}

#[test]
fn test_export_revocation_list_by_claim_type() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_a = kyc(&env);
    let claim_b = aml(&env);

    // Create attestations for different claim types
    let id_a1 = client.create_attestation(&issuer, &subject, &claim_a, &None, &None, &None);
    let id_a2 = client.create_attestation(&issuer, &subject, &claim_a, &None, &None, &None);
    let id_b1 = client.create_attestation(&issuer, &subject, &claim_b, &None, &None, &None);

    // Revocate one from each claim type
    client.revoke_attestation(&issuer, &id_a2, &None);
    client.revoke_attestation(&issuer, &id_b1, &None);

    // Export only KYC revocations
    let list = client.export_revocation_list(&issuer, &Some(claim_a), &RevocationListFormat::SimpleList);
    let list = list.unwrap();

    assert_eq!(list.issuer, issuer);
    assert_eq!(list.claim_type, Some(claim_a.clone()));
    assert_eq!(list.total_attestation_count, 2); // Only KYC attestations
    assert_eq!(list.revoked_count, 1);

    let revoked_ids: Vec<String> = list.revoked_attestation_ids;
    assert!(revoked_ids.iter().any(|id| *id == id_a2));
    assert!(!revoked_ids.iter().any(|id| *id == id_a1));
    assert!(!revoked_ids.iter().any(|id| *id == id_b1)); // Not KYC
}

#[test]
fn test_export_revocation_list_all_revoked() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);

    // Create and immediately revoke attestations
    let id1 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id1, &None);

    let id2 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id2, &None);

    let list = client.export_revocation_list(&issuer, &None, &RevocationListFormat::SimpleList);
    let list = list.unwrap();

    assert_eq!(list.total_attestation_count, 2);
    assert_eq!(list.revoked_count, 2); // All are revoked
}

#[test]
fn test_export_revocation_list_no_revoked() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);

    // Create attestations but don't revoke any
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    let list = client.export_revocation_list(&issuer, &None, &RevocationListFormat::SimpleList);
    let list = list.unwrap();

    assert_eq!(list.total_attestation_count, 2);
    assert_eq!(list.revoked_count, 0);
    assert_eq!(list.revoked_attestation_ids.len(), 0);
}

#[test]
fn test_export_revocation_list_format_bitstring() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);

    // Create and revoke attestations
    let id1 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    let id2 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id2, &None);

    // Export with bitstring format
    let list = client.export_revocation_list(&issuer, &None, &RevocationListFormat::Bitstring);
    let list = list.unwrap();

    assert_eq!(list.total_attestation_count, 2);
    assert_eq!(list.revoked_count, 1);
    // Bitstring is optional in current implementation
    assert!(list.bitstring.is_none());
}

#[test]
fn test_export_revocation_list_timestamp() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, _, client) = setup(&env);

    // Get initial timestamp
    let initial_time = env.ledger().timestamp();

    // Export with delay
    env.ledger().with_mut(|l| l.timestamp = initial_time + 100);
    let list = client.export_revocation_list(&issuer, &None, &RevocationListFormat::SimpleList);
    let list = list.unwrap();

    assert_eq!(list.generated_at, initial_time + 100);
}

#[test]
fn test_export_revocation_list_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, _, client) = setup(&env);

    let other = Address::generate(&env);

    // Other user should not be able to export
    let result = client.export_revocation_list(&other, &None, &RevocationListFormat::SimpleList);
    assert!(result.is_err());
}

#[test]
fn test_export_revocation_list_after_deletion() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim_type = kyc(&env);

    // Create attestations
    let id1 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    let id2 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Revoke one
    client.revoke_attestation(&issuer, &id2, &None);

    // Delete one (without revoking)
    client.request_deletion(&subject, &id1);

    let list = client.export_revocation_list(&issuer, &None, &RevocationListFormat::SimpleList);
    let list = list.unwrap();

    // Deleted attestations should not be included
    assert_eq!(list.total_attestation_count, 1); // Only id2 remains
    assert_eq!(list.revoked_count, 1);
}

#[test]
fn test_export_revocation_list_multiple_issuers() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let issuer_a = Address::generate(&env);
    let issuer_b = Address::generate(&env);
    let subject = Address::generate(&env);

    let id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(&env, &id);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer_a);
    client.register_issuer(&admin, &issuer_b);

    let claim_type = kyc(&env);

    // Create and revoke for issuer A
    let id_a = client.create_attestation(&issuer_a, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer_a, &id_a, &None);

    // Create but don't revoke for issuer B
    let id_b = client.create_attestation(&issuer_b, &subject, &claim_type, &None, &None, &None);

    // Export for issuer A
    let list_a = client.export_revocation_list(&issuer_a, &None, &RevocationListFormat::SimpleList);
    let list_a = list_a.unwrap();
    assert_eq!(list_a.total_attestation_count, 1);
    assert_eq!(list_a.revoked_count, 1);
    assert_eq!(list_a.issuer, issuer_a);

    // Export for issuer B
    let list_b = client.export_revocation_list(&issuer_b, &None, &RevocationListFormat::SimpleList);
    let list_b = list_b.unwrap();
    assert_eq!(list_b.total_attestation_count, 1);
    assert_eq!(list_b.revoked_count, 0);
    assert_eq!(list_b.issuer, issuer_b);
}
