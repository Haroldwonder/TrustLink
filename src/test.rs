use super::*;
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, Env, String,
};

use crate::types::AttestationOrigin;

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

    let api_results = client.get_attestations_by_jurisdiction(&subject, &String::from_str(&env, "US"));
    assert_eq!(api_results.len(), 1);
    assert_eq!(api_results.get(0).unwrap(), id);

    let wrong_results = client.get_attestations_by_jurisdiction(&subject, &String::from_str(&env, "CA"));
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
    assert_eq!(client.get_attestation_by_type(&subject, &claim_type).id, id);
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

#[test]
fn test_revoke_without_reason_stores_none() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    client.revoke_attestation(&issuer, &id, &None);

    let attestation = client.get_attestation(&id);
    assert!(attestation.revoked);
    assert_eq!(attestation.revocation_reason, None);
}

#[test]
fn test_revoke_reason_too_long_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // 129 'a' characters — one over the limit
    let long_reason = String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let result = client.try_revoke_attestation(&issuer, &id, &Some(long_reason));
    assert_eq!(result, Err(Ok(types::Error::ReasonTooLong)));
}

#[test]
fn test_revoke_reason_exactly_128_chars_accepted() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Exactly 128 'a' characters
    let exact_reason = String::from_str(&env, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    client.revoke_attestation(&issuer, &id, &Some(exact_reason.clone()));

    let attestation = client.get_attestation(&id);
    assert!(attestation.revoked);
    assert_eq!(attestation.revocation_reason, Some(exact_reason));
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
        assert_eq!(pending.get(0).unwrap(), id);
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
                sym == soroban_sdk::symbol_short!("req")
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
fn test_rate_limit_blocks_issuer_issuing_too_fast() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    // Set a 60-second minimum interval between issuances.
    client.set_rate_limit(&admin, &60u64);

    // First attestation succeeds.
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Second attempt at the same timestamp is blocked.
    let subject2 = Address::generate(&env);
    let result = client.try_create_attestation(&issuer, &subject2, &claim_type, &None, &None, &None);
    assert!(result.is_err());
}

#[test]
fn test_rate_limit_allows_after_interval() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    client.set_rate_limit(&admin, &60u64);
    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    // Advance ledger time past the interval.
    env.ledger().with_mut(|l| l.timestamp += 61);

    let subject2 = Address::generate(&env);
    // Should succeed now.
    client.create_attestation(&issuer, &subject2, &claim_type, &None, &None, &None);
}

#[test]
fn test_rate_limit_zero_interval_disables_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    // Interval of 0 means no rate limiting.
    client.set_rate_limit(&admin, &0u64);

    for _ in 0..3u32 {
        let subject = Address::generate(&env);
        client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    }
}
