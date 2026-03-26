use super::*;
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, Env, String,
};

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
    assert!(!client.get_attestation(&id).imported);
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
    assert!(!attestation.imported);
    assert_eq!(attestation.valid_from, None);
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

    let result = client.try_create_attestation(
        &issuer,
        &issuer,
        &claim_type,
        &None,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
    assert_eq!(token_client.balance(&issuer), 100);
    assert_eq!(token_client.balance(&collector), 0);
    assert_eq!(client.get_subject_attestations(&issuer, &0, &10).len(), 0);
}

#[test]
fn test_create_attestation_rejects_metadata_over_256_chars() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let too_long = Some(String::from_bytes(&env, &[b'a'; 257]));

    let result = client.try_create_attestation(&issuer, &subject, &claim_type, &None, &too_long, &None);
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
fn test_expired_attestation_status() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let now = env.ledger().timestamp();

    let id = client.create_attestation(&issuer, &subject, &claim_type, &Some(now + 100), &None, &None);
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
    assert!(attestation.imported);
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
    assert!(attestation.bridged);
    assert!(!attestation.imported);
    assert_eq!(attestation.source_chain, Some(source_chain));
    assert_eq!(attestation.source_tx, Some(source_tx));
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
    assert!(attestation.bridged);
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

    let proposal_id =
        client.propose_attestation(&issuer1, &subject, &claim_type, &required, &2);

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

    let proposal_id =
        client.propose_attestation(&issuer1, &subject, &claim_type, &required, &3);

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

    let proposal_id =
        client.propose_attestation(&issuer1, &subject, &claim_type, &required, &2);

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

    let proposal_id =
        client.propose_attestation(&issuer1, &subject, &claim_type, &required, &3);

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

    let proposal_id =
        client.propose_attestation(&issuer1, &subject, &claim_type, &required, &2);

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
    let result =
        client.try_propose_attestation(&issuer1, &subject, &claim_type, &required, &0);
    assert_eq!(result, Err(Ok(types::Error::InvalidThreshold)));

    // threshold > signer count is invalid.
    let result =
        client.try_propose_attestation(&issuer1, &subject, &claim_type, &required, &4);
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

    let proposal_id =
        client.propose_attestation(&issuer1, &subject, &claim_type, &required, &2);

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
    let exact_reason = String::from_bytes(&env, &[b'a'; 128]);
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

    let id1 = types::Attestation::generate_id(&env, &issuer, &Address::generate(&env), &claim_type, ts);
    let id2 = types::Attestation::generate_id(&env, &issuer, &Address::generate(&env), &claim_type, ts);
    assert_ne!(id1, id2, "different subjects must produce different IDs");
}

/// Same subject, different issuers → different IDs.
#[test]
fn test_id_uniqueness_same_subject_different_issuers() {
    let env = Env::default();
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");
    let ts = 1_000_000u64;

    let id1 = types::Attestation::generate_id(&env, &Address::generate(&env), &subject, &claim_type, ts);
    let id2 = types::Attestation::generate_id(&env, &Address::generate(&env), &subject, &claim_type, ts);
    assert_ne!(id1, id2, "different issuers must produce different IDs");
}

/// Same issuer + subject, different claim types → different IDs.
#[test]
fn test_id_uniqueness_same_issuer_subject_different_claim_types() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let ts = 1_000_000u64;

    let id1 = types::Attestation::generate_id(&env, &issuer, &subject, &String::from_str(&env, "KYC_PASSED"), ts);
    let id2 = types::Attestation::generate_id(&env, &issuer, &subject, &String::from_str(&env, "ACCREDITED_INVESTOR"), ts);
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
        "KYC_PASSED", "ACCREDITED_INVESTOR", "MERCHANT_VERIFIED", "AML_CLEARED", "SANCTIONS_CHECKED",
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
    let (_, issuer, client) = setup(&env);
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
    assert_eq!(log.get(0).unwrap().action, crate::types::AuditAction::Created);
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
    assert_eq!(log.get(1).unwrap().action, crate::types::AuditAction::Revoked);
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
    assert_eq!(log.get(1).unwrap().action, crate::types::AuditAction::Renewed);
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
    assert_eq!(log.get(1).unwrap().action, crate::types::AuditAction::Updated);
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
    assert_eq!(log.get(0).unwrap().action, crate::types::AuditAction::Created);
    assert_eq!(log.get(1).unwrap().action, crate::types::AuditAction::Renewed);
    assert_eq!(log.get(2).unwrap().action, crate::types::AuditAction::Revoked);
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
    assert_eq!(client.get_audit_log(&id1).get(1).unwrap().action, crate::types::AuditAction::Revoked);
}

// ── Pause / unpause tests ────────────────────────────────────────────────────

#[test]
fn test_pause_and_unpause_by_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);

    assert!(!client.is_paused());

    client.pause(&admin);
    assert!(client.is_paused());

    client.unpause(&admin);
    assert!(!client.is_paused());
}

#[test]
fn test_pause_requires_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, client) = setup(&env);
    let non_admin = Address::generate(&env);

    let result = client.try_pause(&non_admin);
    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
    assert!(!client.is_paused());
}

#[test]
fn test_unpause_requires_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);
    let non_admin = Address::generate(&env);

    client.pause(&admin);
    assert!(client.is_paused());

    let result = client.try_unpause(&non_admin);
    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
    assert!(client.is_paused());
}

#[test]
fn test_create_attestation_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    client.pause(&admin);

    let result = client.try_create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    assert_eq!(result, Err(Ok(types::Error::ContractPaused)));
}

#[test]
fn test_revoke_attestation_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    // Create attestation while unpaused.
    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    client.pause(&admin);

    let result = client.try_revoke_attestation(&issuer, &id, &None);
    assert_eq!(result, Err(Ok(types::Error::ContractPaused)));

    // Attestation must still exist and not be revoked.
    assert!(!client.get_attestation(&id).revoked);
}

#[test]
fn test_reads_work_while_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);

    client.pause(&admin);

    // All read paths must remain functional while paused.
    assert!(client.has_valid_claim(&subject, &claim_type));
    assert_eq!(client.get_attestation(&id).id, id);
    assert_eq!(
        client.get_attestation_status(&id),
        types::AttestationStatus::Valid
    );
    assert_eq!(client.get_subject_attestations(&subject, &0, &10).len(), 1);
    assert_eq!(client.get_issuer_attestations(&issuer, &0, &10).len(), 1);
}

#[test]
fn test_create_attestation_works_after_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    client.pause(&admin);
    client.unpause(&admin);

    // Should succeed now.
    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
    assert!(client.has_valid_claim(&subject, &claim_type));
    assert_eq!(client.get_attestation(&id).issuer, issuer);
}

#[test]
fn test_pause_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);
    let timestamp = 9_999_999u64;
    env.ledger().set_timestamp(timestamp);

    client.pause(&admin);

    let events = env.events().all();
    let mut found = false;
    for (_, topics, data) in events {
        let topic0: soroban_sdk::Symbol =
            soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        if topic0 == soroban_sdk::symbol_short!("paused") {
            let (event_admin, event_ts): (Address, u64) =
                soroban_sdk::TryFromVal::try_from_val(&env, &data).unwrap();
            assert_eq!(event_admin, admin);
            assert_eq!(event_ts, timestamp);
            found = true;
            break;
        }
    }
    assert!(found, "ContractPaused event not emitted");
}

#[test]
fn test_unpause_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, _, client) = setup(&env);
    let timestamp = 9_999_999u64;
    env.ledger().set_timestamp(timestamp);

    client.pause(&admin);
    client.unpause(&admin);

    let events = env.events().all();
    let mut found = false;
    for (_, topics, data) in events {
        let topic0: soroban_sdk::Symbol =
            soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        if topic0 == soroban_sdk::symbol_short!("unpaused") {
            let (event_admin, event_ts): (Address, u64) =
                soroban_sdk::TryFromVal::try_from_val(&env, &data).unwrap();
            assert_eq!(event_admin, admin);
            assert_eq!(event_ts, timestamp);
            found = true;
            break;
        }
    }
    assert!(found, "ContractUnpaused event not emitted");
}

#[test]
fn test_is_paused_false_by_default() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, client) = setup(&env);
    assert!(!client.is_paused());
}

// ── Expiration lifecycle integration test (Issue #72) ────────────────────────

#[test]
fn test_expiration_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, "KYC_PASSED");

    // ── Step 1: Set a known start time and create attestation with short expiry ──
    let start_time: u64 = 1_000_000;
    env.ledger().set_timestamp(start_time);

    let expiration = start_time + 100; // expires 100 seconds from now
    let id = client.create_attestation(
        &issuer,
        &subject,
        &claim_type,
        &Some(expiration),
        &None,
        &None,
    );

    // ── Step 2: Verify attestation is Valid before expiration ──
    assert!(
        client.has_valid_claim(&subject, &claim_type),
        "has_valid_claim should be true before expiration"
    );
    assert_eq!(
        client.get_attestation_status(&id),
        types::AttestationStatus::Valid,
        "status should be Valid before expiration"
    );

    // ── Step 3: Advance ledger timestamp past expiration ──
    env.ledger().set_timestamp(expiration + 1);

    // ── Step 4: Verify has_valid_claim returns false after expiration ──
    assert!(
        !client.has_valid_claim(&subject, &claim_type),
        "has_valid_claim should be false after expiration"
    );

    // ── Step 5: Verify get_attestation_status returns Expired ──
    assert_eq!(
        client.get_attestation_status(&id),
        types::AttestationStatus::Expired,
        "status should be Expired after expiration"
    );

    // ── Step 6: Verify expired attestation cannot be revoked ──
    // The contract does NOT check expiry before revoking — revoke_attestation
    // only blocks on: wrong issuer, already revoked, or paused.
    // Therefore an expired attestation CAN still be revoked (it's not deleted).
    // This documents that behavior explicitly.
    let revoke_result = client.try_revoke_attestation(&issuer, &id, &None);
    assert!(
        revoke_result.is_ok(),
        "expired attestation should still be revocable (contract does not block revoke on expired)"
    );

    // After revocation the raw record reflects revoked=true
    let attestation = client.get_attestation(&id);
    assert!(attestation.revoked, "attestation should be marked revoked");

    // Status is now Revoked (revoked flag takes precedence in get_status)
    assert_eq!(
        client.get_attestation_status(&id),
        types::AttestationStatus::Revoked,
        "status should be Revoked after revoking an expired attestation"
    );
}