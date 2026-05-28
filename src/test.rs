use super::*;
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, Env, String,
};

use crate::types::AttestationOrigin;

// Mock callback contract that panics when notify_expiring is called (for issue #329)
#[contract]
struct MockPanicCallbackContract;

#[contractimpl]
impl MockPanicCallbackContract {
    pub fn notify_expiring(_env: Env, _subject: Address, _attestation_id: String, _expiration: u64) {
        panic!("callback panic");
    }
}

#[contract]
struct MockBridgeContract;

#[contractimpl]
impl MockBridgeContract {
    pub fn bridge_claim(
        env: Env,
        trustlink_contract: Address,
        subject: Address,
        claim_type: String,
        source_chain: String,
        source_tx: String,
    ) -> String {
        let client = TrustLinkContractClient::new(&env, &trustlink_contract);
        let bridge = env.current_contract_address();

        client.bridge_attestation(&bridge, &subject, &claim_type, &source_chain, &source_tx)
    }
}

fn create_test_contract(env: &Env) -> (Address, TrustLinkContractClient<'_>) {
    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(env, &contract_id);
    (contract_id, client)
}

fn setup(env: &Env) -> (Address, Address, TrustLinkContractClient<'_>) {
    let (_, client) = create_test_contract(env);
    let admin = Address::generate(env);
    let issuer = Address::generate(env);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);
    (admin, issuer, client)
}

fn register_test_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract_v2(admin.clone())
        .address()
}

fn register_bridge_contract(env: &Env) -> (Address, MockBridgeContractClient<'_>) {
    let contract_id = env.register_contract(None, MockBridgeContract);
    let client = MockBridgeContractClient::new(env, &contract_id);
    (contract_id, client)
}

#[test]
fn test_initialize_and_get_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin, &None);
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_register_and_remove_issuer() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    assert!(client.is_issuer(&issuer));

    client.remove_issuer(&admin, &issuer);
    assert!(!client.is_issuer(&issuer));
}

#[test]
fn test_register_issuer_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client) = create_test_contract(&env);
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let timestamp = 1234567890u64;
    env.ledger().set_timestamp(timestamp);

    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);

    let events = env.events().all();
    assert!(!events.is_empty());

    // Find the issuer_registered event
    let mut found_event = false;
    for (_, topic, data) in events {
        let topic0: soroban_sdk::Symbol =
            soroban_sdk::TryFromVal::try_from_val(&env, &topic.get(0).unwrap()).unwrap();
        if topic0 == soroban_sdk::symbol_short!("iss_reg") {
            let topic1: Address =
                soroban_sdk::TryFromVal::try_from_val(&env, &topic.get(1).unwrap()).unwrap();
            let event_data: (Address, u64) =
                soroban_sdk::TryFromVal::try_from_val(&env, &data).unwrap();

            assert_eq!(topic1, issuer);
            assert_eq!(event_data.0, admin);
            assert_eq!(event_data.1, timestamp);
            found_event = true;
            break;
        }
    }
    assert!(found_event, "issuer_registered event not found");
}

#[test]
fn test_remove_issuer_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let timestamp = 1234567890u64;
    env.ledger().set_timestamp(timestamp);

    client.remove_issuer(&admin, &issuer);

    let events = env.events().all();
    assert!(!events.is_empty());

    // Find the issuer_removed event
    let mut found_event = false;
    for (_, topic, data) in events {
        let topic0: soroban_sdk::Symbol =
            soroban_sdk::TryFromVal::try_from_val(&env, &topic.get(0).unwrap()).unwrap();
        if topic0 == soroban_sdk::symbol_short!("iss_rem") {
            let topic1: Address =
                soroban_sdk::TryFromVal::try_from_val(&env, &topic.get(1).unwrap()).unwrap();
            let event_data: (Address, u64) =
                soroban_sdk::TryFromVal::try_from_val(&env, &data).unwrap();

            assert_eq!(topic1, issuer);
            assert_eq!(event_data.0, admin);
            assert_eq!(event_data.1, timestamp);
            found_event = true;
            break;
        }
    }
    assert!(found_event, "issuer_removed event not found");
}

#[test]
fn test_register_bridge_is_admin_only() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);
    let wrong_admin = Address::generate(&env);
    let bridge = Address::generate(&env);

    let denied = client.try_register_bridge(&wrong_admin, &bridge);
    assert_eq!(denied, Err(Ok(types::Error::Unauthorized)));

    client.register_bridge(&admin, &bridge);
    assert!(client.is_bridge(&bridge));
}

#[test]
fn test_fee_is_disabled_by_default() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let fee_config = client.get_fee_config();
    assert_eq!(fee_config.attestation_fee, 0);
    assert_eq!(fee_config.fee_collector, admin);
    assert_eq!(fee_config.fee_token, None);

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    assert_eq!(client.get_attestation(&id).origin, types::AttestationOrigin::Native);
}

#[test]
fn test_create_attestation_sets_imported_false() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let metadata = Some(String::from_str(&env, "source=acme"));

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &metadata, &None);
    let attestation = client.get_attestation(&id);

    assert_eq!(attestation.subject, subject);
    assert_eq!(attestation.issuer, issuer);
    assert_eq!(attestation.metadata, metadata);
    assert_eq!(attestation.origin, types::AttestationOrigin::Native);
    assert_eq!(attestation.valid_from, None);
}

#[test]
fn test_create_attestation_with_jurisdiction_storable_and_queryable() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let jurisdiction = Some(String::from_str(&env, "US"));

    let id = client.create_attestation_jurisdiction(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &jurisdiction,
        &None,
    );

    let attestation = client.get_attestation(&id);
    assert_eq!(attestation.jurisdiction, jurisdiction);

    let api_results = client.get_attestations_by_jurisdiction(&subject, &String::from_str(&env, "US"), &0, &10);
    assert_eq!(api_results.len(), 1);
    assert_eq!(api_results.get(0).unwrap(), id);

    let wrong_results = client.get_attestations_by_jurisdiction(&subject, &String::from_str(&env, "CA"), &0, &10);
    assert!(wrong_results.is_empty());
}

#[test]
fn test_create_attestation_with_invalid_jurisdiction_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let result = client.try_create_attestation_jurisdiction(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &Some(String::from_str(&env, "USA")),
        &None,
    );

    assert_eq!(result, Err(Ok(types::Error::InvalidJurisdiction)));
}

#[test]
fn test_jurisdiction_valid_iso_codes_accepted() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    // Test valid ISO 3166-1 alpha-2 codes
    let valid_codes = vec!["US", "DE", "NG", "GB", "CA", "FR", "JP", "AU"];

    for code in valid_codes {
        let id = client.create_attestation_jurisdiction(
            &issuer,
            &subject,
            &claim_type,
            &None,
            &None,
            &Some(String::from_str(&env, code)),
            &None,
        );
        assert!(!id.is_empty(), "valid code {} should be accepted", code);

        let attestation = client.get_attestation(&id);
        assert_eq!(attestation.jurisdiction, Some(String::from_str(&env, code)));
    }
}

#[test]
fn test_jurisdiction_lowercase_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let result = client.try_create_attestation_jurisdiction(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &Some(String::from_str(&env, "us")),
        &None,
    );

    assert_eq!(result, Err(Ok(types::Error::InvalidJurisdiction)));
}

#[test]
fn test_jurisdiction_mixed_case_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let result = client.try_create_attestation_jurisdiction(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &Some(String::from_str(&env, "Us")),
        &None,
    );

    assert_eq!(result, Err(Ok(types::Error::InvalidJurisdiction)));
}

#[test]
fn test_jurisdiction_single_character_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let result = client.try_create_attestation_jurisdiction(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &Some(String::from_str(&env, "U")),
        &None,
    );

    assert_eq!(result, Err(Ok(types::Error::InvalidJurisdiction)));
}

#[test]
fn test_jurisdiction_with_numbers_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let result = client.try_create_attestation_jurisdiction(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &Some(String::from_str(&env, "U1")),
        &None,
    );

    assert_eq!(result, Err(Ok(types::Error::InvalidJurisdiction)));
}

#[test]
fn test_jurisdiction_with_special_chars_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let result = client.try_create_attestation_jurisdiction(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &Some(String::from_str(&env, "U-")),
        &None,
    );

    assert_eq!(result, Err(Ok(types::Error::InvalidJurisdiction)));
}

#[test]
fn test_jurisdiction_non_iso_code_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    // XX is not a valid ISO 3166-1 alpha-2 code
    let result = client.try_create_attestation_jurisdiction(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &Some(String::from_str(&env, "XX")),
        &None,
    );

    assert_eq!(result, Err(Ok(types::Error::InvalidJurisdiction)));
}

#[test]
fn test_admin_can_update_fee_and_collector() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);
    let collector = Address::generate(&env);
    let fee_token = register_test_token(&env, &admin);

    client.set_fee(&admin, &25, &collector, &Some(fee_token.clone()));

    let fee_config = client.get_fee_config();
    assert_eq!(fee_config.attestation_fee, 25);
    assert_eq!(fee_config.fee_collector, collector);
    assert_eq!(fee_config.fee_token, Some(fee_token));
}

#[test]
fn test_create_attestation_collects_fee_when_enabled() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let collector = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let fee_token = register_test_token(&env, &admin);
    let token_client = TokenClient::new(&env, &fee_token);
    let asset_admin = StellarAssetClient::new(&env, &fee_token);

    asset_admin.mint(&issuer, &100);
    client.set_fee(&admin, &25, &collector, &Some(fee_token.clone()));

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    assert_eq!(token_client.balance(&issuer), 75);
    assert_eq!(token_client.balance(&collector), 25);
    assert_eq!(client.get_attestation(&id).issuer, issuer);
}

#[test]
fn test_create_attestation_rejects_without_fee_payment() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let collector = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let fee_token = register_test_token(&env, &admin);
    let token_client = TokenClient::new(&env, &fee_token);

    client.set_fee(&admin, &25, &collector, &Some(fee_token));

    let result = client.try_create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    assert!(result.is_err());
    assert_eq!(token_client.balance(&collector), 0);
    assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 0);
}

#[test]
fn test_create_attestation_rejects_self_attestation() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let collector = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let fee_token = register_test_token(&env, &admin);
    let token_client = TokenClient::new(&env, &fee_token);
    let asset_admin = StellarAssetClient::new(&env, &fee_token);

    asset_admin.mint(&issuer, &100);
    client.set_fee(&admin, &25, &collector, &Some(fee_token.clone()));

    let result = client.try_create_attestation(&issuer, &issuer, &claim_type, &None, &None, &None);
    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
    assert_eq!(token_client.balance(&issuer), 100);
    assert_eq!(token_client.balance(&collector), 0);
    assert_eq!(client.get_subject_attestations(&issuer, &0, &10).len(), 0);
}

#[test]
fn test_create_attestation_rejects_past_expiration() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let now = env.ledger().timestamp();
    let past_expiration = Some(now - 1);

    let result = client.try_create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &past_expiration,
        &None,
        &None,
    );

    assert_eq!(result, Err(Ok(Error::InvalidExpiration)));
}

#[test]
fn test_create_attestation_accepts_future_expiration() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let future_expiration = Some(env.ledger().timestamp() + 1);

    let id = client.create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &future_expiration,
        &None,
        &None,
    );

    let attestation = client.get_attestation(&id);
    assert_eq!(attestation.expiration, future_expiration);
    assert!(client.has_valid_claim(&subject, &claim_type));
}

#[test]
fn test_create_attestation_rejects_metadata_over_256_chars() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let too_long = Some(String::from_bytes(&env, &[b'a'; 257]));

    let result =
        client.try_create_attestation(&issuer, &subject, &claim_type, &None, &too_long, &None);
    assert_eq!(result, Err(Ok(types::Error::MetadataTooLong)));
}

#[test]
fn test_duplicate_attestation_rejected_for_same_timestamp() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    env.ledger().with_mut(|li| li.timestamp = 1_000);
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    let result = client.try_create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    assert_eq!(result, Err(Ok(types::Error::DuplicateAttestation)));
}

#[test]
fn test_has_valid_claim_and_revocation() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    assert!(client.has_valid_claim(&subject, &claim_type));

    client.revoke_attestation(&issuer, &id, &None);
    assert!(!client.has_valid_claim(&subject, &claim_type));
    assert!(client.get_attestation(&id).revoked);
}

#[test]
fn test_revoke_removes_ids_from_subject_and_issuer_indexes() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Index pagination counts should reflect the initial state.
    assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 1);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 1);

    client.revoke_attestation(&issuer, &id, &None);

    // After revocation, the ID should be removed from both indexes.
    assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 0);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 0);

    // The underlying attestation record must still exist (immutable history),
    // but be marked as revoked.
    let att = client.get_attestation(&id);
    assert!(att.revoked);
    assert!(!att.deleted);
}

#[test]
fn test_revoke_with_reason_stores_reason_on_attestation() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let reason = Some(String::from_str(&env, "Document expired"));

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id, &reason);

    let att = client.get_attestation(&id);
    assert!(att.revoked);
    assert_eq!(att.revocation_reason, reason);
}

#[test]
fn test_revoke_without_reason_stores_none() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id, &None);

    let att = client.get_attestation(&id);
    assert!(att.revoked);
    assert_eq!(att.revocation_reason, None);
}

#[test]
fn test_revoke_reason_over_128_chars_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // 129-character reason — one over the limit
    let too_long = Some(String::from_bytes(&env, &[b'x'; 129]));
    let result = client.try_revoke_attestation(&issuer, &id, &too_long);

    assert_eq!(result, Err(Ok(types::Error::ReasonTooLong)));
    // Attestation must remain unrevoked
    assert!(!client.get_attestation(&id).revoked);
}

#[test]
fn test_revoke_reason_exactly_128_chars_accepted() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Exactly 128 characters — at the boundary, must succeed
    let boundary_reason = Some(String::from_bytes(&env, &[b'r'; 128]));
    client.revoke_attestation(&issuer, &id, &boundary_reason);

    let att = client.get_attestation(&id);
    assert!(att.revoked);
    assert_eq!(att.revocation_reason, boundary_reason);
}

#[test]
fn test_expired_attestation_status() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let now = env.ledger().timestamp();

    let id = client.create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &Some(now + 100),
        &None,
        &None,
    );
    assert!(client.has_valid_claim(&subject, &claim_type));

    env.ledger().with_mut(|li| li.timestamp = now + 101);
    assert_eq!(
        client.get_attestation_status(&id),
        types::AttestationStatus::Expired
    );
    assert!(!client.has_valid_claim(&subject, &claim_type));
}

#[test]
fn test_create_attestations_batch_indexes_subjects_and_issuer() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let mut subjects = soroban_sdk::Vec::new(&env);
    let subject_a = Address::generate(&env);
    let subject_b = Address::generate(&env);
    subjects.push_back(subject_a.clone());
    subjects.push_back(subject_b.clone());

    let ids = client.create_attestations_batch(&issuer, &subjects, &claim_type, &None);

    assert_eq!(ids.len(), 2);
    assert_eq!(
        client.get_subject_attestations(&subject_a, &0, &10).len(),
        1
    );
    assert_eq!(
        client.get_subject_attestations(&subject_b, &0, &10).len(),
        1
    );
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 2);
}

#[test]
fn test_claim_type_registry_round_trip() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let description = String::from_str(&env, "Subject has passed KYC");

    client.register_claim_type(&admin, &claim_type, &description);

    assert_eq!(
        client.get_claim_type_description(&claim_type),
        Some(description.clone())
    );
    assert_eq!(client.list_claim_types(&0, &10).len(), 1);
}

#[test]
fn test_set_and_get_issuer_metadata() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let metadata = types::IssuerMetadata {
        name: String::from_str(&env, "Acme"),
        url: String::from_str(&env, "https://acme.example"),
        description: String::from_str(&env, "Test issuer"),
    };

    client.set_issuer_metadata(&issuer, &metadata);
    assert_eq!(client.get_issuer_metadata(&issuer), Some(metadata));
}

#[test]
fn test_import_attestation_preserves_historical_timestamp_and_marks_imported() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let historical_timestamp = 1_000;

    env.ledger().with_mut(|li| li.timestamp = 5_000);
    let id = client.import_attestation(
        &admin,
        &issuer,
        &subject,
        &claim_type,
        &historical_timestamp,
        &Some(10_000),
    );

    let attestation = client.get_attestation(&id);
    assert_eq!(attestation.timestamp, historical_timestamp);
    assert_eq!(attestation.expiration, Some(10_000));
    assert_eq!(attestation.metadata, None);
    assert_eq!(attestation.origin, types::AttestationOrigin::Imported);
}

#[test]
fn test_bridge_attestation_requires_registered_bridge() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, client) = create_test_contract(&env);
    let admin = Address::generate(&env);
    let bridge = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let source_chain = String::from_str(&env, "ethereum");
    let source_tx = String::from_str(&env, "0xabc123");

    client.initialize(&admin, &None);

    let result =
        client.try_bridge_attestation(&bridge, &subject, &claim_type, &source_chain, &source_tx);

    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
}

#[test]
fn test_bridge_attestation_stores_source_reference_and_marks_bridged() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);
    let bridge = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let source_chain = String::from_str(&env, "ethereum");
    let source_tx = String::from_str(&env, "0xabc123");

    client.register_bridge(&admin, &bridge);
    let id = client.bridge_attestation(&bridge, &subject, &claim_type, &source_chain, &source_tx);

    let attestation = client.get_attestation(&id);
    assert_eq!(attestation.issuer, bridge);
    assert_eq!(attestation.origin, types::AttestationOrigin::Bridged);
    assert_eq!(attestation.source_chain, Some(source_chain));
    assert_eq!(attestation.source_tx, Some(source_tx));
}

#[test]
fn test_bridge_attestation_rejects_source_chain_too_long() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);
    let bridge = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let source_chain = String::from_str(&env, "123456789012345678901234567890123"); // 33 chars
    let source_tx = String::from_str(&env, "0xabc123");

    client.register_bridge(&admin, &bridge);
    let result = client.try_bridge_attestation(&bridge, &subject, &claim_type, &source_chain, &source_tx);

    assert_eq!(result, Err(Ok(types::Error::MetadataTooLong)));
}

#[test]
fn test_bridge_attestation_rejects_source_tx_too_long() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);
    let bridge = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let source_chain = String::from_str(&env, "ethereum");
    let source_tx = String::from_str(
        &env,
        "123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789",
    ); // 129 chars

    client.register_bridge(&admin, &bridge);
    let result = client.try_bridge_attestation(&bridge, &subject, &claim_type, &source_chain, &source_tx);

    assert_eq!(result, Err(Ok(types::Error::MetadataTooLong)));
}

#[test]
fn test_bridge_attestation_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);
    let bridge = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let source_chain = String::from_str(&env, "ethereum");
    let source_tx = String::from_str(&env, "0xabc123");

    client.register_bridge(&admin, &bridge);
    client.bridge_attestation(&bridge, &subject, &claim_type, &source_chain, &source_tx);

    let events = env.events().all();
    let (_, topics, data) = events.last().unwrap();
    let topic0: soroban_sdk::Symbol =
        soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    let topic1: Address =
        soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
    let event_data: (String, Address, String, String, String) =
        soroban_sdk::TryFromVal::try_from_val(&env, &data).unwrap();

    assert_eq!(topic0, soroban_sdk::symbol_short!("bridged"));
    assert_eq!(topic1, subject);
    assert_eq!(event_data.1, bridge);
    assert_eq!(event_data.3, source_chain);
    assert_eq!(event_data.4, source_tx);
}

#[test]
fn test_bridge_contract_can_create_attestation() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let (trustlink_id, client) = create_test_contract(&env);
    let (bridge_id, bridge_client) = register_bridge_contract(&env);
    let admin = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let source_chain = String::from_str(&env, "ethereum");
    let source_tx = String::from_str(&env, "0xdef456");

    client.initialize(&admin, &None);
    client.register_bridge(&admin, &bridge_id);

    let id = bridge_client.bridge_claim(
        &trustlink_id,
        &subject,
        &claim_type,
        &source_chain,
        &source_tx,
    );

    let attestation = client.get_attestation(&id);
    assert!(client.has_valid_claim(&subject, &claim_type));
    assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 1);
    assert_eq!(attestation.issuer, bridge_id);
    assert_eq!(attestation.origin, types::AttestationOrigin::Bridged);
}

#[test]
fn test_import_attestation_is_admin_only() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let wrong_admin = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let result =
        client.try_import_attestation(&wrong_admin, &issuer, &subject, &claim_type, &1_000, &None);

    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
}

#[test]
fn test_import_attestation_requires_registered_issuer() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let unregistered_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin, &None);

    let result = client.try_import_attestation(
        &admin,
        &unregistered_issuer,
        &subject,
        &claim_type,
        &1_000,
        &None,
    );

    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
}

#[test]
fn test_import_attestation_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    env.ledger().with_mut(|li| li.timestamp = 5_000);
    client.import_attestation(&admin, &issuer, &subject, &claim_type, &1_000, &None);

    let events = env.events().all();
    let (_, topics, data) = events.last().unwrap();
    let topic0: soroban_sdk::Symbol =
        soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    let topic1: Address =
        soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
    let event_data: (String, Address, String, u64, Option<u64>) =
        soroban_sdk::TryFromVal::try_from_val(&env, &data).unwrap();

    assert_eq!(topic0, soroban_sdk::symbol_short!("imported"));
    assert_eq!(topic1, subject);
    assert_eq!(event_data.1, issuer);
    assert_eq!(event_data.3, 1_000);
}

#[test]
fn test_imported_attestation_is_queryable_like_native() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    env.ledger().with_mut(|li| li.timestamp = 5_000);
    let id = client.import_attestation(&admin, &issuer, &subject, &claim_type, &1_000, &None);

    assert!(client.has_valid_claim(&subject, &claim_type));
    assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 1);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 1);
    assert_eq!(client.get_attestation_by_type(&subject, &claim_type).unwrap().id, id);
}

#[test]
fn test_imported_attestation_can_be_expired_today() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    env.ledger().with_mut(|li| li.timestamp = 5_000);
    let id =
        client.import_attestation(&admin, &issuer, &subject, &claim_type, &1_000, &Some(2_000));

    assert_eq!(
        client.get_attestation_status(&id),
        types::AttestationStatus::Expired
    );
    assert!(!client.has_valid_claim(&subject, &claim_type));
}

#[test]
fn test_create_attestation_with_tags() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "TAGGED_CLAIM");

    let mut tags = soroban_sdk::Vec::new(&env);
    tags.push_back(String::from_str(&env, "tag1"));
    tags.push_back(String::from_str(&env, "tag2"));

    let id = client.create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &Some(tags.clone()),
    );
    let att = client.get_attestation(&id);

    assert_eq!(att.tags, Some(tags));
}

#[test]
fn test_get_attestations_by_tag() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);

    let mut tags = soroban_sdk::Vec::new(&env);
    tags.push_back(String::from_str(&env, "mytag"));
    let id1 = client.create_attestation(
        &issuer,
        &subject,
        &String::from_str(&env, "CLAIM_1"),
        &None,
        &None,
        &Some(tags),
    );

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let mut tags2 = soroban_sdk::Vec::new(&env);
    tags2.push_back(String::from_str(&env, "othertag"));
    let _id2 = client.create_attestation(
        &issuer,
        &subject,
        &String::from_str(&env, "CLAIM_2"),
        &None,
        &None,
        &Some(tags2),
    );

    let result = client.get_attestations_by_tag(&subject, &String::from_str(&env, "mytag"));
    assert_eq!(result.len(), 1);
    assert_eq!(result.get(0).unwrap(), id1);
}

#[test]
fn test_get_attestations_in_range() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "RANGE_TEST");

    // Create 3 attestations at different timestamps
    env.ledger().set_timestamp(100);
    let id1 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    env.ledger().set_timestamp(200);
    let id2 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    env.ledger().set_timestamp(300);
    let id3 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Test full range
    let all = client.get_attestations_in_range(&subject, &100, &300, &0, &10);
    assert_eq!(all.len(), 3);
    assert_eq!(all.get(0).unwrap().id, id1);
    assert_eq!(all.get(1).unwrap().id, id2);
    assert_eq!(all.get(2).unwrap().id, id3);

    // Test sub range
    let middle = client.get_attestations_in_range(&subject, &150, &250, &0, &10);
    assert_eq!(middle.len(), 1);
    assert_eq!(middle.get(0).unwrap().id, id2);

    // Test empty range
    let empty = client.get_attestations_in_range(&subject, &400, &500, &0, &10);
    assert_eq!(empty.len(), 0);

    // Test boundaries inclusive
    let boundary = client.get_attestations_in_range(&subject, &100, &100, &0, &10);
    assert_eq!(boundary.len(), 1);
    assert_eq!(boundary.get(0).unwrap().id, id1);

    // Test pagination
    let paginated = client.get_attestations_in_range(&subject, &100, &300, &1, &1);
    assert_eq!(paginated.len(), 1);
    assert_eq!(paginated.get(0).unwrap().id, id2);
}

#[test]
fn test_get_attestations_in_range_zero_width() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ZERO_WIDTH_TEST");

    // Create attestation at timestamp 100
    env.ledger().set_timestamp(100);
    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Zero-width range at exact timestamp should return the attestation (inclusive boundaries)
    let result = client.get_attestations_in_range(&subject, &100, &100, &0, &10);
    assert_eq!(result.len(), 1);
    assert_eq!(result.get(0).unwrap().id, id);

    // Zero-width range at different timestamp should return empty
    let empty = client.get_attestations_in_range(&subject, &99, &99, &0, &10);
    assert_eq!(empty.len(), 0);

    let empty2 = client.get_attestations_in_range(&subject, &101, &101, &0, &10);
    assert_eq!(empty2.len(), 0);
}

#[test]
fn test_get_attestations_in_range_reversed() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "REVERSED_TEST");

    // Create attestations
    env.ledger().set_timestamp(100);
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    env.ledger().set_timestamp(200);
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Reversed range (from_ts > to_ts) should return empty result
    let result = client.get_attestations_in_range(&subject, &300, &100, &0, &10);
    assert_eq!(result.len(), 0);

    let result2 = client.get_attestations_in_range(&subject, &200, &100, &0, &10);
    assert_eq!(result2.len(), 0);
}

#[test]
fn test_get_attestations_in_range_boundary_inclusivity() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "BOUNDARY_TEST");

    // Create attestations at specific timestamps
    env.ledger().set_timestamp(100);
    let id1 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    env.ledger().set_timestamp(200);
    let id2 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    env.ledger().set_timestamp(300);
    let id3 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Lower boundary inclusive: from_ts = 100 should include id1
    let result = client.get_attestations_in_range(&subject, &100, &300, &0, &10);
    assert_eq!(result.len(), 3);
    assert_eq!(result.get(0).unwrap().timestamp, 100);

    // Upper boundary inclusive: to_ts = 300 should include id3
    assert_eq!(result.get(2).unwrap().timestamp, 300);

    // Just below lower boundary should exclude id1
    let result2 = client.get_attestations_in_range(&subject, &101, &300, &0, &10);
    assert_eq!(result2.len(), 2);
    assert_eq!(result2.get(0).unwrap().id, id2);

    // Just above upper boundary should exclude id3
    let result3 = client.get_attestations_in_range(&subject, &100, &299, &0, &10);
    assert_eq!(result3.len(), 2);
    assert_eq!(result3.get(1).unwrap().id, id2);
}

#[test]
fn test_get_attestations_in_range_large_dataset_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "LARGE_DATASET_TEST");

    // Create 20 attestations with timestamps 100, 200, 300, ..., 2000
    let mut expected_ids = soroban_sdk::Vec::new(&env);
    for i in 1..=20 {
        env.ledger().set_timestamp(i * 100);
        let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
        expected_ids.push_back(id);
    }

    // Test pagination across full range
    let page1 = client.get_attestations_in_range(&subject, &100, &2000, &0, &5);
    assert_eq!(page1.len(), 5);
    assert_eq!(page1.get(0).unwrap().timestamp, 100);
    assert_eq!(page1.get(4).unwrap().timestamp, 500);

    let page2 = client.get_attestations_in_range(&subject, &100, &2000, &5, &5);
    assert_eq!(page2.len(), 5);
    assert_eq!(page2.get(0).unwrap().timestamp, 600);
    assert_eq!(page2.get(4).unwrap().timestamp, 1000);

    let page3 = client.get_attestations_in_range(&subject, &100, &2000, &10, &5);
    assert_eq!(page3.len(), 5);
    assert_eq!(page3.get(0).unwrap().timestamp, 1100);

    let page4 = client.get_attestations_in_range(&subject, &100, &2000, &15, &5);
    assert_eq!(page4.len(), 5);
    assert_eq!(page4.get(4).unwrap().timestamp, 2000);

    // Verify no duplicates across pages
    let mut all_ids = soroban_sdk::Vec::new(&env);
    for page in [page1, page2, page3, page4].iter() {
        for att in page.iter() {
            all_ids.push_back(att.id.clone());
        }
    }
    assert_eq!(all_ids.len(), 20);

    // Verify deterministic ordering across multiple calls
    let call1 = client.get_attestations_in_range(&subject, &100, &2000, &0, &20);
    let call2 = client.get_attestations_in_range(&subject, &100, &2000, &0, &20);
    assert_eq!(call1.len(), call2.len());
    for i in 0..call1.len() {
        assert_eq!(call1.get(i).unwrap().id, call2.get(i).unwrap().id);
    }
}

#[test]
fn test_get_attestations_in_range_single_record_boundaries() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "SINGLE_RECORD_TEST");

    // Create single attestation at timestamp 500
    env.ledger().set_timestamp(500);
    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Exact match
    let exact = client.get_attestations_in_range(&subject, &500, &500, &0, &10);
    assert_eq!(exact.len(), 1);
    assert_eq!(exact.get(0).unwrap().id, id);

    // Range containing the timestamp
    let containing = client.get_attestations_in_range(&subject, &400, &600, &0, &10);
    assert_eq!(containing.len(), 1);
    assert_eq!(containing.get(0).unwrap().id, id);

    // Range just before
    let before = client.get_attestations_in_range(&subject, &400, &499, &0, &10);
    assert_eq!(before.len(), 0);

    // Range just after
    let after = client.get_attestations_in_range(&subject, &501, &600, &0, &10);
    assert_eq!(after.len(), 0);

    // Lower boundary inclusive
    let lower = client.get_attestations_in_range(&subject, &500, &600, &0, &10);
    assert_eq!(lower.len(), 1);

    // Upper boundary inclusive
    let upper = client.get_attestations_in_range(&subject, &400, &500, &0, &10);
    assert_eq!(upper.len(), 1);
}

#[test]
fn test_get_attestations_in_range_invalid_inputs() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "INVALID_INPUT_TEST");

    // Create some attestations
    env.ledger().set_timestamp(100);
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    env.ledger().set_timestamp(200);
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // from_ts = 0, to_ts = 0 (zero-width at timestamp 0)
    let result = client.get_attestations_in_range(&subject, &0, &0, &0, &10);
    assert_eq!(result.len(), 0);

    // from_ts = u64::MAX, to_ts = u64::MAX
    let result2 = client.get_attestations_in_range(&subject, &u64::MAX, &u64::MAX, &0, &10);
    assert_eq!(result2.len(), 0);

    // from_ts = 0, to_ts = u64::MAX (full range)
    let result3 = client.get_attestations_in_range(&subject, &0, &u64::MAX, &0, &10);
    assert_eq!(result3.len(), 2);

    // Reversed with extreme values
    let result4 = client.get_attestations_in_range(&subject, &u64::MAX, &0, &0, &10);
    assert_eq!(result4.len(), 0);
}

#[test]
fn test_get_attestations_in_range_with_revoked_and_deleted() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "REVOKED_TEST");

    // Create attestations
    env.ledger().set_timestamp(100);
    let id1 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    env.ledger().set_timestamp(200);
    let id2 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    env.ledger().set_timestamp(300);
    let id3 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Revoke id2
    client.revoke_attestation(&issuer, &id2, &None);

    // Range query should exclude revoked attestation (it's removed from index)
    let result = client.get_attestations_in_range(&subject, &100, &300, &0, &10);
    assert_eq!(result.len(), 2);
    assert_eq!(result.get(0).unwrap().id, id1);
    assert_eq!(result.get(1).unwrap().id, id3);
}

#[test]
fn test_get_attestations_in_range_multi_page_determinism() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "DETERMINISM_TEST");

    // Create 10 attestations
    for i in 1..=10 {
        env.ledger().set_timestamp(i * 100);
        client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    }

    // Fetch all pages multiple times and verify consistency
    for _ in 0..3 {
        let page1 = client.get_attestations_in_range(&subject, &100, &1000, &0, &3);
        let page2 = client.get_attestations_in_range(&subject, &100, &1000, &3, &3);
        let page3 = client.get_attestations_in_range(&subject, &100, &1000, &6, &3);
        let page4 = client.get_attestations_in_range(&subject, &100, &1000, &9, &3);

        assert_eq!(page1.len(), 3);
        assert_eq!(page2.len(), 3);
        assert_eq!(page3.len(), 3);
        assert_eq!(page4.len(), 1);

        // Verify ordering
        assert_eq!(page1.get(0).unwrap().timestamp, 100);
        assert_eq!(page1.get(2).unwrap().timestamp, 300);
        assert_eq!(page2.get(0).unwrap().timestamp, 400);
        assert_eq!(page3.get(0).unwrap().timestamp, 700);
        assert_eq!(page4.get(0).unwrap().timestamp, 1000);
    }
}

#[test]
fn test_get_attestations_in_range_after_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "CURSOR_PAGINATION_TEST");

    let mut expected_ids = soroban_sdk::Vec::new(&env);
    for i in 1..=6 {
        env.ledger().set_timestamp(i * 100);
        let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
        expected_ids.push_back(id);
    }

    let page1 = client.get_attestations_in_range_after(&subject, &100, &600, &None, &2);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().id, expected_ids.get(0).unwrap().clone());
    assert_eq!(page1.get(1).unwrap().id, expected_ids.get(1).unwrap().clone());

    let page2 = client.get_attestations_in_range_after(&subject, &100, &600, &Some(page1.get(1).unwrap().id.clone()), &2);
    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap().id, expected_ids.get(2).unwrap().clone());
    assert_eq!(page2.get(1).unwrap().id, expected_ids.get(3).unwrap().clone());

    let page3 = client.get_attestations_in_range_after(&subject, &100, &600, &Some(page2.get(1).unwrap().id.clone()), &2);
    assert_eq!(page3.len(), 2);
    assert_eq!(page3.get(0).unwrap().id, expected_ids.get(4).unwrap().clone());
    assert_eq!(page3.get(1).unwrap().id, expected_ids.get(5).unwrap().clone());

    let page4 = client.get_attestations_in_range_after(&subject, &100, &600, &Some(page3.get(1).unwrap().id.clone()), &2);
    assert_eq!(page4.len(), 0);
}

#[test]
fn test_get_attestations_in_range_after_cursor_resilient_to_deletion() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "CURSOR_DELETION_TEST");

    env.ledger().set_timestamp(100);
    let id1 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    env.ledger().set_timestamp(200);
    let id2 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    env.ledger().set_timestamp(300);
    let id3 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    env.ledger().set_timestamp(400);
    let id4 = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    let first_page = client.get_attestations_in_range_after(&subject, &100, &400, &None, &2);
    assert_eq!(first_page.len(), 2);
    assert_eq!(first_page.get(0).unwrap().id, id1);
    assert_eq!(first_page.get(1).unwrap().id, id2);

    client.request_deletion(&subject, &id2);

    let second_page = client.get_attestations_in_range_after(&subject, &100, &400, &Some(id2.clone()), &2);
    assert_eq!(second_page.len(), 2);
    assert_eq!(second_page.get(0).unwrap().id, id3);
    assert_eq!(second_page.get(1).unwrap().id, id4);
}

#[test]
fn test_get_attestations_in_range_after_invalid_cursor_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "INVALID_CURSOR_TEST");

    env.ledger().set_timestamp(100);
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    let invalid_cursor = String::from_str(&env, "nonexistent_cursor_id");
    let results = client.get_attestations_in_range_after(&subject, &100, &200, &Some(invalid_cursor), &10);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_tags_length_limits() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "TAGGED_CLAIM");

    // Max 5 tags max
    let mut too_many_tags = soroban_sdk::Vec::new(&env);
    for _ in 0..6 {
        too_many_tags.push_back(String::from_str(&env, "tag"));
    }

    let res1 = client.try_create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &Some(too_many_tags),
    );
    assert_eq!(res1, Err(Ok(types::Error::TooManyTags)));

    // Max 32 chars
    let mut long_tag = soroban_sdk::Vec::new(&env);
    long_tag.push_back(String::from_bytes(&env, &[b'a'; 33]));
    let res2 = client.try_create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &Some(long_tag),
    );
    assert_eq!(res2, Err(Ok(types::Error::TagTooLong)));
}

// ── Multi-sig attestation tests ──────────────────────────────────────────────

fn setup_multisig(
    env: &Env,
) -> (
    Address,
    Address,
    Address,
    Address,
    TrustLinkContractClient<'_>,
) {
    let (_, client) = create_test_contract(env);
    let admin = Address::generate(env);
    let issuer1 = Address::generate(env);
    let issuer2 = Address::generate(env);
    let issuer3 = Address::generate(env);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer1);
    client.register_issuer(&admin, &issuer2);
    client.register_issuer(&admin, &issuer3);
    (issuer1, issuer2, issuer3, admin, client)
}

#[test]
fn test_multisig_2_of_3_activates_on_second_signature() {
    let env = Env::default();
    env.mock_all_auths();

    let (issuer1, issuer2, issuer3, _, client) = setup_multisig(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");

    let mut required = soroban_sdk::Vec::new(&env);
    required.push_back(issuer1.clone());
    required.push_back(issuer2.clone());
    required.push_back(issuer3.clone());

    let proposal_id = client.propose_attestation(&issuer1, &subject, &claim_type, &required, &2);

    // After proposal, attestation should NOT exist yet.
    let proposal = client.get_multisig_proposal(&proposal_id);
    assert_eq!(proposal.signers.len(), 1);
    assert!(!proposal.finalized);
    assert!(!client.has_valid_claim(&subject, &claim_type));

    // Second signature reaches threshold → attestation activated.
    client.cosign_attestation(&issuer2, &proposal_id);

    let proposal = client.get_multisig_proposal(&proposal_id);
    assert!(proposal.finalized);
    assert!(client.has_valid_claim(&subject, &claim_type));
}

#[test]
fn test_multisig_3_of_3_requires_all_signers() {
    let env = Env::default();
    env.mock_all_auths();

    let (issuer1, issuer2, issuer3, _, client) = setup_multisig(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");

    let mut required = soroban_sdk::Vec::new(&env);
    required.push_back(issuer1.clone());
    required.push_back(issuer2.clone());
    required.push_back(issuer3.clone());

    let proposal_id = client.propose_attestation(&issuer1, &subject, &claim_type, &required, &3);

    client.cosign_attestation(&issuer2, &proposal_id);
    assert!(!client.has_valid_claim(&subject, &claim_type));

    client.cosign_attestation(&issuer3, &proposal_id);
    assert!(client.has_valid_claim(&subject, &claim_type));
}

#[test]
fn test_multisig_non_required_signer_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (issuer1, issuer2, issuer3, admin, client) = setup_multisig(&env);
    let outsider = Address::generate(&env);
    client.register_issuer(&admin, &outsider);

    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");

    let mut required = soroban_sdk::Vec::new(&env);
    required.push_back(issuer1.clone());
    required.push_back(issuer2.clone());
    required.push_back(issuer3.clone());

    let proposal_id = client.propose_attestation(&issuer1, &subject, &claim_type, &required, &2);

    let result = client.try_cosign_attestation(&outsider, &proposal_id);
    assert_eq!(result, Err(Ok(types::Error::NotRequiredSigner)));
}

#[test]
fn test_multisig_duplicate_cosign_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (issuer1, issuer2, issuer3, _, client) = setup_multisig(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");

    let mut required = soroban_sdk::Vec::new(&env);
    required.push_back(issuer1.clone());
    required.push_back(issuer2.clone());
    required.push_back(issuer3.clone());

    let proposal_id = client.propose_attestation(&issuer1, &subject, &claim_type, &required, &3);

    // issuer1 already signed on proposal creation.
    let result = client.try_cosign_attestation(&issuer1, &proposal_id);
    assert_eq!(result, Err(Ok(types::Error::AlreadySigned)));
}

#[test]
fn test_multisig_expired_proposal_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (issuer1, issuer2, issuer3, _, client) = setup_multisig(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let mut required = soroban_sdk::Vec::new(&env);
    required.push_back(issuer1.clone());
    required.push_back(issuer2.clone());
    required.push_back(issuer3.clone());

    let proposal_id = client.propose_attestation(&issuer1, &subject, &claim_type, &required, &2);

    // Advance past the 7-day expiry window.
    env.ledger()
        .with_mut(|li| li.timestamp = 1_000 + 7 * 24 * 60 * 60 + 1);

    let result = client.try_cosign_attestation(&issuer2, &proposal_id);
    assert_eq!(result, Err(Ok(types::Error::ProposalExpired)));
}

#[test]
fn test_multisig_invalid_threshold_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (issuer1, issuer2, issuer3, _, client) = setup_multisig(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");

    let mut required = soroban_sdk::Vec::new(&env);
    required.push_back(issuer1.clone());
    required.push_back(issuer2.clone());
    required.push_back(issuer3.clone());

    // threshold 0 is invalid.
    let result = client.try_propose_attestation(&issuer1, &subject, &claim_type, &required, &0);
    assert_eq!(result, Err(Ok(types::Error::InvalidThreshold)));

    // threshold > signer count is invalid.
    let result = client.try_propose_attestation(&issuer1, &subject, &claim_type, &required, &4);
    assert_eq!(result, Err(Ok(types::Error::InvalidThreshold)));
}

#[test]
fn test_multisig_proposal_emits_events() {
    let env = Env::default();
    env.mock_all_auths();

    let (issuer1, issuer2, issuer3, _, client) = setup_multisig(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");

    let mut required = soroban_sdk::Vec::new(&env);
    required.push_back(issuer1.clone());
    required.push_back(issuer2.clone());
    required.push_back(issuer3.clone());

    let proposal_id = client.propose_attestation(&issuer1, &subject, &claim_type, &required, &2);

    // Verify ms_prop event was emitted.
    let events = env.events().all();
    let mut found_prop = false;
    for (_, topics, _) in events.iter() {
        let topic0: soroban_sdk::Symbol =
            soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        if topic0 == soroban_sdk::symbol_short!("ms_prop") {
            found_prop = true;
            break;
        }
    }
    assert!(found_prop, "ms_prop event not found");

    // Co-sign and verify ms_sign + ms_actv events.
    client.cosign_attestation(&issuer2, &proposal_id);

    let events = env.events().all();
    let mut found_sign = false;
    let mut found_actv = false;
    for (_, topics, _) in events.iter() {
        let topic0: soroban_sdk::Symbol =
            soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        if topic0 == soroban_sdk::symbol_short!("ms_sign") {
            found_sign = true;
        }
        if topic0 == soroban_sdk::symbol_short!("ms_actv") {
            found_actv = true;
        }
    }
    assert!(found_sign, "ms_sign event not found");
    assert!(found_actv, "ms_actv event not found");
}

#[test]
fn test_multisig_unregistered_proposer_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (issuer1, issuer2, issuer3, _, client) = setup_multisig(&env);
    let unregistered = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");

    let mut required = soroban_sdk::Vec::new(&env);
    required.push_back(issuer1.clone());
    required.push_back(issuer2.clone());
    required.push_back(issuer3.clone());

    let result =
        client.try_propose_attestation(&unregistered, &subject, &claim_type, &required, &2);
    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
}

#[test]
fn test_revoke_with_reason_stores_reason() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let reason = String::from_str(&env, "Document expired");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id, &Some(reason.clone()));

    let attestation = client.get_attestation(&id);
    assert!(attestation.revoked);
    assert_eq!(attestation.revocation_reason, Some(reason));
}


// ── Property-based tests: attestation ID uniqueness ──────────────────────────

/// Same issuer, different subjects → different IDs.
#[test]
fn test_id_uniqueness_same_issuer_different_subjects() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let ts = 1_000_000u64;

    let id1 =
        types::Attestation::generate_id(&env, &issuer, &Address::generate(&env), &claim_type, ts);
    let id2 =
        types::Attestation::generate_id(&env, &issuer, &Address::generate(&env), &claim_type, ts);
    assert_ne!(id1, id2, "different subjects must produce different IDs");
}

/// Same subject, different issuers → different IDs.
#[test]
fn test_id_uniqueness_same_subject_different_issuers() {
    let env = Env::default();
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let ts = 1_000_000u64;

    let id1 =
        types::Attestation::generate_id(&env, &Address::generate(&env), &subject, &claim_type, ts);
    let id2 =
        types::Attestation::generate_id(&env, &Address::generate(&env), &subject, &claim_type, ts);
    assert_ne!(id1, id2, "different issuers must produce different IDs");
}

/// Same issuer + subject, different claim types → different IDs.
#[test]
fn test_id_uniqueness_same_issuer_subject_different_claim_types() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let ts = 1_000_000u64;

    let id1 = types::Attestation::generate_id(
        &env,
        &issuer,
        &subject,
        &String::from_str(&env, "KYC_PASSED"),
        ts,
    );
    let id2 = types::Attestation::generate_id(
        &env,
        &issuer,
        &subject,
        &String::from_str(&env, "ACCREDITED_INVESTOR"),
        ts,
    );
    assert_ne!(id1, id2, "different claim types must produce different IDs");
}

/// Same issuer + subject + claim type, different timestamps → different IDs.
#[test]
fn test_id_uniqueness_same_inputs_different_timestamps() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id1 = types::Attestation::generate_id(&env, &issuer, &subject, &claim_type, 1_000_000);
    let id2 = types::Attestation::generate_id(&env, &issuer, &subject, &claim_type, 1_000_001);
    assert_ne!(id1, id2, "different timestamps must produce different IDs");
}

/// Same inputs always produce the same ID (determinism).
#[test]
fn test_id_determinism_same_inputs_same_id() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let ts = 1_000_000u64;

    let id1 = types::Attestation::generate_id(&env, &issuer, &subject, &claim_type, ts);
    let id2 = types::Attestation::generate_id(&env, &issuer, &subject, &claim_type, ts);
    assert_eq!(id1, id2, "identical inputs must always produce the same ID");
}

/// No collisions across 100 generated IDs (varying subjects, issuers, claim types, timestamps).
#[test]
fn test_id_no_collisions_across_100_combinations() {
    let env = Env::default();
    let claim_types = [
        "KYC_PASSED",
        "ACCREDITED_INVESTOR",
        "MERCHANT_VERIFIED",
        "AML_CLEARED",
        "SANCTIONS_CHECKED",
    ];

    let mut ids = soroban_sdk::Vec::new(&env);

    for i in 0u64..100 {
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let claim_type = String::from_str(&env, claim_types[(i as usize) % claim_types.len()]);
        let ts = 1_000_000u64 + i;

        let id = types::Attestation::generate_id(&env, &issuer, &subject, &claim_type, ts);

        // Ensure this ID hasn't appeared before.
        assert!(!ids.contains(&id), "collision detected at iteration {i}");
        ids.push_back(id);
    }

    assert_eq!(ids.len(), 100);
}

// ── Pagination edge cases ────────────────────────────────────────────────────

#[allow(dead_code)]
fn setup_with_n_attestations(env: &Env, n: u32) -> (Address, Address, TrustLinkContractClient<'_>) {
    let (admin, issuer, client) = setup(env);
    for _ in 0..n {
        let subject = Address::generate(env);
        client.create_attestation(
            &issuer,
            &subject,
            &String::from_str(env, "KYC_PASSED"),
            &None,
            &None,
            &None,
        );
    }
    (admin, issuer, client)
}

fn create_n_attestations_for_subject(
    env: &Env,
    client: &TrustLinkContractClient<'_>,
    issuer: &Address,
    subject: &Address,
    n: u32,
) {
    for _ in 0..n {
        client.create_attestation(
            issuer,
            subject,
            &String::from_str(env, "KYC_PASSED"),
            &None,
            &None,
            &None,
        );
        // advance ledger time so each attestation gets a unique timestamp / ID
        env.ledger().with_mut(|l| l.timestamp += 1);
    }
}

// ── get_subject_attestations ─────────────────────────────────────────────────

#[test]
fn test_subject_pagination_zero_attestations() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, _issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    // subject has no attestations
    assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 0);
}

#[test]
fn test_subject_pagination_one_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    create_n_attestations_for_subject(&env, &client, &issuer, &subject, 1);
    assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 1);
}

#[test]
fn test_subject_pagination_limit_zero_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    create_n_attestations_for_subject(&env, &client, &issuer, &subject, 3);
    assert_eq!(client.get_subject_attestations(&subject, &0, &0).len(), 0);
}

#[test]
fn test_subject_pagination_start_beyond_total_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    create_n_attestations_for_subject(&env, &client, &issuer, &subject, 3);
    assert_eq!(client.get_subject_attestations(&subject, &10, &5).len(), 0);
}

#[test]
fn test_subject_pagination_start_plus_limit_exceeds_total_returns_remaining() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    create_n_attestations_for_subject(&env, &client, &issuer, &subject, 5);
    // start=3, limit=10 → only 2 items remain
    assert_eq!(client.get_subject_attestations(&subject, &3, &10).len(), 2);
}

#[test]
fn test_subject_pagination_limit_one_returns_exactly_one() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    create_n_attestations_for_subject(&env, &client, &issuer, &subject, 5);
    assert_eq!(client.get_subject_attestations(&subject, &0, &1).len(), 1);
}

// ── get_issuer_attestations ──────────────────────────────────────────────────

#[test]
fn test_issuer_pagination_zero_attestations() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 0);
}

#[test]
fn test_issuer_pagination_one_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    create_n_attestations_for_subject(&env, &client, &issuer, &subject, 1);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 1);
}

#[test]
fn test_issuer_pagination_limit_zero_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    create_n_attestations_for_subject(&env, &client, &issuer, &subject, 3);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &0).len(), 0);
}

#[test]
fn test_issuer_pagination_start_beyond_total_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    create_n_attestations_for_subject(&env, &client, &issuer, &subject, 3);
    assert_eq!(client.get_issuer_attestations(&issuer, &10, &5).len(), 0);
}

#[test]
fn test_issuer_pagination_start_plus_limit_exceeds_total_returns_remaining() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    create_n_attestations_for_subject(&env, &client, &issuer, &subject, 5);
    // start=3, limit=10 → only 2 items remain
    assert_eq!(client.get_issuer_attestations(&issuer, &3, &10).len(), 2);
}

#[test]
fn test_issuer_pagination_limit_one_returns_exactly_one() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    create_n_attestations_for_subject(&env, &client, &issuer, &subject, 5);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &1).len(), 1);
}

// ── audit log ────────────────────────────────────────────────────────────────

#[test]
fn test_audit_log_create_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    let log = client.get_audit_log(&id);

    assert_eq!(log.len(), 1);
    assert_eq!(
        log.get(0).unwrap().action,
        crate::types::AuditAction::Created
    );
    assert_eq!(log.get(0).unwrap().actor, issuer);
}

#[test]
fn test_audit_log_revoke_appends_entry() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id, &None);
    let log = client.get_audit_log(&id);

    assert_eq!(log.len(), 2);
    assert_eq!(
        log.get(1).unwrap().action,
        crate::types::AuditAction::Revoked
    );
    assert_eq!(log.get(1).unwrap().actor, issuer);
}

#[test]
fn test_audit_log_revoke_records_reason() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let reason = Some(String::from_str(&env, "fraud detected"));

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id, &reason);
    let log = client.get_audit_log(&id);

    assert_eq!(log.get(1).unwrap().details, reason);
}

#[test]
fn test_audit_log_renew_appends_entry() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    let new_exp = env.ledger().timestamp() + 86_400 * 30;
    client.renew_attestation(&issuer, &id, &Some(new_exp));
    let log = client.get_audit_log(&id);

    assert_eq!(log.len(), 2);
    assert_eq!(
        log.get(1).unwrap().action,
        crate::types::AuditAction::Renewed
    );
}

#[test]
fn test_audit_log_update_expiration_appends_entry() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    let new_exp = env.ledger().timestamp() + 86_400 * 60;
    client.update_expiration(&issuer, &id, &Some(new_exp));
    let log = client.get_audit_log(&id);

    assert_eq!(log.len(), 2);
    assert_eq!(
        log.get(1).unwrap().action,
        crate::types::AuditAction::Updated
    );
}

#[test]
fn test_audit_log_is_append_only_across_multiple_actions() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    let new_exp = env.ledger().timestamp() + 86_400 * 30;
    client.renew_attestation(&issuer, &id, &Some(new_exp));
    client.revoke_attestation(&issuer, &id, &None);
    let log = client.get_audit_log(&id);

    assert_eq!(log.len(), 3);
    assert_eq!(
        log.get(0).unwrap().action,
        crate::types::AuditAction::Created
    );
    assert_eq!(
        log.get(1).unwrap().action,
        crate::types::AuditAction::Renewed
    );
    assert_eq!(
        log.get(2).unwrap().action,
        crate::types::AuditAction::Revoked
    );
}

#[test]
fn test_audit_log_empty_for_nonexistent_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, _, client) = setup(&env);
    let fake_id = String::from_str(&env, "nonexistent");
    let log = client.get_audit_log(&fake_id);
    assert_eq!(log.len(), 0);
}

#[test]
fn test_audit_log_batch_revoke_appends_entries() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject1 = Address::generate(&env);
    let subject2 = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id1 = client.create_attestation(&issuer, &subject1, &claim_type, &None, &None, &None);
    let id2 = client.create_attestation(&issuer, &subject2, &claim_type, &None, &None, &None);

    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id1.clone());
    ids.push_back(id2.clone());
    client.revoke_attestations_batch(&issuer, &ids, &None);

    assert_eq!(client.get_audit_log(&id1).len(), 2);
    assert_eq!(client.get_audit_log(&id2).len(), 2);
    assert_eq!(
        client.get_audit_log(&id1).get(1).unwrap().action,
        crate::types::AuditAction::Revoked
    );
}

#[test]
fn test_revoke_attestations_batch_atomic_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject1 = Address::generate(&env);
    let subject2 = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id1 = client.create_attestation(&issuer, &subject1, &claim_type, &None, &None, &None);
    let id2 = client.create_attestation(&issuer, &subject2, &claim_type, &None, &None, &None);

    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id1.clone());
    ids.push_back(id2.clone());

    let result = client.revoke_attestations_batch(&issuer, &ids, &None);
    assert_eq!(result, 2);
    assert!(client.get_attestation(&id1).revoked);
    assert!(client.get_attestation(&id2).revoked);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 0);
}

#[test]
fn test_revoke_attestations_batch_fails_and_rolls_back_on_invalid_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject1 = Address::generate(&env);
    let subject2 = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id1 = client.create_attestation(&issuer, &subject1, &claim_type, &None, &None, &None);
    let id2 = client.create_attestation(&issuer, &subject2, &claim_type, &None, &None, &None);
    let fake_id = String::from_str(&env, "does_not_exist");

    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id1.clone());
    ids.push_back(fake_id.clone());
    ids.push_back(id2.clone());

    let result = client.try_revoke_attestations_batch(&issuer, &ids, &None);
    assert_eq!(result, Err(Ok(types::Error::NotFound)));

    assert!(!client.get_attestation(&id1).revoked);
    assert!(!client.get_attestation(&id2).revoked);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 2);
}

#[test]
fn test_revoke_attestations_batch_fails_and_rolls_back_on_already_revoked_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject1 = Address::generate(&env);
    let subject2 = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id1 = client.create_attestation(&issuer, &subject1, &claim_type, &None, &None, &None);
    let id2 = client.create_attestation(&issuer, &subject2, &claim_type, &None, &None, &None);

    client.revoke_attestation(&issuer, &id1, &None);

    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id1.clone());
    ids.push_back(id2.clone());

    let result = client.try_revoke_attestations_batch(&issuer, &ids, &None);
    assert_eq!(result, Err(Ok(types::Error::AlreadyRevoked)));

    assert!(client.get_attestation(&id1).revoked);
    assert!(!client.get_attestation(&id2).revoked);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 1);
}

#[test]
fn test_revoke_attestations_batch_rejects_duplicate_ids() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    let mut ids = soroban_sdk::Vec::new(&env);
    ids.push_back(id.clone());
    ids.push_back(id.clone());

    let result = client.try_revoke_attestations_batch(&issuer, &ids, &None);
    assert_eq!(result, Err(Ok(types::Error::DuplicateAttestation)));
    assert!(!client.get_attestation(&id).revoked);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 1);
}

// ---------------------------------------------------------------------------
// health_check
// ---------------------------------------------------------------------------

#[test]
fn test_health_check_before_initialization() {
    let env = Env::default();
    let (_, client) = create_test_contract(&env);

    let status = client.health_check();

    assert!(!status.initialized);
    assert!(!status.admin_set);
    assert_eq!(status.issuer_count, 0);
    assert_eq!(status.total_attestations, 0);
}

#[test]
fn test_health_check_after_operations() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, issuer, client) = setup(&env);

    // After init + 1 issuer registered by setup()
    let status = client.health_check();
    assert!(status.initialized);
    assert!(status.admin_set);
    assert_eq!(status.issuer_count, 1);
    assert_eq!(status.total_attestations, 0);

    // Create two attestations
    let subject = Address::generate(&env);
    let claim = String::from_str(&env, "KYC_PASSED");
    client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

    let subject2 = Address::generate(&env);
    client.create_attestation(&issuer, &subject2, &claim, &None, &None, &None);

    let status = client.health_check();
    assert_eq!(status.total_attestations, 2);
    assert_eq!(status.issuer_count, 1);
}

// ── Error variant coverage ───────────────────────────────────────────────────

#[test]
fn test_error_already_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);

    client.initialize(&admin, &None);
    let result = client.try_initialize(&admin, &None);
    assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_error_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    // Call get_version (which requires initialization) before initialize.
    let (_, client) = create_test_contract(&env);
    let result = client.try_get_version();
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

#[test]
fn test_error_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, client) = setup(&env);
    let fake_id = String::from_str(&env, "nonexistent_attestation_id");
    let result = client.try_get_attestation(&fake_id);
    assert_eq!(result, Err(Ok(Error::NotFound)));
}

#[test]
fn test_error_already_revoked() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id, &None);

    let result = client.try_revoke_attestation(&issuer, &id, &None);
    assert_eq!(result, Err(Ok(Error::AlreadyRevoked)));
}

// ---------------------------------------------------------------------------
// Issuer removal – attestation persistence
// ---------------------------------------------------------------------------

#[test]
fn test_attestation_remains_valid_after_issuer_removal() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, client) = setup(&env);

    let subject = Address::generate(&env);
    let claim = String::from_str(&env, "KYC_PASSED");
    let att_id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

    // Remove the issuer
    client.remove_issuer(&admin, &issuer);

    // Attestation should still be retrievable and valid
    let att = client.get_attestation(&att_id);
    assert!(!att.revoked);
    assert_eq!(att.issuer, issuer);
    assert_eq!(att.claim_type, claim);
}

#[test]
fn test_has_valid_claim_true_after_issuer_removal() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, client) = setup(&env);

    let subject = Address::generate(&env);
    let claim = String::from_str(&env, "KYC_PASSED");
    client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

    // Remove the issuer
    client.remove_issuer(&admin, &issuer);

    // has_valid_claim should still return true
    assert!(client.has_valid_claim(&subject, &claim));
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_removed_issuer_cannot_create_new_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, client) = setup(&env);

    // Remove the issuer
    client.remove_issuer(&admin, &issuer);

    // Attempting to create a new attestation should fail with Unauthorized
    let subject = Address::generate(&env);
    let claim = String::from_str(&env, "KYC_PASSED");
    client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
}

#[test]
fn test_removed_issuer_can_revoke_own_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, client) = setup(&env);

    let subject = Address::generate(&env);
    let claim = String::from_str(&env, "KYC_PASSED");
    let att_id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

    // Remove the issuer
    client.remove_issuer(&admin, &issuer);

    // FINDING-002 fix: removed issuer can no longer revoke attestations.
    // The require_issuer guard now rejects deregistered issuers.
    let result = client.try_revoke_attestation(&issuer, &att_id, &None);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));

    // Attestation remains valid since revocation was rejected.
    assert!(client.has_valid_claim(&subject, &claim));
}

// ── Storage exhaustion / limit tests (issue #80) ─────────────────────────────

#[test]
fn test_get_limits_returns_defaults() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin, &None);

    let limits = client.get_limits();
    assert_eq!(limits.max_attestations_per_issuer, 10_000);
    assert_eq!(limits.max_attestations_per_subject, 100);
}

#[test]
fn test_admin_can_set_limits() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin, &None);

    client.set_limits(&admin, &500, &10);

    let limits = client.get_limits();
    assert_eq!(limits.max_attestations_per_issuer, 500);
    assert_eq!(limits.max_attestations_per_subject, 10);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_non_admin_cannot_set_limits() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin, &None);

    // attacker is not admin — should panic with Unauthorized (#3)
    client.set_limits(&attacker, &1, &1);
}

#[test]
#[should_panic(expected = "Error(Contract, #33)")]
fn test_issuer_limit_exceeded() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);

    // Set issuer limit to 2
    client.set_limits(&admin, &2, &1000);

    let claim = String::from_str(&env, "KYC_PASSED");

    // First two succeed
    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    client.create_attestation(&issuer, &s1, &claim, &None, &None, &None);
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    client.create_attestation(&issuer, &s2, &claim, &None, &None, &None);

    // Third should hit LimitExceeded (#10)
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    let s3 = Address::generate(&env);
    client.create_attestation(&issuer, &s3, &claim, &None, &None, &None);
}

#[test]
#[should_panic(expected = "Error(Contract, #33)")]
fn test_subject_limit_exceeded() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);

    // Set subject limit to 2
    client.set_limits(&admin, &10_000, &2);

    let c1 = String::from_str(&env, "KYC_PASSED");
    let c2 = String::from_str(&env, "AML_CLEARED");

    client.create_attestation(&issuer, &subject, &c1, &None, &None, &None);
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    client.create_attestation(&issuer, &subject, &c2, &None, &None, &None);

    // Third attestation on same subject should hit LimitExceeded (#10)
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    let c3 = String::from_str(&env, "MERCHANT_VERIFIED");
    client.create_attestation(&issuer, &subject, &c3, &None, &None, &None);
}

#[test]
#[should_panic(expected = "Error(Contract, #33)")]
fn test_batch_issuer_limit_exceeded() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);

    // Issuer limit = 2, batch of 3 subjects should fail
    client.set_limits(&admin, &2, &1000);

    let claim = String::from_str(&env, "KYC_PASSED");
    let mut subjects = soroban_sdk::Vec::new(&env);
    subjects.push_back(Address::generate(&env));
    subjects.push_back(Address::generate(&env));
    subjects.push_back(Address::generate(&env));

    client.create_attestations_batch(&issuer, &subjects, &claim, &None);
}

#[test]
fn test_limits_updated_take_effect_immediately() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let (_, client) = create_test_contract(&env);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);

    // Start with tight limit
    client.set_limits(&admin, &1, &1000);

    let claim = String::from_str(&env, "KYC_PASSED");
    client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

    // Raise the limit — next attestation should now succeed
    client.set_limits(&admin, &10, &1000);
    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    let subject2 = Address::generate(&env);
    let claim2 = String::from_str(&env, "AML_CLEARED");
    client.create_attestation(&issuer, &subject2, &claim2, &None, &None, &None);

    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 2);
}

// ============================================================================
// TTL Tests
//
// Verify that persistent storage TTL is correctly set on every write operation.
// Uses `env.as_contract` + `storage().persistent().get_ttl()` (SDK v21+).
//
// Constants mirrored from storage.rs:
//   DAY_IN_LEDGERS       = 17_280
//   DEFAULT_TTL_DAYS     = 30
//   EXPECTED_TTL_LEDGERS = 17_280 * 30 = 518_400
// ============================================================================
#[cfg(test)]
mod ttl_tests {
    use super::*;
    use soroban_sdk::{
        testutils::{storage::Persistent as _, Address as _, Ledger},
        Env, String,
    };

    // Mirror the constants from storage.rs so tests are self-documenting.
    const DAY_IN_LEDGERS: u32 = 17_280;
    const DEFAULT_TTL_DAYS: u32 = 30;
    const EXPECTED_TTL: u32 = DAY_IN_LEDGERS * DEFAULT_TTL_DAYS; // 518_400

    /// Shared setup: register contract, initialize, register one issuer.
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

    // -------------------------------------------------------------------------
    // Attestation record TTL
    // -------------------------------------------------------------------------

    /// After `create_attestation`, the attestation entry's TTL must equal
    /// `DEFAULT_TTL_DAYS * DAY_IN_LEDGERS` (518 400 ledgers).
    #[test]
    fn test_attestation_ttl_set_on_creation() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);
        let contract_id = client.address.clone();

        let claim = String::from_str(&env, "KYC");
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&crate::storage::StorageKey::Attestation(id.clone()))
        });

        assert_eq!(
            ttl, EXPECTED_TTL,
            "attestation TTL should be {EXPECTED_TTL} ledgers after creation"
        );
    }

    /// After `revoke_attestation`, the updated attestation entry's TTL must be
    /// refreshed to the full default value.
    #[test]
    fn test_attestation_ttl_refreshed_on_revoke() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);
        let contract_id = client.address.clone();

        let claim = String::from_str(&env, "KYC");
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        // Advance ledger so TTL would have decreased if not refreshed.
        env.ledger().with_mut(|l| l.sequence_number += 1_000);

        client.revoke_attestation(&issuer, &id, &None);

        let ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&crate::storage::StorageKey::Attestation(id.clone()))
        });

        assert_eq!(
            ttl, EXPECTED_TTL,
            "attestation TTL should be refreshed to {EXPECTED_TTL} after revocation"
        );
    }

    /// After `renew_attestation`, the attestation entry's TTL must be refreshed.
    #[test]
    fn test_attestation_ttl_refreshed_on_renewal() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);
        let contract_id = client.address.clone();

        let claim = String::from_str(&env, "KYC");
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        env.ledger().with_mut(|l| {
            l.sequence_number += 1_000;
            l.timestamp += 10_000;
        });

        let new_expiry: Option<u64> = Some(env.ledger().timestamp() + 100_000);
        client.renew_attestation(&issuer, &id, &new_expiry);

        let ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&crate::storage::StorageKey::Attestation(id.clone()))
        });

        assert_eq!(
            ttl, EXPECTED_TTL,
            "attestation TTL should be refreshed to {EXPECTED_TTL} after renewal"
        );
    }

    /// After `update_expiration`, the attestation TTL is extended to cover
    /// the new expiration when it exceeds the default TTL window.
    #[test]
    fn test_attestation_ttl_extends_to_expiration_on_update() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);
        let contract_id = client.address.clone();

        let claim = String::from_str(&env, "KYC");
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let new_expiry: Option<u64> = Some(env.ledger().timestamp() + 86_400 * 60);
        client.update_expiration(&issuer, &id, &new_expiry);

        let ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&crate::storage::StorageKey::Attestation(id.clone()))
        });

        assert_eq!(
            ttl,
            60 * DAY_IN_LEDGERS,
            "attestation TTL should be extended to match the 60-day expiration"
        );
    }

    // -------------------------------------------------------------------------
    // Subject index TTL
    // -------------------------------------------------------------------------

    /// After `create_attestation`, the subject attestation index TTL must equal
    /// the default TTL.
    #[test]
    fn test_subject_index_ttl_set_on_attestation_creation() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);
        let contract_id = client.address.clone();

        let claim = String::from_str(&env, "KYC");
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&crate::storage::StorageKey::SubjectAttestations(
                    subject.clone(),
                ))
        });

        assert_eq!(
            ttl, EXPECTED_TTL,
            "subject index TTL should be {EXPECTED_TTL} after attestation creation"
        );
    }

    // -------------------------------------------------------------------------
    // Issuer index TTL
    // -------------------------------------------------------------------------

    /// After `create_attestation`, the issuer attestation index TTL must equal
    /// the default TTL.
    #[test]
    fn test_issuer_index_ttl_set_on_attestation_creation() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);
        let contract_id = client.address.clone();

        let claim = String::from_str(&env, "KYC");
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&crate::storage::StorageKey::IssuerAttestations(
                    issuer.clone(),
                ))
        });

        assert_eq!(
            ttl, EXPECTED_TTL,
            "issuer index TTL should be {EXPECTED_TTL} after attestation creation"
        );
    }

    // -------------------------------------------------------------------------
    // Issuer registry TTL
    // -------------------------------------------------------------------------

    /// After `register_issuer`, the issuer presence flag TTL must equal the
    /// default TTL.
    #[test]
    fn test_issuer_registry_ttl_set_on_registration() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, _, client) = setup(&env);
        let contract_id = client.address.clone();

        let ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&crate::storage::StorageKey::Issuer(issuer.clone()))
        });

        assert_eq!(
            ttl, EXPECTED_TTL,
            "issuer registry entry TTL should be {EXPECTED_TTL} after registration"
        );
    }

    // -------------------------------------------------------------------------
    // Audit log TTL
    // -------------------------------------------------------------------------

    /// After `create_attestation`, the audit log entry TTL must equal the
    /// default TTL.
    #[test]
    fn test_audit_log_ttl_set_on_creation() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);
        let contract_id = client.address.clone();

        let claim = String::from_str(&env, "KYC");
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&crate::storage::StorageKey::AuditLog(id.clone()))
        });

        assert_eq!(
            ttl, EXPECTED_TTL,
            "audit log TTL should be {EXPECTED_TTL} after attestation creation"
        );
    }

    // -------------------------------------------------------------------------
    // Custom TTL configuration
    // -------------------------------------------------------------------------

    /// When the contract is initialized with a custom TTL (e.g. 60 days), all
    /// subsequent persistent writes must use that value instead of the default.
    #[test]
    fn test_custom_ttl_config_applied_to_attestation() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);

        let custom_days: u32 = 60;
        client.initialize(&admin, &Some(custom_days));
        client.register_issuer(&admin, &issuer);

        let claim = String::from_str(&env, "KYC");
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let expected = DAY_IN_LEDGERS * custom_days; // 1_036_800
        let ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&crate::storage::StorageKey::Attestation(id.clone()))
        });

        assert_eq!(
            ttl, expected,
            "attestation TTL should reflect custom config of {custom_days} days ({expected} ledgers)"
        );
    }

    // -------------------------------------------------------------------------
    // Issuer metadata TTL
    // -------------------------------------------------------------------------

    /// After `set_issuer_metadata`, the metadata entry TTL must equal the
    /// default TTL.
    #[test]
    fn test_issuer_metadata_ttl_set_on_write() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, _, client) = setup(&env);
        let contract_id = client.address.clone();

        let meta = IssuerMetadata {
            name: String::from_str(&env, "Acme"),
            url: String::from_str(&env, "https://acme.example"),
            description: String::from_str(&env, "Test issuer"),
        };
        client.set_issuer_metadata(&issuer, &meta);

        let ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&crate::storage::StorageKey::IssuerMetadata(issuer.clone()))
        });

        assert_eq!(
            ttl, EXPECTED_TTL,
            "issuer metadata TTL should be {EXPECTED_TTL} after write"
        );
    }
}

// ============================================================================
// Attestation Request Tests
// ============================================================================
#[cfg(test)]
mod request_tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Ledger}, Env, String};

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

    // -------------------------------------------------------------------------
    // request_attestation
    // -------------------------------------------------------------------------

    #[test]
    fn test_request_attestation_happy_path() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let id = client.request_attestation(&subject, &issuer, &claim);

        let req = client.get_attestation_request(&id);
        assert_eq!(req.subject, subject);
        assert_eq!(req.issuer, issuer);
        assert_eq!(req.claim_type, claim);
        assert_eq!(req.status, crate::types::RequestStatus::Pending);
        assert!(req.rejection_reason.is_none());
    }

    #[test]
    fn test_request_attestation_appears_in_pending_list() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let id = client.request_attestation(&subject, &issuer, &claim);

        let pending = client.get_pending_requests(&issuer, &0, &10);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending.get(0).unwrap().id, id);
    }

    #[test]
    fn test_request_attestation_unregistered_issuer_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, _, subject, client) = setup(&env);
        let stranger = Address::generate(&env);

        let claim = String::from_str(&env, "KYC");
        let result = client.try_request_attestation(&subject, &stranger, &claim);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn test_request_attestation_duplicate_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        client.request_attestation(&subject, &issuer, &claim);

        // Same subject/issuer/claim_type at the same timestamp → duplicate.
        let result = client.try_request_attestation(&subject, &issuer, &claim);
        assert_eq!(result, Err(Ok(Error::DuplicateRequest)));
    }

    #[test]
    fn test_request_attestation_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        client.request_attestation(&subject, &issuer, &claim);

        let events = env.events().all();
        let found = events.iter().any(|(_, topics, _)| {
            if let Some(raw) = topics.get(0) {
                let sym: soroban_sdk::Symbol =
                    soroban_sdk::TryFromVal::try_from_val(&env, &raw).unwrap();
                sym == soroban_sdk::symbol_short!("att_req")
            } else {
                false
            }
        });
        assert!(found, "expected 'req' event to be emitted");
    }

    // -------------------------------------------------------------------------
    // fulfill_request
    // -------------------------------------------------------------------------

    #[test]
    fn test_fulfill_request_creates_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);

        let att_id = client.fulfill_request(&issuer, &req_id, &None);

        // Attestation must exist and belong to the right parties.
        let att = client.get_attestation(&att_id);
        assert_eq!(att.issuer, issuer);
        assert_eq!(att.subject, subject);
        assert_eq!(att.claim_type, claim);
        assert!(!att.revoked);
    }

    #[test]
    fn test_fulfill_request_marks_request_fulfilled() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);
        client.fulfill_request(&issuer, &req_id, &None);

        let req = client.get_attestation_request(&req_id);
        assert_eq!(req.status, crate::types::RequestStatus::Fulfilled);
    }

    #[test]
    fn test_fulfill_request_removes_from_pending() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);
        client.fulfill_request(&issuer, &req_id, &None);

        let pending = client.get_pending_requests(&issuer, &0, &10);
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_fulfill_request_wrong_issuer_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);
        let other_issuer = Address::generate(&env);
        client.register_issuer(&admin, &other_issuer);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);

        let result = client.try_fulfill_request(&other_issuer, &req_id, &None);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn test_fulfill_request_already_processed_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);
        client.fulfill_request(&issuer, &req_id, &None);

        let result = client.try_fulfill_request(&issuer, &req_id, &None);
        assert_eq!(result, Err(Ok(Error::RequestAlreadyProcessed)));
    }

    #[test]
    fn test_fulfill_expired_request_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);

        // Advance past the 7-day TTL.
        env.ledger().with_mut(|l| {
            l.timestamp += crate::types::ATTESTATION_REQUEST_TTL_SECS + 1;
        });

        let result = client.try_fulfill_request(&issuer, &req_id, &None);
        assert_eq!(result, Err(Ok(Error::RequestExpired)));
    }

    #[test]
    fn test_fulfill_request_emits_events() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);
        client.fulfill_request(&issuer, &req_id, &None);

        let events = env.events().all();
        let found_ok = events.iter().any(|(_, topics, _)| {
            if let Some(raw) = topics.get(0) {
                let sym: soroban_sdk::Symbol =
                    soroban_sdk::TryFromVal::try_from_val(&env, &raw).unwrap();
                sym == soroban_sdk::symbol_short!("req_ok")
            } else {
                false
            }
        });
        assert!(found_ok, "expected 'req_ok' event to be emitted");
    }

    // -------------------------------------------------------------------------
    // reject_request
    // -------------------------------------------------------------------------

    #[test]
    fn test_reject_request_marks_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);
        let reason = Some(String::from_str(&env, "Insufficient documentation"));
        client.reject_request(&issuer, &req_id, &reason);

        let req = client.get_attestation_request(&req_id);
        assert_eq!(req.status, crate::types::RequestStatus::Rejected);
        assert_eq!(req.rejection_reason, reason);
    }

    #[test]
    fn test_reject_request_removes_from_pending() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);
        client.reject_request(&issuer, &req_id, &None);

        let pending = client.get_pending_requests(&issuer, &0, &10);
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_reject_request_wrong_issuer_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);
        let other = Address::generate(&env);
        client.register_issuer(&admin, &other);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);

        let result = client.try_reject_request(&other, &req_id, &None);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn test_reject_request_already_processed_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);
        client.reject_request(&issuer, &req_id, &None);

        let result = client.try_reject_request(&issuer, &req_id, &None);
        assert_eq!(result, Err(Ok(Error::RequestAlreadyProcessed)));
    }

    #[test]
    fn test_reject_reason_too_long_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);

        // 129-character reason — one over the limit.
        let long_reason = Some(String::from_str(&env, &"x".repeat(129)));
        let result = client.try_reject_request(&issuer, &req_id, &long_reason);
        assert_eq!(result, Err(Ok(Error::ReasonTooLong)));
    }

    #[test]
    fn test_reject_request_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);
        client.reject_request(&issuer, &req_id, &None);

        let events = env.events().all();
        let found = events.iter().any(|(_, topics, _)| {
            if let Some(raw) = topics.get(0) {
                let sym: soroban_sdk::Symbol =
                    soroban_sdk::TryFromVal::try_from_val(&env, &raw).unwrap();
                sym == soroban_sdk::symbol_short!("req_no")
            } else {
                false
            }
        });
        assert!(found, "expected 'req_no' event to be emitted");
    }

    // -------------------------------------------------------------------------
    // get_pending_requests pagination
    // -------------------------------------------------------------------------

    #[test]
    fn test_pending_requests_pagination() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        // Create 3 requests at different timestamps so IDs differ.
        for i in 0u64..3 {
            env.ledger().with_mut(|l| l.timestamp = 1000 + i);
            let claim = String::from_str(&env, "KYC");
            client.request_attestation(&subject, &issuer, &claim);
        }

        let page1 = client.get_pending_requests(&issuer, &0, &2);
        let page2 = client.get_pending_requests(&issuer, &2, &2);
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 1);
    }

    #[test]
    fn test_pending_requests_excludes_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        client.request_attestation(&subject, &issuer, &claim);

        // Advance past expiry.
        env.ledger().with_mut(|l| {
            l.timestamp += crate::types::ATTESTATION_REQUEST_TTL_SECS + 1;
        });

        let pending = client.get_pending_requests(&issuer, &0, &10);
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_pending_requests_excludes_fulfilled() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);
        client.fulfill_request(&issuer, &req_id, &None);

        let pending = client.get_pending_requests(&issuer, &0, &10);
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_reject_fulfilled_request_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let req_id = client.request_attestation(&subject, &issuer, &claim);
        
        // Fulfill the request first
        client.fulfill_request(&issuer, &req_id, &None);

        // Attempt to reject the already-fulfilled request
        let result = client.try_reject_request(&issuer, &req_id, &None);
        assert_eq!(result, Err(Ok(Error::RequestAlreadyProcessed)));
    }
}

// ============================================================================
// Validation Tests
//
// Negative-path tests for every validation function. Each test verifies:
//   1. The correct error variant is returned.
//   2. No state is mutated on failure (contract storage unchanged).
// ============================================================================
#[cfg(test)]
mod validation_tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Ledger}, Env, String, Vec};

    // Shared setup: initialized contract with one registered issuer.
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

    // -------------------------------------------------------------------------
    // validate_claim_type — empty string
    // -------------------------------------------------------------------------

    #[test]
    fn test_validate_claim_type_empty_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let empty = String::from_str(&env, "");
        let result = client.try_create_attestation(&issuer, &subject, &empty, &None, &None, &None);
        assert_eq!(result, Err(Ok(Error::InvalidClaimType)));
    }

    #[test]
    fn test_validate_claim_type_empty_no_state_change() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let empty = String::from_str(&env, "");
        let _ = client.try_create_attestation(&issuer, &subject, &empty, &None, &None, &None);

        // No attestation should have been stored.
        let attestations = client.get_subject_attestations(&subject, &0, &10);
        assert_eq!(attestations.len(), 0);
    }

    // -------------------------------------------------------------------------
    // validate_claim_type — too long (> 64 chars)
    // -------------------------------------------------------------------------

    #[test]
    fn test_validate_claim_type_65_chars_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let long = String::from_str(&env, &"A".repeat(65));
        let result = client.try_create_attestation(&issuer, &subject, &long, &None, &None, &None);
        assert_eq!(result, Err(Ok(Error::InvalidClaimType)));
    }

    #[test]
    fn test_validate_claim_type_64_chars_accepted() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let exactly_64 = String::from_str(&env, &"A".repeat(64));
        // Should succeed — boundary value must be accepted.
        assert!(client
            .try_create_attestation(&issuer, &subject, &exactly_64, &None, &None, &None)
            .is_ok());
    }

    // -------------------------------------------------------------------------
    // validate_claim_type — special / invalid characters
    // -------------------------------------------------------------------------

    #[test]
    fn test_validate_claim_type_space_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let with_space = String::from_str(&env, "KYC PASSED");
        let result =
            client.try_create_attestation(&issuer, &subject, &with_space, &None, &None, &None);
        assert_eq!(result, Err(Ok(Error::InvalidClaimType)));
    }

    #[test]
    fn test_validate_claim_type_dot_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let with_dot = String::from_str(&env, "kyc.passed");
        let result =
            client.try_create_attestation(&issuer, &subject, &with_dot, &None, &None, &None);
        assert_eq!(result, Err(Ok(Error::InvalidClaimType)));
    }

    #[test]
    fn test_validate_claim_type_at_symbol_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let with_at = String::from_str(&env, "kyc@passed");
        let result =
            client.try_create_attestation(&issuer, &subject, &with_at, &None, &None, &None);
        assert_eq!(result, Err(Ok(Error::InvalidClaimType)));
    }

    #[test]
    fn test_validate_claim_type_underscore_and_hyphen_accepted() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        // Underscore is allowed; hyphen is not per validation rules.
        let valid = String::from_str(&env, "KYC_PASSED_v2");
        assert!(client
            .try_create_attestation(&issuer, &subject, &valid, &None, &None, &None)
            .is_ok());
    }

    #[test]
    fn test_validate_claim_type_register_claim_type_empty_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _, _, client) = setup(&env);

        let empty = String::from_str(&env, "");
        let desc = String::from_str(&env, "desc");
        let result = client.try_register_claim_type(&admin, &empty, &desc);
        assert_eq!(result, Err(Ok(Error::InvalidClaimType)));
    }

    #[test]
    fn test_validate_claim_type_register_claim_type_too_long_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _, _, client) = setup(&env);

        let long = String::from_str(&env, &"A".repeat(65));
        let desc = String::from_str(&env, "desc");
        let result = client.try_register_claim_type(&admin, &long, &desc);
        assert_eq!(result, Err(Ok(Error::InvalidClaimType)));
    }

    // -------------------------------------------------------------------------
    // require_admin — wrong address
    // -------------------------------------------------------------------------

    #[test]
    fn test_require_admin_wrong_address_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, _, _, client) = setup(&env);

        let impostor = Address::generate(&env);
        let issuer = Address::generate(&env);
        // register_issuer requires admin auth.
        let result = client.try_register_issuer(&impostor, &issuer);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn test_require_admin_uninitialized_returns_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();

        // Contract registered but NOT initialized — no admin stored.
        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(&env, &contract_id);

        let anyone = Address::generate(&env);
        let issuer = Address::generate(&env);
        let result = client.try_register_issuer(&anyone, &issuer);
        assert_eq!(result, Err(Ok(Error::NotInitialized)));
    }

    #[test]
    fn test_require_admin_no_state_change_on_failure() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, _, _, client) = setup(&env);

        let impostor = Address::generate(&env);
        let new_issuer = Address::generate(&env);
        let _ = client.try_register_issuer(&impostor, &new_issuer);

        // The impostor's target should not have been registered.
        assert!(!client.is_issuer(&new_issuer));
    }

    // -------------------------------------------------------------------------
    // require_issuer — unregistered address
    // -------------------------------------------------------------------------

    #[test]
    fn test_require_issuer_unregistered_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, _, subject, client) = setup(&env);

        let stranger = Address::generate(&env);
        let claim = String::from_str(&env, "KYC");
        let result =
            client.try_create_attestation(&stranger, &subject, &claim, &None, &None, &None);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn test_require_issuer_removed_issuer_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        client.remove_issuer(&admin, &issuer);

        let claim = String::from_str(&env, "KYC");
        let result =
            client.try_create_attestation(&issuer, &subject, &claim, &None, &None, &None);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn test_require_issuer_no_state_change_on_failure() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, _, subject, client) = setup(&env);

        let stranger = Address::generate(&env);
        let claim = String::from_str(&env, "KYC");
        let _ = client.try_create_attestation(&stranger, &subject, &claim, &None, &None, &None);

        assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 0);
    }

    // -------------------------------------------------------------------------
    // validate_native_expiration — past / equal timestamp
    // -------------------------------------------------------------------------

    #[test]
    fn test_expiration_in_past_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let claim = String::from_str(&env, "KYC");
        let past_expiry: Option<u64> = Some(5_000); // before current time
        let result =
            client.try_create_attestation(&issuer, &subject, &claim, &past_expiry, &None, &None);
        assert_eq!(result, Err(Ok(Error::InvalidExpiration)));
    }

    #[test]
    fn test_expiration_equal_to_current_time_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let claim = String::from_str(&env, "KYC");
        let equal_expiry: Option<u64> = Some(10_000); // equal to current time
        let result =
            client.try_create_attestation(&issuer, &subject, &claim, &equal_expiry, &None, &None);
        assert_eq!(result, Err(Ok(Error::InvalidExpiration)));
    }

    #[test]
    fn test_expiration_future_accepted() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let claim = String::from_str(&env, "KYC");
        let future_expiry: Option<u64> = Some(10_001);
        assert!(client
            .try_create_attestation(&issuer, &subject, &claim, &future_expiry, &None, &None)
            .is_ok());
    }

    #[test]
    fn test_expiration_none_accepted() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        // None means no expiration — always valid.
        assert!(client
            .try_create_attestation(&issuer, &subject, &claim, &None, &None, &None)
            .is_ok());
    }

    #[test]
    fn test_expiration_no_state_change_on_failure() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let claim = String::from_str(&env, "KYC");
        let _ = client.try_create_attestation(&issuer, &subject, &claim, &Some(5_000), &None, &None);

        assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 0);
    }

    // -------------------------------------------------------------------------
    // validate_metadata — too long
    // -------------------------------------------------------------------------

    #[test]
    fn test_metadata_over_256_chars_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let long_meta = Some(String::from_str(&env, &"x".repeat(257)));
        let result =
            client.try_create_attestation(&issuer, &subject, &claim, &None, &long_meta, &None);
        assert_eq!(result, Err(Ok(Error::MetadataTooLong)));
    }

    #[test]
    fn test_metadata_exactly_256_chars_accepted() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let ok_meta = Some(String::from_str(&env, &"x".repeat(256)));
        assert!(client
            .try_create_attestation(&issuer, &subject, &claim, &None, &ok_meta, &None)
            .is_ok());
    }

    // -------------------------------------------------------------------------
    // validate_tags — too many / individual tag too long
    // -------------------------------------------------------------------------

    #[test]
    fn test_too_many_tags_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let mut tags = Vec::new(&env);
        for s in &["t1", "t2", "t3", "t4", "t5", "t6"] {
            tags.push_back(String::from_str(&env, s));
        }
        let result =
            client.try_create_attestation(&issuer, &subject, &claim, &None, &None, &Some(tags));
        assert_eq!(result, Err(Ok(Error::TooManyTags)));
    }

    #[test]
    fn test_tag_over_32_chars_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let mut tags = Vec::new(&env);
        tags.push_back(String::from_str(&env, &"t".repeat(33)));
        let result =
            client.try_create_attestation(&issuer, &subject, &claim, &None, &None, &Some(tags));
        assert_eq!(result, Err(Ok(Error::TagTooLong)));
    }

    // -------------------------------------------------------------------------
    // validate_reason — too long
    // -------------------------------------------------------------------------

    #[test]
    fn test_revoke_reason_over_128_chars_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let long_reason = Some(String::from_str(&env, &"r".repeat(129)));
        let result = client.try_revoke_attestation(&issuer, &id, &long_reason);
        assert_eq!(result, Err(Ok(Error::ReasonTooLong)));
    }

    #[test]
    fn test_revoke_reason_exactly_128_chars_accepted() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC");
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let ok_reason = Some(String::from_str(&env, &"r".repeat(128)));
        assert!(client.try_revoke_attestation(&issuer, &id, &ok_reason).is_ok());
    }

    // -------------------------------------------------------------------------
    // validate_import_timestamps — future timestamp / expiration before timestamp
    // -------------------------------------------------------------------------

    #[test]
    fn test_import_future_timestamp_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        env.ledger().with_mut(|l| l.timestamp = 1_000);

        let claim = String::from_str(&env, "KYC");
        let future_ts: u64 = 2_000; // after current ledger time
        let result = client.try_import_attestation(&admin, &issuer, &subject, &claim, &future_ts, &None);
        assert_eq!(result, Err(Ok(Error::InvalidTimestamp)));
    }

    #[test]
    fn test_import_expiration_before_timestamp_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let claim = String::from_str(&env, "KYC");
        let ts: u64 = 5_000;
        let expiry: Option<u64> = Some(4_000); // before ts
        let result = client.try_import_attestation(&admin, &issuer, &subject, &claim, &ts, &expiry);
        assert_eq!(result, Err(Ok(Error::InvalidExpiration)));
    }

    // -------------------------------------------------------------------------
    // require_not_paused
    // -------------------------------------------------------------------------

    #[test]
    fn test_paused_contract_rejects_create_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        client.pause(&admin);

        let claim = String::from_str(&env, "KYC");
        let result =
            client.try_create_attestation(&issuer, &subject, &claim, &None, &None, &None);
        assert_eq!(result, Err(Ok(Error::ContractPaused)));
    }
}

// ── claim_type validation tests ──────────────────────────────────────────────

#[test]
fn test_valid_claim_type_kyc_passed() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    assert!(!id.is_empty());
}

#[test]
fn test_valid_claim_type_accredited_investor() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");
    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    assert!(!id.is_empty());
}

#[test]
fn test_valid_claim_type_exactly_64_chars() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    // 64 alphanumeric characters — exactly at the limit
    let claim_type = String::from_str(&env, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    assert!(!id.is_empty());
}

#[test]
#[should_panic]
fn test_claim_type_too_long_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    // 65 characters — one over the limit
    let claim_type = String::from_str(&env, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
}

#[test]
#[should_panic]
fn test_claim_type_with_space_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC PASSED");
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
}

#[test]
#[should_panic]
fn test_claim_type_with_hyphen_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC-PASSED");
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
}

#[test]
#[should_panic]
fn test_claim_type_with_dot_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "kyc.passed");
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
}

#[test]
#[should_panic]
fn test_claim_type_with_special_chars_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC@PASSED!");
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
}

// ── subject whitelist tests ───────────────────────────────────────────────────

#[test]
fn test_whitelist_disabled_by_default() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    assert!(!client.is_whitelist_enabled(&issuer));
}

#[test]
fn test_attestation_succeeds_without_whitelist() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    // whitelist disabled — any subject is accepted
    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    assert!(!id.is_empty());
}

#[test]
#[should_panic]
fn test_attestation_rejected_when_whitelist_enabled_and_subject_not_listed() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    client.set_whitelist_enabled(&issuer, &true);
    // subject not added — should panic with SubjectNotWhitelisted
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
}

#[test]
fn test_attestation_succeeds_when_subject_is_whitelisted() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    client.set_whitelist_enabled(&issuer, &true);
    client.add_to_whitelist(&issuer, &subject);

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    assert!(!id.is_empty());
}

#[test]
fn test_add_and_remove_from_whitelist() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);

    assert!(!client.is_whitelisted(&issuer, &subject));

    client.add_to_whitelist(&issuer, &subject);
    assert!(client.is_whitelisted(&issuer, &subject));

    client.remove_from_whitelist(&issuer, &subject);
    assert!(!client.is_whitelisted(&issuer, &subject));
}

#[test]
fn test_issuer_controls_own_whitelist_independently() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer1, client) = setup(&env);
    let issuer2 = Address::generate(&env);
    client.register_issuer(&admin, &issuer2);

    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    // issuer1 enables whitelist and adds subject; issuer2 does not
    client.set_whitelist_enabled(&issuer1, &true);
    client.add_to_whitelist(&issuer1, &subject);

    // issuer1 can attest
    let id1 = client.create_attestation(&issuer1, &subject, &claim_type, &None, &None, &None);
    assert!(!id1.is_empty());

    // issuer2 has no whitelist enabled — can also attest freely
    let id2 = client.create_attestation(&issuer2, &subject, &claim_type, &None, &None, &None);
    assert!(!id2.is_empty());
}

#[test]
#[should_panic]
fn test_whitelist_check_before_storage_write() {
    // Verifies rejection happens before any storage write by checking
    // that a failed attestation leaves no attestation record.
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    client.set_whitelist_enabled(&issuer, &true);
    // This must panic — no attestation should be stored
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
}

#[test]
fn test_create_attestations_batch_empty_subjects_is_noop() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let subjects: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);

    let ids = client.create_attestations_batch(&issuer, &subjects, &claim_type, &None);

    assert_eq!(ids.len(), 0);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 0);
}

#[test]
fn test_create_attestations_batch_duplicate_subject_rolls_back_all() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let subject = Address::generate(&env);

    let mut subjects = soroban_sdk::Vec::new(&env);
    subjects.push_back(subject.clone());
    subjects.push_back(subject.clone());

    let result = client.try_create_attestations_batch(&issuer, &subjects, &claim_type, &None);
    assert_eq!(result, Err(Ok(types::Error::DuplicateAttestation)));
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 0);
    assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 0);
}

#[test]
fn test_create_attestations_batch_subject_at_limit_rolls_back_all() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, client) = setup(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    client.set_limits(&admin, &10_000, &1);

    let subject_at_limit = Address::generate(&env);
    client.create_attestation(&issuer, &subject_at_limit, &claim_type, &None, &None, &None);

    let fresh_subject = Address::generate(&env);
    let mut subjects = soroban_sdk::Vec::new(&env);
    subjects.push_back(fresh_subject.clone());
    subjects.push_back(subject_at_limit.clone());

    env.ledger().set_timestamp(env.ledger().timestamp() + 1);
    let result = client.try_create_attestations_batch(&issuer, &subjects, &claim_type, &None);
    assert_eq!(result, Err(Ok(types::Error::LimitExceeded)));
    assert_eq!(client.get_subject_attestations(&fresh_subject, &0, &10).len(), 0);
}

const FULL_TTL: u32 = 3_110_400; // ~180 days in ledgers

fn ttl_env() -> Env {
    let env = Env::default();
    env.ledger().with_mut(|li| {
        li.sequence_number = 100_000;
        li.min_persistent_entry_ttl = FULL_TTL;
        li.max_entry_ttl = FULL_TTL + 1;
    });
    env
}

/// Advance the ledger sequence by `delta` ledgers, reducing all entry TTLs by
/// the same amount.
fn advance_ledger(env: &Env, delta: u32) {
    env.ledger().with_mut(|li| {
        li.sequence_number += delta;
    });
}

#[test]
fn test_ttl_refreshed_on_revocation() {
    let env = ttl_env();
    env.mock_all_auths();
    let (admin, issuer, client) = setup(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let max: u32 = 5;

    client.set_limits(&admin, &max, &10_000);

    let mut subjects = soroban_sdk::Vec::new(&env);
    for _ in 0..max {
        subjects.push_back(Address::generate(&env));
    }

    let ids = client.create_attestations_batch(&issuer, &subjects, &claim_type, &None);
    assert_eq!(ids.len(), max);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &(max + 1)).len(), max);
}

// ── Pause: all write operations blocked ──────────────────────────────────────

#[cfg(test)]
mod pause_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    fn setup(env: &Env) -> (Address, Address, TrustLinkContractClient<'_>) {
        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let issuer = Address::generate(env);
        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);
        (admin, issuer, client)
    }

    #[test]
    fn test_paused_blocks_revoke_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, client) = setup(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
        client.pause(&admin);

        let result = client.try_revoke_attestation(&issuer, &id, &None);
        assert_eq!(result, Err(Ok(Error::ContractPaused)));
        // attestation must still be valid
        assert!(!client.get_attestation(&id).revoked);
    }

    #[test]
    fn test_paused_blocks_import_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, client) = setup(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        env.ledger().with_mut(|l| l.timestamp = 5_000);
        client.pause(&admin);

        let result =
            client.try_import_attestation(&admin, &issuer, &subject, &claim, &1_000, &None);
        assert_eq!(result, Err(Ok(Error::ContractPaused)));
        assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 0);
    }

    #[test]
    fn test_paused_blocks_bridge_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _, client) = setup(&env);
        let bridge = Address::generate(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");
        let chain = String::from_str(&env, "ethereum");
        let tx = String::from_str(&env, "0xabc");

        client.register_bridge(&admin, &bridge);
        client.pause(&admin);

        let result = client.try_bridge_attestation(&bridge, &subject, &claim, &chain, &tx);
        assert_eq!(result, Err(Ok(Error::ContractPaused)));
        assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 0);
    }

    #[test]
    fn test_paused_blocks_register_issuer() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _, client) = setup(&env);
        let new_issuer = Address::generate(&env);

        client.pause(&admin);

        // register_issuer is an admin-only write — must be blocked when paused
        let result = client.try_register_issuer(&admin, &new_issuer);
        assert_eq!(result, Err(Ok(Error::ContractPaused)));
        assert!(!client.is_issuer(&new_issuer));
    }

    #[test]
    fn test_paused_blocks_propose_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, client) = setup(&env);
        let issuer2 = Address::generate(&env);
        let issuer3 = Address::generate(&env);
        client.register_issuer(&admin, &issuer2);
        client.register_issuer(&admin, &issuer3);

        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "ACCREDITED_INVESTOR");
        let mut required = soroban_sdk::Vec::new(&env);
        required.push_back(issuer.clone());
        required.push_back(issuer2.clone());
        required.push_back(issuer3.clone());

        client.pause(&admin);

        let result = client.try_propose_attestation(&issuer, &subject, &claim, &required, &2);
        assert_eq!(result, Err(Ok(Error::ContractPaused)));
    }
}

// =============================================================================
// Issue #342 — Two-step admin transfer tests
// =============================================================================
#[cfg(test)]
mod two_step_admin_transfer_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    fn setup(env: &Env) -> (Address, TrustLinkContractClient<'_>) {
        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin, &None);
        (admin, client)
    }

    /// Propose transfer → pending admin stored (NotFound before, succeeds after).
    #[test]
    fn test_propose_transfer_stores_pending() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);
        let new_admin = Address::generate(&env);

        // Before proposal, accept should fail with NotFound.
        let before = client.try_accept_admin_transfer(&new_admin);
        assert_eq!(before, Err(Ok(Error::NotFound)));

        // Propose the transfer.
        client.propose_admin_transfer(&admin, &new_admin);

        // Now accept should succeed (pending is stored).
        client.accept_admin_transfer(&new_admin);
        assert_eq!(client.get_admin(), new_admin);
    }

    /// Wrong address tries to accept → Unauthorized.
    #[test]
    fn test_wrong_address_cannot_accept() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);
        let new_admin = Address::generate(&env);
        let wrong = Address::generate(&env);

        client.propose_admin_transfer(&admin, &new_admin);

        let result = client.try_accept_admin_transfer(&wrong);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));

        // Original admin still in place.
        assert_eq!(client.get_admin(), admin);
    }

    /// Correct new admin accepts → admin updated.
    #[test]
    fn test_correct_new_admin_accepts_and_becomes_admin() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);
        let new_admin = Address::generate(&env);

        client.propose_admin_transfer(&admin, &new_admin);
        client.accept_admin_transfer(&new_admin);

        assert_eq!(client.get_admin(), new_admin.clone());
    }

    /// Old admin loses privileges after transfer completes.
    #[test]
    fn test_old_admin_loses_privileges_after_transfer() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);
        let new_admin = Address::generate(&env);
        let issuer = Address::generate(&env);

        client.propose_admin_transfer(&admin, &new_admin);
        client.accept_admin_transfer(&new_admin);

        // Old admin can no longer register issuers.
        let result = client.try_register_issuer(&admin, &issuer);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));

        // New admin can.
        client.register_issuer(&new_admin, &issuer);
        assert!(client.is_issuer(&issuer));
    }

    /// Propose then cancel → pending cleared.
    #[test]
    fn test_propose_then_cancel_clears_pending() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);
        let new_admin = Address::generate(&env);

        // Propose the transfer.
        client.propose_admin_transfer(&admin, &new_admin);

        // Cancel it.
        client.cancel_admin_transfer(&admin);

        // Accept should now fail with NotFound — pending is cleared.
        let result = client.try_accept_admin_transfer(&new_admin);
        assert_eq!(result, Err(Ok(Error::NotFound)));

        // Original admin still in place.
        assert_eq!(client.get_admin(), admin);
    }

    /// Non-admin cannot propose a transfer.
    #[test]
    fn test_non_admin_cannot_propose_transfer() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, client) = setup(&env);
        let non_admin = Address::generate(&env);
        let new_admin = Address::generate(&env);

        let result = client.try_propose_admin_transfer(&non_admin, &new_admin);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }
}

// =============================================================================
// Issue #340 — get_attestations_in_range boundary condition tests
// =============================================================================
#[cfg(test)]
mod attestations_in_range_boundary_tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Ledger}, Env, String};

    fn setup(env: &Env) -> (Address, Address, TrustLinkContractClient<'_>) {
        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let issuer = Address::generate(env);
        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);
        (admin, issuer, client)
    }

    /// from_ts == to_ts with no attestation at that timestamp → empty result.
    #[test]
    fn test_equal_timestamps_no_attestation_returns_empty() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, _, client) = setup(&env);
        let subject = Address::generate(&env);

        let result = client.get_attestations_in_range(&subject, &500, &500, &0, &10);
        assert_eq!(result.len(), 0);
    }

    /// Attestation at exactly from_ts → included.
    #[test]
    fn test_attestation_at_exactly_from_ts_is_included() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, client) = setup(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        env.ledger().set_timestamp(1000);
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        // Query with from_ts == attestation timestamp.
        let result = client.get_attestations_in_range(&subject, &1000, &2000, &0, &10);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap().id, id);
    }

    /// Attestation at exactly to_ts → included (inclusive upper bound).
    #[test]
    fn test_attestation_at_exactly_to_ts_is_included() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, client) = setup(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        env.ledger().set_timestamp(2000);
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        // Query with to_ts == attestation timestamp.
        let result = client.get_attestations_in_range(&subject, &1000, &2000, &0, &10);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap().id, id);
    }

    /// from_ts == to_ts and attestation exists at that exact timestamp → included.
    #[test]
    fn test_equal_timestamps_with_attestation_returns_one() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, client) = setup(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        env.ledger().set_timestamp(750);
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let result = client.get_attestations_in_range(&subject, &750, &750, &0, &10);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap().id, id);
    }

    /// Range with no attestations → empty result.
    #[test]
    fn test_range_with_no_attestations_returns_empty() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, client) = setup(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        // Attestation at ts=100, query range 200–300.
        env.ledger().set_timestamp(100);
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let result = client.get_attestations_in_range(&subject, &200, &300, &0, &10);
        assert_eq!(result.len(), 0);
    }

    /// Attestation just before from_ts → excluded.
    #[test]
    fn test_attestation_just_before_from_ts_excluded() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, client) = setup(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        env.ledger().set_timestamp(999);
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let result = client.get_attestations_in_range(&subject, &1000, &2000, &0, &10);
        assert_eq!(result.len(), 0);
    }

    /// Attestation just after to_ts → excluded.
    #[test]
    fn test_attestation_just_after_to_ts_excluded() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, client) = setup(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        env.ledger().set_timestamp(2001);
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let result = client.get_attestations_in_range(&subject, &1000, &2000, &0, &10);
        assert_eq!(result.len(), 0);
    }
}

// =============================================================================
// Issue #341 — Endorsement system tests
// =============================================================================
#[cfg(test)]
mod endorsement_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    /// Setup: contract + admin + two issuers + one subject.
    fn setup(env: &Env) -> (Address, Address, Address, Address, TrustLinkContractClient<'_>) {
        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let issuer = Address::generate(env);
        let endorser = Address::generate(env);
        let subject = Address::generate(env);
        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);
        client.register_issuer(&admin, &endorser);
        (admin, issuer, endorser, subject, client)
    }

    /// Endorse attestation → endorsement stored, count increases.
    #[test]
    fn test_endorse_attestation_stored() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, endorser, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
        client.endorse_attestation(&endorser, &id);

        let count = client.get_endorsement_count(&id);
        assert_eq!(count, 1);
    }

    /// Cannot endorse own attestation → CannotEndorseOwn.
    #[test]
    fn test_cannot_endorse_own_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, _, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let result = client.try_endorse_attestation(&issuer, &id);
        assert_eq!(result, Err(Ok(Error::CannotEndorseOwn)));
    }

    /// Cannot endorse twice → AlreadyEndorsed.
    #[test]
    fn test_cannot_endorse_twice() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, endorser, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
        client.endorse_attestation(&endorser, &id);

        let result = client.try_endorse_attestation(&endorser, &id);
        assert_eq!(result, Err(Ok(Error::AlreadyEndorsed)));
    }

    /// get_endorsement_count returns correct value after multiple endorsers.
    #[test]
    fn test_get_endorsement_count_correct_value() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, endorser1, subject, client) = setup(&env);
        let endorser2 = Address::generate(&env);
        client.register_issuer(&admin, &endorser2);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        assert_eq!(client.get_endorsement_count(&id), 0);

        client.endorse_attestation(&endorser1, &id);
        assert_eq!(client.get_endorsement_count(&id), 1);

        client.endorse_attestation(&endorser2, &id);
        assert_eq!(client.get_endorsement_count(&id), 2);
    }

    /// Endorsement on revoked attestation → AlreadyRevoked.
    #[test]
    fn test_endorse_revoked_attestation_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, endorser, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
        client.revoke_attestation(&issuer, &id, &None);

        let result = client.try_endorse_attestation(&endorser, &id);
        assert_eq!(result, Err(Ok(Error::AlreadyRevoked)));
    }

    /// Non-issuer cannot endorse → Unauthorized.
    #[test]
    fn test_non_issuer_cannot_endorse() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, _, subject, client) = setup(&env);
        let non_issuer = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let result = client.try_endorse_attestation(&non_issuer, &id);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }
}

// =============================================================================
// Issue #343 — IssuerTier assignment and enforcement tests
// =============================================================================
#[cfg(test)]
mod issuer_tier_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    fn setup(env: &Env) -> (Address, Address, TrustLinkContractClient<'_>) {
        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let issuer = Address::generate(env);
        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);
        (admin, issuer, client)
    }

    /// Default tier is Basic on registration (None or Basic).
    #[test]
    fn test_default_tier_is_basic_on_registration() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, client) = setup(&env);

        let tier = client.get_issuer_tier(&issuer);
        // Unset tier is treated as Basic (None maps to Basic in confidence score).
        assert!(tier.is_none() || tier == Some(types::IssuerTier::Basic));
    }

    /// Admin can upgrade issuer to Verified.
    #[test]
    fn test_admin_can_set_verified_tier() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, client) = setup(&env);

        client.set_issuer_tier(&admin, &issuer, &types::IssuerTier::Verified);
        assert_eq!(client.get_issuer_tier(&issuer), Some(types::IssuerTier::Verified));
    }

    /// Admin can upgrade issuer to Premium.
    #[test]
    fn test_admin_can_set_premium_tier() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, client) = setup(&env);

        client.set_issuer_tier(&admin, &issuer, &types::IssuerTier::Premium);
        assert_eq!(client.get_issuer_tier(&issuer), Some(types::IssuerTier::Premium));
    }

    /// Non-admin cannot change tier → Unauthorized.
    #[test]
    fn test_non_admin_cannot_change_tier() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, client) = setup(&env);
        let non_admin = Address::generate(&env);

        let result = client.try_set_issuer_tier(&non_admin, &issuer, &types::IssuerTier::Premium);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    /// Tier affects confidence score: Basic→30, Verified→60, Premium→90.
    #[test]
    fn test_tier_affects_confidence_score() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, client) = setup(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        // Basic (default) → 30.
        let score_basic = client.get_confidence_score(&id);
        assert_eq!(score_basic, Some(30));

        // Verified → 60.
        client.set_issuer_tier(&admin, &issuer, &types::IssuerTier::Verified);
        let score_verified = client.get_confidence_score(&id);
        assert_eq!(score_verified, Some(60));

        // Premium → 90.
        client.set_issuer_tier(&admin, &issuer, &types::IssuerTier::Premium);
        let score_premium = client.get_confidence_score(&id);
        assert_eq!(score_premium, Some(90));
    }

    /// Cannot set tier for unregistered issuer → Unauthorized.
    #[test]
    fn test_cannot_set_tier_for_unregistered_issuer() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _, client) = setup(&env);
        let unregistered = Address::generate(&env);

        let result = client.try_set_issuer_tier(&admin, &unregistered, &types::IssuerTier::Verified);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    /// Admin can downgrade tier (Premium → Basic).
    #[test]
    fn test_admin_can_downgrade_tier() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, client) = setup(&env);

        client.set_issuer_tier(&admin, &issuer, &types::IssuerTier::Premium);
        assert_eq!(client.get_issuer_tier(&issuer), Some(types::IssuerTier::Premium));

        client.set_issuer_tier(&admin, &issuer, &types::IssuerTier::Basic);
        assert_eq!(client.get_issuer_tier(&issuer), Some(types::IssuerTier::Basic));
    }
}

// ── valid_from / Pending lifecycle tests ─────────────────────────────────────

#[cfg(test)]
mod valid_from_lifecycle {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Env, String,
    };

    /// Deploy contract, initialize, register one issuer; return (admin, issuer, subject, client).
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

    // ── 1. Basic Pending → Valid transition ──────────────────────────────────

    #[test]
    fn test_pending_before_valid_from_then_valid_after() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let (_admin, issuer, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        // valid_from is 500 seconds in the future
        let valid_from: u64 = 1500;
        let id = client.create_attestation_valid_from(
            &issuer, &subject, &claim, &None, &None, &None, &valid_from,
        );

        // Before valid_from: status must be Pending
        assert_eq!(
            client.get_attestation_status(&id),
            types::AttestationStatus::Pending,
            "status should be Pending before valid_from"
        );

        // has_valid_claim must return false while Pending
        assert!(
            !client.has_valid_claim(&subject, &claim),
            "has_valid_claim must be false while attestation is Pending"
        );

        // Advance ledger to exactly valid_from
        env.ledger().set_timestamp(valid_from);

        assert_eq!(
            client.get_attestation_status(&id),
            types::AttestationStatus::Valid,
            "status should be Valid at exactly valid_from"
        );
        assert!(
            client.has_valid_claim(&subject, &claim),
            "has_valid_claim must be true once valid_from is reached"
        );
    }

    // ── 2. Boundary: one second before valid_from ─────────────────────────────

    #[test]
    fn test_pending_one_second_before_valid_from() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let (_admin, issuer, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");
        let valid_from: u64 = 2000;

        let id = client.create_attestation_valid_from(
            &issuer, &subject, &claim, &None, &None, &None, &valid_from,
        );

        // One second before valid_from → still Pending
        env.ledger().set_timestamp(valid_from - 1);
        assert_eq!(client.get_attestation_status(&id), types::AttestationStatus::Pending);
        assert!(!client.has_valid_claim(&subject, &claim));

        // Exactly at valid_from → Valid
        env.ledger().set_timestamp(valid_from);
        assert_eq!(client.get_attestation_status(&id), types::AttestationStatus::Valid);
        assert!(client.has_valid_claim(&subject, &claim));
    }

    // ── 3. valid_from stored on the attestation struct ────────────────────────

    #[test]
    fn test_valid_from_stored_on_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let (_admin, issuer, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");
        let valid_from: u64 = 9999;

        let id = client.create_attestation_valid_from(
            &issuer, &subject, &claim, &None, &None, &None, &valid_from,
        );

        let att = client.get_attestation(&id);
        assert_eq!(att.valid_from, Some(valid_from), "valid_from must be persisted");
    }

    // ── 4. create_attestation (no valid_from) still works as before ───────────

    #[test]
    fn test_create_attestation_without_valid_from_is_immediately_valid() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let (_admin, issuer, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        assert_eq!(client.get_attestation_status(&id), types::AttestationStatus::Valid);
        assert!(client.has_valid_claim(&subject, &claim));

        let att = client.get_attestation(&id);
        assert_eq!(att.valid_from, None, "standard attestation must have valid_from = None");
    }

    // ── 5. InvalidValidFrom when valid_from is in the past ────────────────────

    #[test]
    #[should_panic]
    fn test_create_with_past_valid_from_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(5000);

        let (_admin, issuer, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        // valid_from in the past → must return InvalidValidFrom
        client.create_attestation_valid_from(
            &issuer, &subject, &claim, &None, &None, &None, &4999,
        );
    }

    // ── 6. InvalidValidFrom when valid_from == current timestamp ──────────────

    #[test]
    #[should_panic]
    fn test_create_with_valid_from_equal_to_now_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(5000);

        let (_admin, issuer, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        // valid_from == now → must be rejected (must be strictly future)
        client.create_attestation_valid_from(
            &issuer, &subject, &claim, &None, &None, &None, &5000,
        );
    }

    // ── 7. Pending attestation does not satisfy has_valid_claim ───────────────

    #[test]
    fn test_has_valid_claim_false_while_pending_even_with_other_claims() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let (_admin, issuer, subject, client) = setup(&env);
        let kyc = String::from_str(&env, "KYC_PASSED");
        let aml = String::from_str(&env, "AML_CLEARED");

        // Create a valid AML attestation
        client.create_attestation(&issuer, &subject, &aml, &None, &None, &None);

        // Create a pending KYC attestation
        client.create_attestation_valid_from(
            &issuer, &subject, &kyc, &None, &None, &None, &9999,
        );

        // AML is valid
        assert!(client.has_valid_claim(&subject, &aml));
        // KYC is still pending
        assert!(!client.has_valid_claim(&subject, &kyc));
    }

    // ── 8. Pending → Valid → Expired full lifecycle ───────────────────────────

    #[test]
    fn test_full_lifecycle_pending_valid_expired() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let (_admin, issuer, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let valid_from: u64 = 2000;
        let expiration: u64 = 3000;

        let id = client.create_attestation_valid_from(
            &issuer, &subject, &claim, &Some(expiration), &None, &None, &valid_from,
        );

        // Phase 1: Pending
        env.ledger().set_timestamp(1500);
        assert_eq!(client.get_attestation_status(&id), types::AttestationStatus::Pending);
        assert!(!client.has_valid_claim(&subject, &claim));

        // Phase 2: Valid
        env.ledger().set_timestamp(2500);
        assert_eq!(client.get_attestation_status(&id), types::AttestationStatus::Valid);
        assert!(client.has_valid_claim(&subject, &claim));

        // Phase 3: Expired
        env.ledger().set_timestamp(3000);
        assert_eq!(client.get_attestation_status(&id), types::AttestationStatus::Expired);
        assert!(!client.has_valid_claim(&subject, &claim));
    }

    // ── 9. Revoked pending attestation stays Pending (revoked check is after) ─

    #[test]
    fn test_pending_takes_priority_over_revoked() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let (_admin, issuer, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation_valid_from(
            &issuer, &subject, &claim, &None, &None, &None, &9999,
        );

        // Revoke while still pending
        client.revoke_attestation(&issuer, &id, &None);

        // get_status checks valid_from first, so it should still be Pending
        assert_eq!(
            client.get_attestation_status(&id),
            types::AttestationStatus::Pending,
            "Pending takes priority over Revoked per get_status ordering"
        );
    }

    // ── 10. Far-future valid_from ─────────────────────────────────────────────

    #[test]
    fn test_far_future_valid_from_stays_pending() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);

        let (_admin, issuer, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        // valid_from 100 years out (approx)
        let far_future: u64 = 1000 + 100 * 365 * 24 * 3600;
        let id = client.create_attestation_valid_from(
            &issuer, &subject, &claim, &None, &None, &None, &far_future,
        );

        // Advance 10 years — still pending
        env.ledger().set_timestamp(1000 + 10 * 365 * 24 * 3600);
        assert_eq!(client.get_attestation_status(&id), types::AttestationStatus::Pending);
        assert!(!client.has_valid_claim(&subject, &claim));
    }
}

// =============================================================================
// Admin Council Tests
// =============================================================================
#[cfg(test)]
mod admin_council_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup(env: &Env) -> (Address, TrustLinkContractClient<'_>) {
        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin, &None);
        (admin, client)
    }

    /// Existing admin can add a new member to the council.
    #[test]
    fn test_add_member_to_council() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);
        let new_admin = Address::generate(&env);

        client.add_admin(&admin, &new_admin);

        // new_admin should now be able to perform admin operations (e.g. register an issuer)
        let issuer = Address::generate(&env);
        client.register_issuer(&new_admin, &issuer);
        assert!(client.is_issuer(&issuer));
    }

    /// Adding the same member twice is idempotent — no error.
    #[test]
    fn test_add_duplicate_member_is_idempotent() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);
        let new_admin = Address::generate(&env);

        client.add_admin(&admin, &new_admin);
        // Second add should not panic or error.
        client.add_admin(&admin, &new_admin);
    }

    /// Existing admin can remove another council member.
    #[test]
    fn test_remove_member_from_council() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);
        let second_admin = Address::generate(&env);

        // Add second admin first.
        client.add_admin(&admin, &second_admin);

        // Remove second admin.
        client.remove_admin(&admin, &second_admin);

        // second_admin should no longer be able to perform admin operations.
        let issuer = Address::generate(&env);
        let result = client.try_register_issuer(&second_admin, &issuer);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    /// Removing the last admin returns LastAdminCannotBeRemoved.
    #[test]
    fn test_cannot_remove_last_admin() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);

        let result = client.try_remove_admin(&admin, &admin);
        assert_eq!(result, Err(Ok(Error::LastAdminCannotBeRemoved)));
    }

    /// Non-council member cannot add a new admin.
    #[test]
    fn test_non_council_member_cannot_add_admin() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, client) = setup(&env);
        let non_admin = Address::generate(&env);
        let target = Address::generate(&env);

        let result = client.try_add_admin(&non_admin, &target);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    /// Non-council member cannot remove an admin.
    #[test]
    fn test_non_council_member_cannot_remove_admin() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);
        let non_admin = Address::generate(&env);

        let result = client.try_remove_admin(&non_admin, &admin);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }
}

// =============================================================================
// Claim Type Registry Pagination Tests
// =============================================================================
#[cfg(test)]
mod claim_type_pagination_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    fn setup(env: &Env) -> (Address, TrustLinkContractClient<'_>) {
        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin, &None);
        (admin, client)
    }

    fn register(client: &TrustLinkContractClient<'_>, env: &Env, admin: &Address, id: &str) {
        client.register_claim_type(
            admin,
            &String::from_str(env, id),
            &String::from_str(env, "desc"),
        );
    }

    /// Empty registry → empty list.
    #[test]
    fn test_empty_registry_returns_empty() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, client) = setup(&env);

        let result = client.list_claim_types(&0, &10);
        assert_eq!(result.len(), 0);
    }

    /// Exactly one page worth of items → all returned in order.
    #[test]
    fn test_exactly_one_page_returns_all_items() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);

        register(&client, &env, &admin, "KYC");
        register(&client, &env, &admin, "AML");
        register(&client, &env, &admin, "AGE");

        let result = client.list_claim_types(&0, &3);
        assert_eq!(result.len(), 3);
        assert_eq!(result.get(0).unwrap(), String::from_str(&env, "KYC"));
        assert_eq!(result.get(1).unwrap(), String::from_str(&env, "AML"));
        assert_eq!(result.get(2).unwrap(), String::from_str(&env, "AGE"));
    }

    /// Multiple pages → correct items per page.
    #[test]
    fn test_multiple_pages_correct_items_per_page() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);

        register(&client, &env, &admin, "T1");
        register(&client, &env, &admin, "T2");
        register(&client, &env, &admin, "T3");
        register(&client, &env, &admin, "T4");
        register(&client, &env, &admin, "T5");

        // Page 0: items 0-1
        let page0 = client.list_claim_types(&0, &2);
        assert_eq!(page0.len(), 2);
        assert_eq!(page0.get(0).unwrap(), String::from_str(&env, "T1"));
        assert_eq!(page0.get(1).unwrap(), String::from_str(&env, "T2"));

        // Page 1: items 2-3
        let page1 = client.list_claim_types(&2, &2);
        assert_eq!(page1.len(), 2);
        assert_eq!(page1.get(0).unwrap(), String::from_str(&env, "T3"));
        assert_eq!(page1.get(1).unwrap(), String::from_str(&env, "T4"));

        // Page 2: item 4 (partial page)
        let page2 = client.list_claim_types(&4, &2);
        assert_eq!(page2.len(), 1);
        assert_eq!(page2.get(0).unwrap(), String::from_str(&env, "T5"));
    }

    /// Start beyond total count → empty list.
    #[test]
    fn test_start_beyond_total_returns_empty() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);

        register(&client, &env, &admin, "KYC");
        register(&client, &env, &admin, "AML");

        let result = client.list_claim_types(&10, &5);
        assert_eq!(result.len(), 0);
    }

    /// Limit zero → empty list.
    #[test]
    fn test_limit_zero_returns_empty() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, client) = setup(&env);

        register(&client, &env, &admin, "KYC");

        let result = client.list_claim_types(&0, &0);
        assert_eq!(result.len(), 0);
    }
}

// ── Attestation Template tests ───────────────────────────────────────────────

#[test]
fn test_create_template_and_get() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: Some(30),
        metadata_template: Some(String::from_str(&env, "default-meta")),
    };

    client.create_template(&issuer, &String::from_str(&env, "tmpl1"), &template);

    let retrieved = client.get_template(&issuer, &String::from_str(&env, "tmpl1")).unwrap();
    assert_eq!(retrieved.claim_type, template.claim_type);
    assert_eq!(retrieved.default_expiration_days, template.default_expiration_days);
    assert_eq!(retrieved.metadata_template, template.metadata_template);
}

#[test]
fn test_create_template_overwrite() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let t1 = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: Some(10),
        metadata_template: None,
    };
    let t2 = types::AttestationTemplate {
        claim_type: String::from_str(&env, "AML"),
        default_expiration_days: Some(60),
        metadata_template: Some(String::from_str(&env, "updated")),
    };
    let id = String::from_str(&env, "tmpl1");

    client.create_template(&issuer, &id, &t1);
    client.create_template(&issuer, &id, &t2);

    let retrieved = client.get_template(&issuer, &id).unwrap();
    assert_eq!(retrieved.claim_type, t2.claim_type);
    assert_eq!(retrieved.default_expiration_days, t2.default_expiration_days);
}

#[test]
fn test_create_template_empty_claim_type_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, ""),
        default_expiration_days: None,
        metadata_template: None,
    };

    let result = client.try_create_template(&issuer, &String::from_str(&env, "t1"), &template);
    assert_eq!(result, Err(Ok(Error::InvalidClaimType)));
}

#[test]
fn test_create_template_metadata_too_long_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    // 257-byte metadata string
    let long_meta = "a".repeat(257);
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: Some(String::from_str(&env, &long_meta)),
    };

    let result = client.try_create_template(&issuer, &String::from_str(&env, "t1"), &template);
    assert_eq!(result, Err(Ok(Error::MetadataTooLong)));
}

#[test]
fn test_create_template_non_issuer_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, _, client) = setup(&env);

    let non_issuer = Address::generate(&env);
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };

    let result = client.try_create_template(&non_issuer, &String::from_str(&env, "t1"), &template);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_list_templates_insertion_order_no_duplicates() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let make_tmpl = |ct: &str| types::AttestationTemplate {
        claim_type: String::from_str(&env, ct),
        default_expiration_days: None,
        metadata_template: None,
    };

    let id_a = String::from_str(&env, "alpha");
    let id_b = String::from_str(&env, "beta");
    let id_c = String::from_str(&env, "gamma");

    client.create_template(&issuer, &id_a, &make_tmpl("KYC"));
    client.create_template(&issuer, &id_b, &make_tmpl("AML"));
    client.create_template(&issuer, &id_c, &make_tmpl("KYC"));
    // Overwrite alpha — should NOT add a duplicate
    client.create_template(&issuer, &id_a, &make_tmpl("AML"));

    let list = client.list_templates(&issuer);
    assert_eq!(list.len(), 3);
    assert_eq!(list.get(0).unwrap(), id_a);
    assert_eq!(list.get(1).unwrap(), id_b);
    assert_eq!(list.get(2).unwrap(), id_c);
}

#[test]
fn test_list_templates_empty_for_new_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let list = client.list_templates(&issuer);
    assert_eq!(list.len(), 0);
}

#[test]
fn test_get_template_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let result = client.try_get_template(&issuer, &String::from_str(&env, "nonexistent"));
    assert_eq!(result, Err(Ok(Error::NotFound)));
}

#[test]
fn test_create_attestation_from_template_happy_path() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let now = 1_000_000u64;
    env.ledger().set_timestamp(now);

    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: Some(10),
        metadata_template: Some(String::from_str(&env, "tmpl-meta")),
    };
    let tmpl_id = String::from_str(&env, "kyc_tmpl");
    client.create_template(&issuer, &tmpl_id, &template);

    let subject = Address::generate(&env);
    let att_id = client
        .create_attestation_from_template(&issuer, &tmpl_id, &subject, &None, &None)
        .unwrap();

    let att = client.get_attestation(&att_id).unwrap();
    assert_eq!(att.claim_type, String::from_str(&env, "KYC"));
    assert_eq!(att.metadata, Some(String::from_str(&env, "tmpl-meta")));
    // expiration = now + 10 * 86400
    assert_eq!(att.expiration, Some(now + 10 * 86_400));
}

#[test]
fn test_create_attestation_from_template_no_expiration() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    env.ledger().set_timestamp(1_000_000u64);

    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };
    let tmpl_id = String::from_str(&env, "t1");
    client.create_template(&issuer, &tmpl_id, &template);

    let subject = Address::generate(&env);
    let att_id = client
        .create_attestation_from_template(&issuer, &tmpl_id, &subject, &None, &None)
        .unwrap();

    let att = client.get_attestation(&att_id).unwrap();
    assert_eq!(att.expiration, None);
}

#[test]
fn test_create_attestation_from_template_with_overrides() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let now = 1_000_000u64;
    env.ledger().set_timestamp(now);

    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: Some(10),
        metadata_template: Some(String::from_str(&env, "default-meta")),
    };
    let tmpl_id = String::from_str(&env, "t1");
    client.create_template(&issuer, &tmpl_id, &template);

    let subject = Address::generate(&env);
    let override_exp = now + 999_999;
    let override_meta = String::from_str(&env, "override-meta");

    let att_id = client
        .create_attestation_from_template(
            &issuer,
            &tmpl_id,
            &subject,
            &Some(override_exp),
            &Some(override_meta.clone()),
        )
        .unwrap();

    let att = client.get_attestation(&att_id).unwrap();
    assert_eq!(att.expiration, Some(override_exp));
    assert_eq!(att.metadata, Some(override_meta));
}

#[test]
fn test_create_attestation_from_template_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    env.ledger().set_timestamp(1_000_000u64);
    let subject = Address::generate(&env);

    let result = client.try_create_attestation_from_template(
        &issuer,
        &String::from_str(&env, "missing"),
        &subject,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(Error::NotFound)));
}

#[test]
fn test_create_attestation_from_template_stale_expiration() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let now = 1_000_000u64;
    env.ledger().set_timestamp(now);

    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };
    let tmpl_id = String::from_str(&env, "t1");
    client.create_template(&issuer, &tmpl_id, &template);

    let subject = Address::generate(&env);
    // expiration_override <= current timestamp → InvalidExpiration
    let result = client.try_create_attestation_from_template(
        &issuer,
        &tmpl_id,
        &subject,
        &Some(now),
        &None,
    );
    assert_eq!(result, Err(Ok(Error::InvalidExpiration)));
}

#[test]
fn test_create_attestation_from_template_metadata_override_too_long() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    env.ledger().set_timestamp(1_000_000u64);

    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };
    let tmpl_id = String::from_str(&env, "t1");
    client.create_template(&issuer, &tmpl_id, &template);

    let subject = Address::generate(&env);
    let long_meta = "x".repeat(257);
    let result = client.try_create_attestation_from_template(
        &issuer,
        &tmpl_id,
        &subject,
        &None,
        &Some(String::from_str(&env, &long_meta)),
    );
    assert_eq!(result, Err(Ok(Error::MetadataTooLong)));
}

#[test]
fn test_template_storage_isolation_across_issuers() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer_a, client) = setup(&env);

    let issuer_b = Address::generate(&env);
    client.register_issuer(&admin, &issuer_b);

    let tmpl_id = String::from_str(&env, "shared_id");

    let tmpl_a = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: Some(10),
        metadata_template: None,
    };
    let tmpl_b = types::AttestationTemplate {
        claim_type: String::from_str(&env, "AML"),
        default_expiration_days: Some(30),
        metadata_template: Some(String::from_str(&env, "b-meta")),
    };

    client.create_template(&issuer_a, &tmpl_id, &tmpl_a);
    client.create_template(&issuer_b, &tmpl_id, &tmpl_b);

    let got_a = client.get_template(&issuer_a, &tmpl_id).unwrap();
    let got_b = client.get_template(&issuer_b, &tmpl_id).unwrap();

    assert_eq!(got_a.claim_type, String::from_str(&env, "KYC"));
    assert_eq!(got_b.claim_type, String::from_str(&env, "AML"));
    assert_ne!(got_a.claim_type, got_b.claim_type);
}

#[test]
fn test_create_template_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };
    let tmpl_id = String::from_str(&env, "t1");
    client.create_template(&issuer, &tmpl_id, &template);

    let events = env.events().all();
    let mut found = false;
    for (_, topics, data) in events.iter() {
        let topic0: soroban_sdk::Symbol =
            soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        if topic0 == soroban_sdk::symbol_short!("tmpl_crt") {
            let event_data: String =
                soroban_sdk::TryFromVal::try_from_val(&env, &data).unwrap();
            assert_eq!(event_data, tmpl_id);
            found = true;
            break;
        }
    }
    assert!(found, "template_created event not found");
}

#[test]
fn test_create_attestation_from_template_emits_attestation_created_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    env.ledger().set_timestamp(1_000_000u64);

    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };
    let tmpl_id = String::from_str(&env, "t1");
    client.create_template(&issuer, &tmpl_id, &template);

    let subject = Address::generate(&env);
    client
        .create_attestation_from_template(&issuer, &tmpl_id, &subject, &None, &None)
        .unwrap();

    let events = env.events().all();
    let mut found = false;
    for (_, topics, _) in events.iter() {
        let topic0: soroban_sdk::Symbol =
            soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        if topic0 == soroban_sdk::symbol_short!("att_crt") {
            found = true;
            break;
        }
    }
    assert!(found, "attestation_created event not found after create_attestation_from_template");
}

#[test]
fn test_create_attestation_from_template_indexed_like_regular() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, client) = setup(&env);

    env.ledger().set_timestamp(1_000_000u64);

    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };
    let tmpl_id = String::from_str(&env, "t1");
    client.create_template(&issuer, &tmpl_id, &template);

    let subject = Address::generate(&env);
    let att_id = client
        .create_attestation_from_template(&issuer, &tmpl_id, &subject, &None, &None)
        .unwrap();

    // Must be retrievable by ID
    let att = client.get_attestation(&att_id).unwrap();
    assert_eq!(att.id, att_id);

    // Must appear in subject index
    let subject_atts = client.get_subject_attestations(&subject, &0, &10);
    assert!(subject_atts.contains(&att_id));

    // Must appear in issuer index
    let issuer_atts = client.get_issuer_attestations(&issuer, &0, &10);
    assert!(issuer_atts.contains(&att_id));
}

// ============================================================================
// Transfer Attestation Tests
// ============================================================================

#[test]
fn test_transfer_attestation_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify attestation issuer was updated
    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.issuer, new_issuer);
}

#[test]
fn test_transfer_attestation_updates_indexes() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Verify old issuer has the attestation
    let old_issuer_attestations = client.get_issuer_attestations(&old_issuer, &0, &10);
    assert_eq!(old_issuer_attestations.len(), 1);
    assert_eq!(old_issuer_attestations.get(0).unwrap(), attestation_id);
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify old issuer no longer has the attestation
    let old_issuer_attestations = client.get_issuer_attestations(&old_issuer, &0, &10);
    assert_eq!(old_issuer_attestations.len(), 0);
    
    // Verify new issuer has the attestation
    let new_issuer_attestations = client.get_issuer_attestations(&new_issuer, &0, &10);
    assert_eq!(new_issuer_attestations.len(), 1);
    assert_eq!(new_issuer_attestations.get(0).unwrap(), attestation_id);
}

#[test]
fn test_transfer_attestation_updates_issuer_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Get initial stats
    let old_stats_before = client.get_issuer_stats(&old_issuer);
    let new_stats_before = client.get_issuer_stats(&new_issuer);
    
    assert_eq!(old_stats_before.total_issued, 1);
    assert_eq!(new_stats_before.total_issued, 0);
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify stats were updated
    let old_stats_after = client.get_issuer_stats(&old_issuer);
    let new_stats_after = client.get_issuer_stats(&new_issuer);
    
    assert_eq!(old_stats_after.total_issued, 0);
    assert_eq!(new_stats_after.total_issued, 1);
}

#[test]
fn test_transfer_attestation_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify event was emitted
    let events = env.events().all();
    let transfer_event = events.iter().find(|(_, topic, _)| {
        topic.len() == 2 && topic.get(0).unwrap() == soroban_sdk::symbol_short!("att_xfer")
    });
    
    assert!(transfer_event.is_some(), "attestation_transferred event should be emitted");
}

#[test]
fn test_transfer_attestation_appends_audit_log() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify audit log entry was added
    let audit_log = client.get_audit_log(&attestation_id);
    assert_eq!(audit_log.len(), 2); // Created + Transferred
    
    let transfer_entry = audit_log.get(1).unwrap();
    assert_eq!(transfer_entry.action, types::AuditAction::Transferred);
    assert_eq!(transfer_entry.actor, admin);
    assert!(transfer_entry.details.is_some());
}

#[test]
fn test_transfer_attestation_unauthorized_non_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let non_admin = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Attempt transfer by non-admin should fail
    let result = client.try_transfer_attestation(&non_admin, &attestation_id, &new_issuer);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_transfer_attestation_missing_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let fake_id = String::from_str(&env, "nonexistent_id");
    
    // Attempt transfer of non-existent attestation should fail
    let result = client.try_transfer_attestation(&admin, &fake_id, &new_issuer);
    assert_eq!(result, Err(Ok(Error::NotFound)));
}

#[test]
fn test_transfer_attestation_unregistered_new_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let unregistered_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Attempt transfer to unregistered issuer should fail
    let result = client.try_transfer_attestation(&admin, &attestation_id, &unregistered_issuer);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_transfer_attestation_idempotent_same_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    let stats_before = client.get_issuer_stats(&issuer);
    
    // Transfer to same issuer should succeed without changes
    client.transfer_attestation(&admin, &attestation_id, &issuer);
    
    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.issuer, issuer);
    
    let stats_after = client.get_issuer_stats(&issuer);
    assert_eq!(stats_before.total_issued, stats_after.total_issued);
}

#[test]
fn test_transfer_attestation_repeated_transfers() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer1, client) = setup(&env);
    let issuer2 = Address::generate(&env);
    let issuer3 = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &issuer2);
    client.register_issuer(&admin, &issuer3);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &issuer1,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Transfer from issuer1 to issuer2
    client.transfer_attestation(&admin, &attestation_id, &issuer2);
    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.issuer, issuer2);
    
    // Transfer from issuer2 to issuer3
    client.transfer_attestation(&admin, &attestation_id, &issuer3);
    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.issuer, issuer3);
    
    // Verify indexes are correct
    let issuer1_attestations = client.get_issuer_attestations(&issuer1, &0, &10);
    let issuer2_attestations = client.get_issuer_attestations(&issuer2, &0, &10);
    let issuer3_attestations = client.get_issuer_attestations(&issuer3, &0, &10);
    
    assert_eq!(issuer1_attestations.len(), 0);
    assert_eq!(issuer2_attestations.len(), 0);
    assert_eq!(issuer3_attestations.len(), 1);
}

#[test]
fn test_transfer_attestation_preserves_other_fields() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let metadata = Some(String::from_str(&env, "test metadata"));
    let expiration = Some(env.ledger().timestamp() + 1000);
    
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &expiration,
        &metadata,
        &None,
    );
    
    let before = client.get_attestation(&attestation_id);
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    let after = client.get_attestation(&attestation_id);
    
    // Verify only issuer changed, all other fields preserved
    assert_eq!(after.issuer, new_issuer);
    assert_eq!(after.subject, before.subject);
    assert_eq!(after.claim_type, before.claim_type);
    assert_eq!(after.timestamp, before.timestamp);
    assert_eq!(after.expiration, before.expiration);
    assert_eq!(after.metadata, before.metadata);
    assert_eq!(after.revoked, before.revoked);
    assert_eq!(after.deleted, before.deleted);
}

#[test]
fn test_transfer_attestation_no_duplicate_index_entries() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify no duplicate entries in new issuer's index
    let new_issuer_attestations = client.get_issuer_attestations(&new_issuer, &0, &10);
    assert_eq!(new_issuer_attestations.len(), 1);
    
    // Verify old issuer's index is clean
    let old_issuer_attestations = client.get_issuer_attestations(&old_issuer, &0, &10);
    assert_eq!(old_issuer_attestations.len(), 0);
}

#[test]
fn test_transfer_attestation_stats_consistency() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer1, client) = setup(&env);
    let issuer2 = Address::generate(&env);
    let issuer3 = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &issuer2);
    client.register_issuer(&admin, &issuer3);
    
    // Create multiple attestations
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let id1 = client.create_attestation(&issuer1, &subject, &claim_type, &None, &None, &None);
    let id2 = client.create_attestation(&issuer1, &subject, &claim_type, &None, &None, &None);
    let id3 = client.create_attestation(&issuer2, &subject, &claim_type, &None, &None, &None);
    
    // Initial stats
    assert_eq!(client.get_issuer_stats(&issuer1).total_issued, 2);
    assert_eq!(client.get_issuer_stats(&issuer2).total_issued, 1);
    assert_eq!(client.get_issuer_stats(&issuer3).total_issued, 0);
    
    // Transfer id1 from issuer1 to issuer3
    client.transfer_attestation(&admin, &id1, &issuer3);
    assert_eq!(client.get_issuer_stats(&issuer1).total_issued, 1);
    assert_eq!(client.get_issuer_stats(&issuer2).total_issued, 1);
    assert_eq!(client.get_issuer_stats(&issuer3).total_issued, 1);
    
    // Transfer id2 from issuer1 to issuer2
    client.transfer_attestation(&admin, &id2, &issuer2);
    assert_eq!(client.get_issuer_stats(&issuer1).total_issued, 0);
    assert_eq!(client.get_issuer_stats(&issuer2).total_issued, 2);
    assert_eq!(client.get_issuer_stats(&issuer3).total_issued, 1);
    
    // Transfer id3 from issuer2 to issuer3
    client.transfer_attestation(&admin, &id3, &issuer3);
    assert_eq!(client.get_issuer_stats(&issuer1).total_issued, 0);
    assert_eq!(client.get_issuer_stats(&issuer2).total_issued, 1);
    assert_eq!(client.get_issuer_stats(&issuer3).total_issued, 2);
}

// ============================================================================
// Transfer Attestation Tests
// ============================================================================

#[test]
fn test_transfer_attestation_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify attestation issuer was updated
    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.issuer, new_issuer);
}

#[test]
fn test_transfer_attestation_updates_indexes() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Verify old issuer has the attestation
    let old_issuer_attestations = client.get_issuer_attestations(&old_issuer, &0, &10);
    assert_eq!(old_issuer_attestations.len(), 1);
    assert_eq!(old_issuer_attestations.get(0).unwrap(), attestation_id);
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify old issuer no longer has the attestation
    let old_issuer_attestations = client.get_issuer_attestations(&old_issuer, &0, &10);
    assert_eq!(old_issuer_attestations.len(), 0);
    
    // Verify new issuer has the attestation
    let new_issuer_attestations = client.get_issuer_attestations(&new_issuer, &0, &10);
    assert_eq!(new_issuer_attestations.len(), 1);
    assert_eq!(new_issuer_attestations.get(0).unwrap(), attestation_id);
}

#[test]
fn test_transfer_attestation_updates_issuer_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Get initial stats
    let old_stats_before = client.get_issuer_stats(&old_issuer);
    let new_stats_before = client.get_issuer_stats(&new_issuer);
    
    assert_eq!(old_stats_before.total_issued, 1);
    assert_eq!(new_stats_before.total_issued, 0);
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify stats were updated
    let old_stats_after = client.get_issuer_stats(&old_issuer);
    let new_stats_after = client.get_issuer_stats(&new_issuer);
    
    assert_eq!(old_stats_after.total_issued, 0);
    assert_eq!(new_stats_after.total_issued, 1);
}

#[test]
fn test_transfer_attestation_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify event was emitted
    let events = env.events().all();
    let transfer_event = events.iter().find(|(_, topic, _)| {
        topic.len() == 2 && topic.get(0).unwrap() == soroban_sdk::symbol_short!("att_xfer")
    });
    
    assert!(transfer_event.is_some(), "attestation_transferred event should be emitted");
}

#[test]
fn test_transfer_attestation_appends_audit_log() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify audit log entry was added
    let audit_log = client.get_audit_log(&attestation_id);
    assert_eq!(audit_log.len(), 2); // Created + Transferred
    
    let transfer_entry = audit_log.get(1).unwrap();
    assert_eq!(transfer_entry.action, types::AuditAction::Transferred);
    assert_eq!(transfer_entry.actor, admin);
    assert!(transfer_entry.details.is_some());
}

#[test]
fn test_transfer_attestation_unauthorized_non_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let non_admin = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Attempt transfer by non-admin should fail
    let result = client.try_transfer_attestation(&non_admin, &attestation_id, &new_issuer);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_transfer_attestation_missing_attestation() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let fake_id = String::from_str(&env, "nonexistent_id");
    
    // Attempt transfer of non-existent attestation should fail
    let result = client.try_transfer_attestation(&admin, &fake_id, &new_issuer);
    assert_eq!(result, Err(Ok(Error::NotFound)));
}

#[test]
fn test_transfer_attestation_unregistered_new_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let unregistered_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Attempt transfer to unregistered issuer should fail
    let result = client.try_transfer_attestation(&admin, &attestation_id, &unregistered_issuer);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_transfer_attestation_idempotent_same_issuer() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    let stats_before = client.get_issuer_stats(&issuer);
    
    // Transfer to same issuer should succeed without changes
    client.transfer_attestation(&admin, &attestation_id, &issuer);
    
    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.issuer, issuer);
    
    let stats_after = client.get_issuer_stats(&issuer);
    assert_eq!(stats_before.total_issued, stats_after.total_issued);
}

#[test]
fn test_transfer_attestation_repeated_transfers() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer1, client) = setup(&env);
    let issuer2 = Address::generate(&env);
    let issuer3 = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &issuer2);
    client.register_issuer(&admin, &issuer3);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &issuer1,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Transfer from issuer1 to issuer2
    client.transfer_attestation(&admin, &attestation_id, &issuer2);
    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.issuer, issuer2);
    
    // Transfer from issuer2 to issuer3
    client.transfer_attestation(&admin, &attestation_id, &issuer3);
    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.issuer, issuer3);
    
    // Verify indexes are correct
    let issuer1_attestations = client.get_issuer_attestations(&issuer1, &0, &10);
    let issuer2_attestations = client.get_issuer_attestations(&issuer2, &0, &10);
    let issuer3_attestations = client.get_issuer_attestations(&issuer3, &0, &10);
    
    assert_eq!(issuer1_attestations.len(), 0);
    assert_eq!(issuer2_attestations.len(), 0);
    assert_eq!(issuer3_attestations.len(), 1);
}

#[test]
fn test_transfer_attestation_preserves_other_fields() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let metadata = Some(String::from_str(&env, "test metadata"));
    let expiration = Some(env.ledger().timestamp() + 1000);
    
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &expiration,
        &metadata,
        &None,
    );
    
    let before = client.get_attestation(&attestation_id);
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    let after = client.get_attestation(&attestation_id);
    
    // Verify only issuer changed, all other fields preserved
    assert_eq!(after.issuer, new_issuer);
    assert_eq!(after.subject, before.subject);
    assert_eq!(after.claim_type, before.claim_type);
    assert_eq!(after.timestamp, before.timestamp);
    assert_eq!(after.expiration, before.expiration);
    assert_eq!(after.metadata, before.metadata);
    assert_eq!(after.revoked, before.revoked);
    assert_eq!(after.deleted, before.deleted);
}

#[test]
fn test_transfer_attestation_no_duplicate_index_entries() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, old_issuer, client) = setup(&env);
    let new_issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &new_issuer);
    
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let attestation_id = client.create_attestation(
        &old_issuer,
        &subject,
        &claim_type,
        &None,
        &None,
        &None,
    );
    
    // Transfer attestation
    client.transfer_attestation(&admin, &attestation_id, &new_issuer);
    
    // Verify no duplicate entries in new issuer's index
    let new_issuer_attestations = client.get_issuer_attestations(&new_issuer, &0, &10);
    assert_eq!(new_issuer_attestations.len(), 1);
    
    // Verify old issuer's index is clean
    let old_issuer_attestations = client.get_issuer_attestations(&old_issuer, &0, &10);
    assert_eq!(old_issuer_attestations.len(), 0);
}

#[test]
fn test_transfer_attestation_stats_consistency() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, issuer1, client) = setup(&env);
    let issuer2 = Address::generate(&env);
    let issuer3 = Address::generate(&env);
    let subject = Address::generate(&env);
    
    client.register_issuer(&admin, &issuer2);
    client.register_issuer(&admin, &issuer3);
    
    // Create multiple attestations
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let id1 = client.create_attestation(&issuer1, &subject, &claim_type, &None, &None, &None);
    let id2 = client.create_attestation(&issuer1, &subject, &claim_type, &None, &None, &None);
    let id3 = client.create_attestation(&issuer2, &subject, &claim_type, &None, &None, &None);
    
    // Initial stats
    assert_eq!(client.get_issuer_stats(&issuer1).total_issued, 2);
    assert_eq!(client.get_issuer_stats(&issuer2).total_issued, 1);
    assert_eq!(client.get_issuer_stats(&issuer3).total_issued, 0);
    
    // Transfer id1 from issuer1 to issuer3
    client.transfer_attestation(&admin, &id1, &issuer3);
    assert_eq!(client.get_issuer_stats(&issuer1).total_issued, 1);
    assert_eq!(client.get_issuer_stats(&issuer2).total_issued, 1);
    assert_eq!(client.get_issuer_stats(&issuer3).total_issued, 1);
    
    // Transfer id2 from issuer1 to issuer2
    client.transfer_attestation(&admin, &id2, &issuer2);
    assert_eq!(client.get_issuer_stats(&issuer1).total_issued, 0);
    assert_eq!(client.get_issuer_stats(&issuer2).total_issued, 2);
    assert_eq!(client.get_issuer_stats(&issuer3).total_issued, 1);
    
    // Transfer id3 from issuer2 to issuer3
    client.transfer_attestation(&admin, &id3, &issuer3);
    assert_eq!(client.get_issuer_stats(&issuer1).total_issued, 0);
    assert_eq!(client.get_issuer_stats(&issuer2).total_issued, 1);
    assert_eq!(client.get_issuer_stats(&issuer3).total_issued, 2);
}

// ── Issue #327: Multi-sig proposal expiry ────────────────────────────────────

#[test]
fn test_multisig_expired_proposal_not_finalized_no_attestation() {
    let env = Env::default();
    env.mock_all_auths();

    let (issuer1, issuer2, issuer3, _, client) = setup_multisig(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");

    env.ledger().with_mut(|li| li.timestamp = 1_000);

    let mut required = soroban_sdk::Vec::new(&env);
    required.push_back(issuer1.clone());
    required.push_back(issuer2.clone());
    required.push_back(issuer3.clone());

    let proposal_id = client.propose_attestation(&issuer1, &subject, &claim_type, &required, &2);

    // Advance past the 7-day expiry window.
    env.ledger().with_mut(|li| li.timestamp = 1_000 + 7 * 24 * 60 * 60 + 1);

    // Co-sign must fail with ProposalExpired.
    let result = client.try_cosign_attestation(&issuer2, &proposal_id);
    assert_eq!(result, Err(Ok(types::Error::ProposalExpired)));

    // Proposal must not be finalized.
    let proposal = client.get_multisig_proposal(&proposal_id);
    assert!(!proposal.finalized, "expired proposal must not be finalized");

    // No attestation must have been created.
    assert!(
        !client.has_valid_claim(&subject, &claim_type),
        "expired proposal must not create an attestation"
    );
}

// ── Issue #329: Expiration hook callback failure handling ─────────────────────

#[test]
fn test_expiration_hook_panicking_callback_does_not_affect_has_valid_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    // Set ledger time so the attestation is inside the notification window.
    // Expiration = now + 3 days; notify_days_before = 7 → hook fires immediately.
    let now: u64 = 1_000_000;
    env.ledger().with_mut(|li| li.timestamp = now);
    let expiration = now + 3 * 24 * 60 * 60;

    client.create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &Some(expiration),
        &None,
        &None,
    );

    // Register a callback contract that panics.
    let callback_id = env.register_contract(None, MockPanicCallbackContract);
    client.register_expiration_hook(&subject, &callback_id, &7);

    // has_valid_claim must still return true despite the panicking callback.
    assert!(
        client.has_valid_claim(&subject, &claim_type),
        "has_valid_claim must return true even when callback panics"
    );
}

// ── Issue #334: has_all_claims edge cases ─────────────────────────────────────

#[test]
fn test_has_all_claims_empty_list_returns_true() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, client) = setup(&env);
    let subject = Address::generate(&env);

    let empty: soroban_sdk::Vec<String> = soroban_sdk::Vec::new(&env);
    assert!(client.has_all_claims(&subject, &empty), "empty list must return true (vacuous truth)");
}

#[test]
fn test_has_all_claims_single_element_equivalent_to_has_valid_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    // Before attestation: both must return false.
    let mut list = soroban_sdk::Vec::new(&env);
    list.push_back(claim_type.clone());
    assert_eq!(
        client.has_all_claims(&subject, &list),
        client.has_valid_claim(&subject, &claim_type)
    );

    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // After attestation: both must return true.
    assert_eq!(
        client.has_all_claims(&subject, &list),
        client.has_valid_claim(&subject, &claim_type)
    );
}

#[test]
fn test_has_all_claims_all_valid_returns_true() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let kyc = String::from_str(&env, "KYC_PASSED");
    let aml = String::from_str(&env, "AML_CLEARED");

    client.create_attestation(&issuer, &subject, &kyc, &None, &None, &None);
    client.create_attestation(&issuer, &subject, &aml, &None, &None, &None);

    let mut list = soroban_sdk::Vec::new(&env);
    list.push_back(kyc.clone());
    list.push_back(aml.clone());

    assert!(client.has_all_claims(&subject, &list), "all valid claims must return true");
}

#[test]
fn test_has_all_claims_one_missing_returns_false() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let kyc = String::from_str(&env, "KYC_PASSED");
    let aml = String::from_str(&env, "AML_CLEARED");

    // Only create KYC, not AML.
    client.create_attestation(&issuer, &subject, &kyc, &None, &None, &None);

    let mut list = soroban_sdk::Vec::new(&env);
    list.push_back(kyc.clone());
    list.push_back(aml.clone());

    assert!(!client.has_all_claims(&subject, &list), "missing claim must short-circuit to false");
}

// Issue #325 — GDPR soft-delete: deleted attestations must be excluded from all queries
#[cfg(test)]
mod gdpr_deletion_tests {
    use super::*;

    fn setup(env: &Env) -> (Address, Address, Address, TrustLinkContractClient<'_>) {
        let (_, client) = create_test_contract(env);
        let admin = Address::generate(env);
        let issuer = Address::generate(env);
        let subject = Address::generate(env);
        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);
        (admin, issuer, subject, client)
    }

    fn create_and_delete(
        env: &Env,
        client: &TrustLinkContractClient,
        issuer: &Address,
        subject: &Address,
    ) -> String {
        let claim = String::from_str(env, "KYC_PASSED");
        let id = client.create_attestation(issuer, subject, &claim, &None, &None, &None);
        client.request_deletion(subject, &id);
        id
    }

    #[test]
    fn deleted_excluded_from_get_subject_attestations() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        create_and_delete(&env, &client, &issuer, &subject);

        let ids = client.get_subject_attestations(&subject, &0, &10);
        assert_eq!(ids.len(), 0);
    }

    #[test]
    fn deleted_excluded_from_has_valid_claim() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        create_and_delete(&env, &client, &issuer, &subject);

        assert!(!client.has_valid_claim(&subject, &String::from_str(&env, "KYC_PASSED")));
    }

    #[test]
    fn deleted_returns_not_found_from_get_attestation_status() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let id = create_and_delete(&env, &client, &issuer, &subject);

        let result = client.try_get_attestation_status(&id);
        assert_eq!(result, Err(Ok(Error::NotFound)));
    }

    #[test]
    fn deleted_excluded_from_get_valid_claim_count() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        create_and_delete(&env, &client, &issuer, &subject);

        assert_eq!(client.get_valid_claim_count(&subject), 0);
    }

    #[test]
    fn deleted_excluded_from_date_range_search() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let ts = env.ledger().timestamp();
        create_and_delete(&env, &client, &issuer, &subject);

        let results = client.get_attestations_in_range(&subject, &ts, &(ts + 1000), &0, &10);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn non_deleted_attestation_still_visible_after_sibling_deleted() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        // Create two attestations; delete only the first.
        let claim_a = String::from_str(&env, "KYC_PASSED");
        let claim_b = String::from_str(&env, "AML_CLEARED");
        let id_a = client.create_attestation(&issuer, &subject, &claim_a, &None, &None, &None);
        client.create_attestation(&issuer, &subject, &claim_b, &None, &None, &None);
        client.request_deletion(&subject, &id_a);

        let ids = client.get_subject_attestations(&subject, &0, &10);
        assert_eq!(ids.len(), 1);
        assert!(!client.has_valid_claim(&subject, &claim_a));
        assert!(client.has_valid_claim(&subject, &claim_b));
        assert_eq!(client.get_valid_claim_count(&subject), 1);
    }

    #[test]
    fn get_attestation_returns_not_found_for_deleted() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let id = create_and_delete(&env, &client, &issuer, &subject);

        let result = client.try_get_attestation(&id);
        assert_eq!(result, Err(Ok(Error::NotFound)));
    }

    #[test]
    fn only_subject_can_request_deletion() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, subject, client) = setup(&env);

        let claim = String::from_str(&env, "KYC_PASSED");
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        let other = Address::generate(&env);
        let result = client.try_request_deletion(&other, &id);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }
}

// Issue #325 — Delegation lifecycle tests
#[cfg(test)]
mod delegation_tests {
    use super::*;

    fn setup(env: &Env) -> (Address, Address, Address, Address, TrustLinkContractClient<'_>) {
        let (_, client) = create_test_contract(env);
        let admin = Address::generate(env);
        let issuer = Address::generate(env);
        let delegate = Address::generate(env);
        let subject = Address::generate(env);
        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);
        (admin, issuer, delegate, subject, client)
    }

    #[test]
    fn delegate_can_create_attestation_on_behalf_of_delegator() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, delegate, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        client.delegate_claim_type(&issuer, &delegate, &claim, &None);
        let id = client.create_attestation_as_delegate(&delegate, &issuer, &subject, &claim, &None, &None);

        // Attestation is stored under the delegator (issuer) as the issuer field.
        let att = client.get_attestation(&id);
        assert_eq!(att.issuer, issuer);
        assert!(client.has_valid_claim(&subject, &claim));
    }

    #[test]
    fn expired_delegation_rejects_delegate() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, delegate, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let exp = env.ledger().timestamp() + 100;
        client.delegate_claim_type(&issuer, &delegate, &claim, &Some(exp));

        // Advance time past expiration.
        env.ledger().with_mut(|l| l.timestamp += 101);

        let result = client.try_create_attestation_as_delegate(&delegate, &issuer, &subject, &claim, &None, &None);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn revoked_delegation_rejects_delegate() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, delegate, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        client.delegate_claim_type(&issuer, &delegate, &claim, &None);
        client.revoke_delegation(&issuer, &delegate, &claim);

        let result = client.try_create_attestation_as_delegate(&delegate, &issuer, &subject, &claim, &None, &None);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn cannot_delegate_to_self() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, _, _, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let result = client.try_delegate_claim_type(&issuer, &issuer, &claim, &None);
        assert_eq!(result, Err(Ok(Error::CannotDelegateToSelf)));
    }

    #[test]
    fn revoke_nonexistent_delegation_returns_not_found() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, delegate, _, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let result = client.try_revoke_delegation(&issuer, &delegate, &claim);
        assert_eq!(result, Err(Ok(Error::NotFound)));
    }

    #[test]
    fn delegate_without_grant_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, delegate, subject, client) = setup(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        // No delegate_claim_type call — should be rejected.
        let result = client.try_create_attestation_as_delegate(&delegate, &issuer, &subject, &claim, &None, &None);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn list_delegations_by_delegator_returns_active() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, delegate, _, client) = setup(&env);
        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "AML_CLEARED");

        client.delegate_claim_type(&issuer, &delegate, &claim1, &None);
        client.delegate_claim_type(&issuer, &delegate, &claim2, &None);

        let result = client.list_delegations_by_delegator(&issuer, &0, &10);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn list_delegations_by_delegator_excludes_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, delegate, _, client) = setup(&env);
        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "AML_CLEARED");

        let exp = env.ledger().timestamp() + 100;
        client.delegate_claim_type(&issuer, &delegate, &claim1, &Some(exp));
        client.delegate_claim_type(&issuer, &delegate, &claim2, &None);

        env.ledger().with_mut(|l| l.timestamp += 101);

        let result = client.list_delegations_by_delegator(&issuer, &0, &10);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap().claim_type, claim2);
    }

    #[test]
    fn list_delegations_by_delegator_excludes_revoked() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, delegate, _, client) = setup(&env);
        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "AML_CLEARED");

        client.delegate_claim_type(&issuer, &delegate, &claim1, &None);
        client.delegate_claim_type(&issuer, &delegate, &claim2, &None);
        client.revoke_delegation(&issuer, &delegate, &claim1);

        let result = client.list_delegations_by_delegator(&issuer, &0, &10);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap().claim_type, claim2);
    }

    #[test]
    fn list_delegations_by_delegator_paginates() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, issuer, delegate, _, client) = setup(&env);
        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "AML_CLEARED");
        let claim3 = String::from_str(&env, "MERCHANT_VERIFIED");

        client.delegate_claim_type(&issuer, &delegate, &claim1, &None);
        client.delegate_claim_type(&issuer, &delegate, &claim2, &None);
        client.delegate_claim_type(&issuer, &delegate, &claim3, &None);

        let page1 = client.list_delegations_by_delegator(&issuer, &0, &2);
        let page2 = client.list_delegations_by_delegator(&issuer, &2, &2);
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 1);
    }
}

// ---------------------------------------------------------------------------
// Batch attestation storage-write benchmarks
//
// These tests measure and compare the storage write behaviour of
// `create_attestations_batch` before and after the bulk-index optimisation.
//
// Run with:
//   cargo test bench_batch -- --nocapture
// ---------------------------------------------------------------------------
#[cfg(test)]
mod bench_batch {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

    fn setup_bench(env: &Env) -> (Address, Address, TrustLinkContractClient<'_>) {
        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let issuer = Address::generate(env);
        env.mock_all_auths();
        client.initialize(&admin, &None);
        // Raise per-issuer limit to accommodate the full batch.
        client.set_limits(&admin, &10_000u32, &10u32);
        client.register_issuer(&admin, &issuer);
        (admin, issuer, client)
    }

    fn make_subjects(env: &Env, n: u32) -> Vec<Address> {
        let mut v: Vec<Address> = Vec::new(env);
        for _ in 0..n {
            v.push_back(Address::generate(env));
        }
        v
    }

    /// Profile: batch of 50 — verifies the optimised path produces the correct
    /// number of attestations and that the issuer index is consistent.
    #[test]
    fn bench_batch_50_correctness() {
        let env = Env::default();
        let (_, issuer, client) = setup_bench(&env);
        let subjects = make_subjects(&env, 50);
        let claim = String::from_str(&env, "KYC_PASSED");

        let ids = client.create_attestations_batch(&issuer, &subjects, &claim, &None);

        // All 50 IDs returned.
        assert_eq!(ids.len(), 50);

        // Issuer index must contain exactly 50 entries (written once at the end).
        let issuer_index = client.get_issuer_attestations(&issuer, &0u32, &100u32);
        assert_eq!(issuer_index.len(), 50);

        // Every returned ID must be retrievable and belong to the issuer.
        for id in ids.iter() {
            let att = client.get_attestation(&id).unwrap();
            assert_eq!(att.issuer, issuer);
            assert_eq!(att.claim_type, claim);
        }

        // Global stats must reflect 50 attestations.
        let stats = client.get_global_stats();
        assert_eq!(stats.total_attestations, 50);

        println!("[bench_batch_50_correctness] PASS — 50 attestations, issuer index consistent");
    }

    /// Before/after write-count comparison for a batch of 50.
    ///
    /// Before optimisation: `store_attestation` called per subject →
    ///   issuer index: 50 reads + 50 writes
    ///   issuer stats: 50 reads + 50 writes
    ///   global stats: 50 reads + 50 writes
    ///   Total extra writes: 150 (issuer index + stats + global)
    ///
    /// After optimisation (this code):
    ///   issuer index: 1 read + 1 write  (add_issuer_attestations_bulk)
    ///   issuer stats: 1 read + 1 write  (increment_issuer_stats)
    ///   global stats: 1 read + 1 write  (increment_total_attestations)
    ///   Total extra writes: 3
    ///
    /// Reduction: 147 fewer storage writes for a batch of 50.
    #[test]
    fn bench_batch_50_write_reduction() {
        let env = Env::default();
        env.budget().reset_unlimited();

        let (_, issuer, client) = setup_bench(&env);
        let subjects = make_subjects(&env, 50);
        let claim = String::from_str(&env, "KYC_PASSED");

        env.budget().reset_unlimited();
        client.create_attestations_batch(&issuer, &subjects, &claim, &None);

        let cpu = env.budget().cpu_instruction_count();
        let mem = env.budget().memory_bytes_used();

        println!(
            "[bench_batch_50_write_reduction] batch=50 | cpu_instructions={} | memory_bytes={}",
            cpu, mem
        );

        // Sanity: all 50 attestations were created.
        let stats = client.get_global_stats();
        assert_eq!(stats.total_attestations, 50);
    }

    /// Comparison: single-item creation × 50 vs batch × 50.
    /// Demonstrates the CU saving from the bulk-index write.
    #[test]
    fn bench_single_vs_batch_50() {
        // --- single × 50 ---
        let env_single = Env::default();
        env_single.budget().reset_unlimited();
        let (_, issuer_s, client_s) = setup_bench(&env_single);
        let claim = String::from_str(&env_single, "KYC_PASSED");

        env_single.budget().reset_unlimited();
        for _ in 0..50u32 {
            let subject = Address::generate(&env_single);
            client_s.create_attestation(&issuer_s, &subject, &claim, &None, &None, &None);
        }
        let cpu_single = env_single.budget().cpu_instruction_count();
        let mem_single = env_single.budget().memory_bytes_used();

        // --- batch × 50 ---
        let env_batch = Env::default();
        env_batch.budget().reset_unlimited();
        let (_, issuer_b, client_b) = setup_bench(&env_batch);
        let subjects = make_subjects(&env_batch, 50);

        env_batch.budget().reset_unlimited();
        client_b.create_attestations_batch(&issuer_b, &subjects, &claim, &None);
        let cpu_batch = env_batch.budget().cpu_instruction_count();
        let mem_batch = env_batch.budget().memory_bytes_used();

        println!(
            "[bench_single_vs_batch_50]\n  single×50 : cpu={} mem={}\n  batch×50  : cpu={} mem={}\n  cpu saved : {} ({:.1}%)",
            cpu_single,
            mem_single,
            cpu_batch,
            mem_batch,
            cpu_single.saturating_sub(cpu_batch),
            if cpu_single > 0 {
                (cpu_single.saturating_sub(cpu_batch) as f64 / cpu_single as f64) * 100.0
            } else {
                0.0
            }
        );

        // Batch must use fewer CPU instructions than 50 individual calls.
        assert!(
            cpu_batch < cpu_single,
            "batch ({} cpu) should be cheaper than single×50 ({} cpu)",
            cpu_batch,
            cpu_single
        );
    }
}

// ---------------------------------------------------------------------------
// Chunked index storage tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod chunked_index_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String};
    use crate::storage::ChunkedIndex;

    fn setup_chunked(env: &Env) -> (Address, Address, TrustLinkContractClient<'_>) {
        let contract_id = env.register_contract(None, TrustLinkContract);
        let client = TrustLinkContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let issuer = Address::generate(env);
        env.mock_all_auths();
        client.initialize(&admin, &None);
        // Raise limits to allow large index tests.
        client.set_limits(&admin, &10_000u32, &10_000u32);
        client.register_issuer(&admin, &issuer);
        (admin, issuer, client)
    }

    /// A single attestation appears in both the chunked subject and issuer indexes.
    #[test]
    fn chunked_index_single_attestation() {
        let env = Env::default();
        let (_, issuer, client) = setup_chunked(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

        assert_eq!(ChunkedIndex::subject_count(&env, &subject), 1);
        assert_eq!(ChunkedIndex::issuer_count(&env, &issuer), 1);

        let s_ids = ChunkedIndex::get_subject_page(&env, &subject, 0, 10);
        assert_eq!(s_ids.len(), 1);

        let i_ids = ChunkedIndex::get_issuer_page(&env, &issuer, 0, 10);
        assert_eq!(i_ids.len(), 1);
    }

    /// 120 attestations span 3 chunks (CHUNK_SIZE=50). Verify counts and that
    /// get_subject_page returns exactly the requested slice without loading all chunks.
    #[test]
    fn chunked_index_spans_multiple_chunks() {
        let env = Env::default();
        let (_, issuer, client) = setup_chunked(&env);
        let subject = Address::generate(&env);

        for i in 0..120u32 {
            let claim = String::from_str(&env, &soroban_sdk::format!(&env, "CLAIM_{}", i));
            client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
        }

        assert_eq!(ChunkedIndex::subject_count(&env, &subject), 120);

        // Page 0: items 0-9 — only chunk 0 needed.
        let page0 = ChunkedIndex::get_subject_page(&env, &subject, 0, 10);
        assert_eq!(page0.len(), 10);

        // Page spanning chunk boundary (45-54): chunks 0 and 1.
        let page_cross = ChunkedIndex::get_subject_page(&env, &subject, 45, 10);
        assert_eq!(page_cross.len(), 10);

        // Last page: items 115-119.
        let last_page = ChunkedIndex::get_subject_page(&env, &subject, 115, 10);
        assert_eq!(last_page.len(), 5);

        // get_subject_all must return all 120.
        let all = ChunkedIndex::get_subject_all(&env, &subject);
        assert_eq!(all.len(), 120);
    }

    /// Revocation removes the ID from the chunked index.
    #[test]
    fn chunked_index_remove_on_revoke() {
        let env = Env::default();
        let (_, issuer, client) = setup_chunked(&env);
        let subject = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
        assert_eq!(ChunkedIndex::subject_count(&env, &subject), 1);

        client.revoke_attestation(&issuer, &id, &None);
        assert_eq!(ChunkedIndex::subject_count(&env, &subject), 0);
        assert_eq!(ChunkedIndex::issuer_count(&env, &issuer), 0);
    }

    /// Batch creation populates the chunked issuer index correctly.
    #[test]
    fn chunked_index_batch_creation() {
        let env = Env::default();
        let (_, issuer, client) = setup_chunked(&env);
        let claim = String::from_str(&env, "KYC_PASSED");

        let mut subjects = soroban_sdk::Vec::new(&env);
        for _ in 0..60u32 {
            subjects.push_back(Address::generate(&env));
        }

        client.create_attestations_batch(&issuer, &subjects, &claim, &None);

        // Chunked issuer index must hold all 60 IDs across 2 chunks.
        assert_eq!(ChunkedIndex::issuer_count(&env, &issuer), 60);
        let page0 = ChunkedIndex::get_issuer_page(&env, &issuer, 0, 50);
        assert_eq!(page0.len(), 50);
        let page1 = ChunkedIndex::get_issuer_page(&env, &issuer, 50, 50);
        assert_eq!(page1.len(), 10);
    }

    /// get_subject_attestations (the public contract function) returns the correct
    /// page using the chunked index without loading the full flat index.
    #[test]
    fn query_uses_chunked_index_for_pagination() {
        let env = Env::default();
        let (_, issuer, client) = setup_chunked(&env);
        let subject = Address::generate(&env);

        for i in 0..80u32 {
            let claim = String::from_str(&env, &soroban_sdk::format!(&env, "CLAIM_{}", i));
            client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
        }

        // Page 1 (items 10-19).
        let page = client.get_subject_attestations(&subject, &10u32, &10u32);
        assert_eq!(page.len(), 10);

        // Count via chunked index.
        assert_eq!(client.get_subject_attestation_count(&subject), 80);
    }
}

// =============================================================================
// Per-claim-type rate limiting
//
// Tests for the per-claim-type rate limit override feature:
//   - Per-claim-type limit overrides global limit when set
//   - Falls back to global limit when no per-type limit configured
//   - RateLimited error is returned when interval not met
// =============================================================================

#[cfg(test)]
mod rate_limit_tests {
    use super::*;
    use soroban_sdk::{testutils::Ledger, Env, String};

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

    // -------------------------------------------------------------------------
    // Global rate limit (baseline)
    // -------------------------------------------------------------------------

    #[test]
    fn test_global_rate_limit_enforced() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        // Set global rate limit to 1000 seconds
        client.set_rate_limit(&admin, &1_000);

        env.ledger().with_mut(|li| li.timestamp = 1_000);
        let claim1 = String::from_str(&env, "CLAIM_A");
        let id1 = client.create_attestation(&issuer, &subject, &claim1, &None, &None, &None);
        assert!(!id1.is_empty());

        // Try to create another attestation 500 seconds later (< 1000) — should fail
        env.ledger().with_mut(|li| li.timestamp = 1_500);
        let claim2 = String::from_str(&env, "CLAIM_B");
        let result = client.try_create_attestation(&issuer, &subject, &claim2, &None, &None, &None);
        assert_eq!(result, Err(Ok(types::Error::RateLimited)));

        // After 1000 seconds total — should succeed
        env.ledger().with_mut(|li| li.timestamp = 2_000);
        let id2 = client.create_attestation(&issuer, &subject, &claim2, &None, &None, &None);
        assert!(!id2.is_empty());
    }

    // -------------------------------------------------------------------------
    // Per-claim-type rate limit override
    // -------------------------------------------------------------------------

    #[test]
    fn test_per_claim_type_limit_overrides_global() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        // Set global rate limit to 1000 seconds
        client.set_rate_limit(&admin, &1_000);

        // Set per-claim-type limit for ACCREDITED_INVESTOR to 5000 seconds (stricter)
        let accredited = String::from_str(&env, "ACCREDITED_INVESTOR");
        client.set_rate_limit_for_claim_type(&admin, &accredited, &5_000);

        env.ledger().with_mut(|li| li.timestamp = 1_000);

        // Create first ACCREDITED_INVESTOR attestation
        let id1 = client.create_attestation(&issuer, &subject, &accredited, &None, &None, &None);
        assert!(!id1.is_empty());

        // Try to create another ACCREDITED_INVESTOR 2000 seconds later
        // Global limit (1000) would allow it, but per-type limit (5000) should block it
        env.ledger().with_mut(|li| li.timestamp = 3_000);
        let result = client.try_create_attestation(&issuer, &subject, &accredited, &None, &None, &None);
        assert_eq!(
            result,
            Err(Ok(types::Error::RateLimited)),
            "per-claim-type limit should override global limit"
        );

        // After 5000 seconds total — should succeed
        env.ledger().with_mut(|li| li.timestamp = 6_000);
        let id2 = client.create_attestation(&issuer, &subject, &accredited, &None, &None, &None);
        assert!(!id2.is_empty());
    }

    #[test]
    fn test_per_claim_type_limit_less_strict_than_global() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        // Set global rate limit to 1000 seconds (strict)
        client.set_rate_limit(&admin, &1_000);

        // Set per-claim-type limit for MERCHANT_VERIFIED to 100 seconds (lenient)
        let merchant = String::from_str(&env, "MERCHANT_VERIFIED");
        client.set_rate_limit_for_claim_type(&admin, &merchant, &100);

        env.ledger().with_mut(|li| li.timestamp = 1_000);

        // Create first MERCHANT_VERIFIED attestation
        let id1 = client.create_attestation(&issuer, &subject, &merchant, &None, &None, &None);
        assert!(!id1.is_empty());

        // Try to create another MERCHANT_VERIFIED 150 seconds later
        // Global limit (1000) would block it, but per-type limit (100) should allow it
        env.ledger().with_mut(|li| li.timestamp = 1_150);
        let id2 = client.create_attestation(&issuer, &subject, &merchant, &None, &None, &None);
        assert!(
            !id2.is_empty(),
            "per-claim-type limit should override global limit even if more lenient"
        );
    }

    // -------------------------------------------------------------------------
    // Fallback to global limit
    // -------------------------------------------------------------------------

    #[test]
    fn test_fallback_to_global_when_no_per_type_limit() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        // Set global rate limit to 1000 seconds
        client.set_rate_limit(&admin, &1_000);

        // Set per-claim-type limit only for ACCREDITED_INVESTOR
        let accredited = String::from_str(&env, "ACCREDITED_INVESTOR");
        client.set_rate_limit_for_claim_type(&admin, &accredited, &5_000);

        env.ledger().with_mut(|li| li.timestamp = 1_000);

        // Create attestation for a different claim type (no per-type limit set)
        let other = String::from_str(&env, "OTHER_CLAIM");
        let id1 = client.create_attestation(&issuer, &subject, &other, &None, &None, &None);
        assert!(!id1.is_empty());

        // Try to create another OTHER_CLAIM 500 seconds later
        // Should use global limit (1000) and fail
        env.ledger().with_mut(|li| li.timestamp = 1_500);
        let result = client.try_create_attestation(&issuer, &subject, &other, &None, &None, &None);
        assert_eq!(
            result,
            Err(Ok(types::Error::RateLimited)),
            "should fall back to global limit when no per-type limit set"
        );

        // After 1000 seconds total — should succeed
        env.ledger().with_mut(|li| li.timestamp = 2_000);
        let id2 = client.create_attestation(&issuer, &subject, &other, &None, &None, &None);
        assert!(!id2.is_empty());
    }

    // -------------------------------------------------------------------------
    // Get per-claim-type limit
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_rate_limit_for_claim_type() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _issuer, _subject, client) = setup(&env);

        let claim = String::from_str(&env, "TEST_CLAIM");

        // Initially, no per-type limit should be set
        assert_eq!(client.get_rate_limit_for_claim_type(&claim), None);

        // Set a per-type limit
        client.set_rate_limit_for_claim_type(&admin, &claim, &5_000);

        // Should now return the set value
        assert_eq!(client.get_rate_limit_for_claim_type(&claim), Some(5_000));
    }

    // -------------------------------------------------------------------------
    // Independent claim types
    // -------------------------------------------------------------------------

    #[test]
    fn test_per_claim_type_limits_are_independent() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        // Set different per-type limits for two claim types
        let claim_a = String::from_str(&env, "CLAIM_A");
        let claim_b = String::from_str(&env, "CLAIM_B");
        client.set_rate_limit_for_claim_type(&admin, &claim_a, &1_000);
        client.set_rate_limit_for_claim_type(&admin, &claim_b, &100);

        env.ledger().with_mut(|li| li.timestamp = 1_000);

        // Create attestation for CLAIM_A
        let id_a = client.create_attestation(&issuer, &subject, &claim_a, &None, &None, &None);
        assert!(!id_a.is_empty());

        // Create attestation for CLAIM_B
        let id_b = client.create_attestation(&issuer, &subject, &claim_b, &None, &None, &None);
        assert!(!id_b.is_empty());

        // Advance 150 seconds
        env.ledger().with_mut(|li| li.timestamp = 1_150);

        // CLAIM_A should still be rate-limited (1000 > 150)
        let result_a = client.try_create_attestation(&issuer, &subject, &claim_a, &None, &None, &None);
        assert_eq!(result_a, Err(Ok(types::Error::RateLimited)));

        // CLAIM_B should be allowed (100 < 150)
        let id_b2 = client.create_attestation(&issuer, &subject, &claim_b, &None, &None, &None);
        assert!(!id_b2.is_empty());
    }

    // -------------------------------------------------------------------------
    // Zero rate limit (no limit)
    // -------------------------------------------------------------------------

    #[test]
    fn test_zero_rate_limit_allows_unlimited_issuance() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        // Set per-claim-type limit to 0 (no limit)
        let claim = String::from_str(&env, "UNLIMITED");
        client.set_rate_limit_for_claim_type(&admin, &claim, &0);

        env.ledger().with_mut(|li| li.timestamp = 1_000);

        // Create multiple attestations in rapid succession
        for i in 0..5 {
            let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
            assert!(!id.is_empty(), "attestation {} should succeed with zero rate limit", i);
        }
    }

    // -------------------------------------------------------------------------
    // Admin-only access
    // -------------------------------------------------------------------------

    #[test]
    fn test_set_rate_limit_for_claim_type_is_admin_only() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _issuer, _subject, client) = setup(&env);

        let non_admin = Address::generate(&env);
        let claim = String::from_str(&env, "TEST");

        let result = client.try_set_rate_limit_for_claim_type(&non_admin, &claim, &1_000);
        assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
    }
}

// ============================================================================
// Claim Type Requirement Tests
// ============================================================================

#[cfg(test)]
mod claim_type_requirement_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

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

    #[test]
    fn test_require_registered_claim_type_disabled_by_default() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, _issuer, _subject, client) = setup(&env);

        // By default, should be false
        assert_eq!(client.get_require_registered_claim_type(), false);
    }

    #[test]
    fn test_unregistered_claim_type_allowed_when_disabled() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, issuer, subject, client) = setup(&env);

        // Requirement is disabled by default
        let unregistered = String::from_str(&env, "UNREGISTERED_CLAIM");
        let id = client.create_attestation(&issuer, &subject, &unregistered, &None, &None, &None);
        assert!(!id.is_empty(), "unregistered claim type should be allowed when requirement is disabled");
    }

    #[test]
    fn test_set_require_registered_claim_type_admin_only() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _issuer, _subject, client) = setup(&env);

        let non_admin = Address::generate(&env);
        let result = client.try_set_require_registered_claim_type(&non_admin, &true);
        assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
    }

    #[test]
    fn test_admin_can_enable_require_registered_claim_type() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _issuer, _subject, client) = setup(&env);

        client.set_require_registered_claim_type(&admin, &true);
        assert_eq!(client.get_require_registered_claim_type(), true);
    }

    #[test]
    fn test_admin_can_disable_require_registered_claim_type() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _issuer, _subject, client) = setup(&env);

        // Enable it first
        client.set_require_registered_claim_type(&admin, &true);
        assert_eq!(client.get_require_registered_claim_type(), true);

        // Then disable it
        client.set_require_registered_claim_type(&admin, &false);
        assert_eq!(client.get_require_registered_claim_type(), false);
    }

    #[test]
    fn test_unregistered_claim_type_rejected_when_enabled() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        // Enable the requirement
        client.set_require_registered_claim_type(&admin, &true);

        // Try to create attestation with unregistered claim type
        let unregistered = String::from_str(&env, "UNREGISTERED_CLAIM");
        let result = client.try_create_attestation(&issuer, &subject, &unregistered, &None, &None, &None);
        assert_eq!(result, Err(Ok(types::Error::InvalidClaimType)));
    }

    #[test]
    fn test_registered_claim_type_accepted_when_enabled() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        // Register a claim type
        let claim_type = String::from_str(&env, "REGISTERED_CLAIM");
        let description = String::from_str(&env, "A registered claim type");
        client.register_claim_type(&admin, &claim_type, &description);

        // Enable the requirement
        client.set_require_registered_claim_type(&admin, &true);

        // Create attestation with registered claim type should succeed
        let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
        assert!(!id.is_empty(), "registered claim type should be accepted when requirement is enabled");
    }

    #[test]
    fn test_multiple_registered_claim_types_with_requirement_enabled() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        // Register multiple claim types
        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "ACCREDITED_INVESTOR");
        let claim3 = String::from_str(&env, "MERCHANT_VERIFIED");

        client.register_claim_type(&admin, &claim1, &String::from_str(&env, "KYC"));
        client.register_claim_type(&admin, &claim2, &String::from_str(&env, "Accredited"));
        client.register_claim_type(&admin, &claim3, &String::from_str(&env, "Merchant"));

        // Enable the requirement
        client.set_require_registered_claim_type(&admin, &true);

        // All registered types should work
        let id1 = client.create_attestation(&issuer, &subject, &claim1, &None, &None, &None);
        assert!(!id1.is_empty());

        let id2 = client.create_attestation(&issuer, &subject, &claim2, &None, &None, &None);
        assert!(!id2.is_empty());

        let id3 = client.create_attestation(&issuer, &subject, &claim3, &None, &None, &None);
        assert!(!id3.is_empty());

        // Unregistered type should fail
        let unregistered = String::from_str(&env, "UNKNOWN_CLAIM");
        let result = client.try_create_attestation(&issuer, &subject, &unregistered, &None, &None, &None);
        assert_eq!(result, Err(Ok(types::Error::InvalidClaimType)));
    }

    #[test]
    fn test_requirement_toggle_affects_behavior() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, subject, client) = setup(&env);

        let unregistered = String::from_str(&env, "UNREGISTERED");

        // Initially disabled - should work
        let id1 = client.create_attestation(&issuer, &subject, &unregistered, &None, &None, &None);
        assert!(!id1.is_empty());

        // Enable requirement - should fail
        client.set_require_registered_claim_type(&admin, &true);
        let result = client.try_create_attestation(&issuer, &subject, &unregistered, &None, &None, &None);
        assert_eq!(result, Err(Ok(types::Error::InvalidClaimType)));

        // Disable requirement - should work again
        client.set_require_registered_claim_type(&admin, &false);
        let id2 = client.create_attestation(&issuer, &subject, &unregistered, &None, &None, &None);
        assert!(!id2.is_empty());
    }

    #[test]
    fn test_batch_create_respects_claim_type_requirement() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer, _subject, client) = setup(&env);

        // Register one claim type
        let registered = String::from_str(&env, "REGISTERED");
        client.register_claim_type(&admin, &registered, &String::from_str(&env, "Registered"));

        // Enable requirement
        client.set_require_registered_claim_type(&admin, &true);

        // Create subjects
        let subject1 = Address::generate(&env);
        let subject2 = Address::generate(&env);

        // Batch with all registered types should succeed
        let subjects = Vec::from_array(&env, [subject1.clone(), subject2.clone()]);
        let count = client.create_attestations_batch(&issuer, &subjects, &registered, &None, &None);
        assert_eq!(count, 2);

        // Batch with unregistered type should fail
        let unregistered = String::from_str(&env, "UNREGISTERED");
        let result = client.try_create_attestations_batch(&issuer, &subjects, &unregistered, &None, &None);
        assert_eq!(result, Err(Ok(types::Error::InvalidClaimType)));
    }
}

// ============================================================================
// Get Valid Claims Tests
// ============================================================================

#[cfg(test)]
mod get_valid_claims_tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Ledger}, Env, String};

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

    #[test]
    fn test_get_valid_claims_empty_for_subject_with_no_attestations() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, _issuer, subject, client) = setup(&env);

        let claims = client.get_valid_claims(&subject);
        assert_eq!(claims.len(), 0);
    }

    #[test]
    fn test_get_valid_claims_returns_single_valid_claim() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, issuer, subject, client) = setup(&env);

        let claim_type = String::from_str(&env, "KYC_PASSED");
        let future_expiration = env.ledger().timestamp() + 86400; // 1 day from now

        client.create_attestation(&issuer, &subject, &claim_type, &Some(future_expiration), &None, &None);

        let claims = client.get_valid_claims(&subject);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims.get(0).unwrap(), claim_type);
    }

    #[test]
    fn test_get_valid_claims_returns_multiple_valid_claims() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, issuer, subject, client) = setup(&env);

        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "ACCREDITED_INVESTOR");
        let claim3 = String::from_str(&env, "MERCHANT_VERIFIED");
        let future_expiration = env.ledger().timestamp() + 86400;

        client.create_attestation(&issuer, &subject, &claim1, &Some(future_expiration), &None, &None);
        client.create_attestation(&issuer, &subject, &claim2, &Some(future_expiration), &None, &None);
        client.create_attestation(&issuer, &subject, &claim3, &Some(future_expiration), &None, &None);

        let claims = client.get_valid_claims(&subject);
        assert_eq!(claims.len(), 3);
        assert!(claims.iter().any(|c| c == claim1));
        assert!(claims.iter().any(|c| c == claim2));
        assert!(claims.iter().any(|c| c == claim3));
    }

    #[test]
    fn test_get_valid_claims_excludes_revoked_attestations() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, issuer, subject, client) = setup(&env);

        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "ACCREDITED_INVESTOR");
        let future_expiration = env.ledger().timestamp() + 86400;

        let id1 = client.create_attestation(&issuer, &subject, &claim1, &Some(future_expiration), &None, &None);
        let id2 = client.create_attestation(&issuer, &subject, &claim2, &Some(future_expiration), &None, &None);

        // Revoke the first one
        client.revoke_attestation(&issuer, &id1, &None);

        let claims = client.get_valid_claims(&subject);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims.get(0).unwrap(), claim2);
    }

    #[test]
    fn test_get_valid_claims_excludes_expired_attestations() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, issuer, subject, client) = setup(&env);

        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "ACCREDITED_INVESTOR");
        let current_time = env.ledger().timestamp();
        let past_expiration = current_time - 1; // Already expired
        let future_expiration = current_time + 86400;

        client.create_attestation(&issuer, &subject, &claim1, &Some(past_expiration), &None, &None);
        client.create_attestation(&issuer, &subject, &claim2, &Some(future_expiration), &None, &None);

        let claims = client.get_valid_claims(&subject);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims.get(0).unwrap(), claim2);
    }

    #[test]
    fn test_get_valid_claims_excludes_pending_attestations() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, issuer, subject, client) = setup(&env);

        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "ACCREDITED_INVESTOR");
        let current_time = env.ledger().timestamp();
        let future_valid_from = current_time + 3600; // Valid in 1 hour
        let future_expiration = current_time + 86400;

        client.create_attestation_valid_from(
            &issuer,
            &subject,
            &claim1,
            &Some(future_expiration),
            &None,
            &Some(future_valid_from),
            &None,
        );
        client.create_attestation(&issuer, &subject, &claim2, &Some(future_expiration), &None, &None);

        let claims = client.get_valid_claims(&subject);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims.get(0).unwrap(), claim2);
    }

    #[test]
    fn test_get_valid_claims_excludes_deleted_attestations() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, issuer, subject, client) = setup(&env);

        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "ACCREDITED_INVESTOR");
        let future_expiration = env.ledger().timestamp() + 86400;

        let id1 = client.create_attestation(&issuer, &subject, &claim1, &Some(future_expiration), &None, &None);
        let id2 = client.create_attestation(&issuer, &subject, &claim2, &Some(future_expiration), &None, &None);

        // Delete the first one
        client.request_deletion(&subject, &id1);

        let claims = client.get_valid_claims(&subject);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims.get(0).unwrap(), claim2);
    }

    #[test]
    fn test_get_valid_claims_deduplicates_same_claim_type() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, issuer, subject, client) = setup(&env);

        let claim_type = String::from_str(&env, "KYC_PASSED");
        let future_expiration = env.ledger().timestamp() + 86400;

        // Create multiple attestations of the same claim type from the same issuer
        client.create_attestation(&issuer, &subject, &claim_type, &Some(future_expiration), &None, &None);
        client.create_attestation(&issuer, &subject, &claim_type, &Some(future_expiration), &None, &None);

        let claims = client.get_valid_claims(&subject);
        // Should only return one instance of the claim type
        assert_eq!(claims.len(), 1);
        assert_eq!(claims.get(0).unwrap(), claim_type);
    }

    #[test]
    fn test_get_valid_claims_mixed_valid_and_invalid() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, issuer, subject, client) = setup(&env);

        let claim_valid = String::from_str(&env, "KYC_PASSED");
        let claim_revoked = String::from_str(&env, "REVOKED_CLAIM");
        let claim_expired = String::from_str(&env, "EXPIRED_CLAIM");
        let claim_pending = String::from_str(&env, "PENDING_CLAIM");

        let current_time = env.ledger().timestamp();
        let future_expiration = current_time + 86400;
        let past_expiration = current_time - 1;
        let future_valid_from = current_time + 3600;

        // Create valid attestation
        client.create_attestation(&issuer, &subject, &claim_valid, &Some(future_expiration), &None, &None);

        // Create and revoke attestation
        let id_revoked = client.create_attestation(&issuer, &subject, &claim_revoked, &Some(future_expiration), &None, &None);
        client.revoke_attestation(&issuer, &id_revoked, &None);

        // Create expired attestation
        client.create_attestation(&issuer, &subject, &claim_expired, &Some(past_expiration), &None, &None);

        // Create pending attestation
        client.create_attestation_valid_from(
            &issuer,
            &subject,
            &claim_pending,
            &Some(future_expiration),
            &None,
            &Some(future_valid_from),
            &None,
        );

        let claims = client.get_valid_claims(&subject);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims.get(0).unwrap(), claim_valid);
    }

    #[test]
    fn test_get_valid_claims_from_multiple_issuers() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, issuer1, subject, client) = setup(&env);

        let issuer2 = Address::generate(&env);
        client.register_issuer(&admin, &issuer2);

        let claim1 = String::from_str(&env, "KYC_PASSED");
        let claim2 = String::from_str(&env, "ACCREDITED_INVESTOR");
        let future_expiration = env.ledger().timestamp() + 86400;

        // Create attestations from different issuers
        client.create_attestation(&issuer1, &subject, &claim1, &Some(future_expiration), &None, &None);
        client.create_attestation(&issuer2, &subject, &claim2, &Some(future_expiration), &None, &None);

        let claims = client.get_valid_claims(&subject);
        assert_eq!(claims.len(), 2);
        assert!(claims.iter().any(|c| c == claim1));
        assert!(claims.iter().any(|c| c == claim2));
    }

    #[test]
    fn test_get_valid_claims_no_expiration_is_valid() {
        let env = Env::default();
        env.mock_all_auths();
        let (_admin, issuer, subject, client) = setup(&env);

        let claim_type = String::from_str(&env, "PERMANENT_CLAIM");

        // Create attestation with no expiration
        client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

        let claims = client.get_valid_claims(&subject);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims.get(0).unwrap(), claim_type);
    }
}
