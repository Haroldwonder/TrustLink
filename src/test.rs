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
    client.register_issuer(&admin, &issuer, &None);
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

    client.revoke_attestation(&issuer, &id);
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
    client.register_issuer(&admin, &issuer1, &None);
    client.register_issuer(&admin, &issuer2, &None);
    client.register_issuer(&admin, &issuer3, &None);
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
    client.register_issuer(&admin, &outsider, &None);

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

// ── Attestation Template tests ───────────────────────────────────────────────

#[test]
fn test_create_template_happy_path() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let template_id = String::from_str(&env, "KYC_TEMPLATE");
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: Some(30),
        metadata_template: Some(String::from_str(&env, "source=acme")),
    };

    client.create_template(&issuer, &template_id, &template);

    let retrieved = client.get_template(&issuer, &template_id);
    assert_eq!(retrieved, template);
}

#[test]
fn test_create_template_overwrite() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let template_id = String::from_str(&env, "MY_TEMPLATE");

    let first = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: Some(10),
        metadata_template: None,
    };
    let second = types::AttestationTemplate {
        claim_type: String::from_str(&env, "AML"),
        default_expiration_days: Some(60),
        metadata_template: Some(String::from_str(&env, "v2")),
    };

    client.create_template(&issuer, &template_id, &first);
    client.create_template(&issuer, &template_id, &second);

    let retrieved = client.get_template(&issuer, &template_id);
    assert_eq!(retrieved, second);
}

#[test]
fn test_create_template_empty_claim_type() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let template_id = String::from_str(&env, "BAD_TEMPLATE");
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, ""),
        default_expiration_days: None,
        metadata_template: None,
    };

    let result = client.try_create_template(&issuer, &template_id, &template);
    assert_eq!(result, Err(Ok(types::Error::InvalidClaimType)));
}

#[test]
fn test_create_template_metadata_too_long() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let template_id = String::from_str(&env, "LONG_META");
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: Some(String::from_bytes(&env, &[b'a'; 257])),
    };

    let result = client.try_create_template(&issuer, &template_id, &template);
    assert_eq!(result, Err(Ok(types::Error::MetadataTooLong)));
}

#[test]
fn test_create_template_non_issuer_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, client) = setup(&env);
    let non_issuer = Address::generate(&env);
    let template_id = String::from_str(&env, "SOME_TEMPLATE");
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };

    let result = client.try_create_template(&non_issuer, &template_id, &template);
    assert_eq!(result, Err(Ok(types::Error::Unauthorized)));
}

#[test]
fn test_list_templates_insertion_order_no_duplicates() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);

    let id_a = String::from_str(&env, "ALPHA");
    let id_b = String::from_str(&env, "BETA");
    let id_c = String::from_str(&env, "GAMMA");

    let tmpl = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };

    client.create_template(&issuer, &id_a, &tmpl);
    client.create_template(&issuer, &id_b, &tmpl);
    client.create_template(&issuer, &id_c, &tmpl);

    // Overwrite BETA — should not duplicate it in the registry.
    let tmpl2 = types::AttestationTemplate {
        claim_type: String::from_str(&env, "AML"),
        default_expiration_days: Some(5),
        metadata_template: None,
    };
    client.create_template(&issuer, &id_b, &tmpl2);

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
    let unknown_id = String::from_str(&env, "DOES_NOT_EXIST");

    let result = client.try_get_template(&issuer, &unknown_id);
    assert_eq!(result, Err(Ok(types::Error::NotFound)));
}

#[test]
fn test_create_attestation_from_template_happy_path() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);

    let timestamp = 1_000_000u64;
    env.ledger().with_mut(|li| li.timestamp = timestamp);

    let template_id = String::from_str(&env, "KYC_TMPL");
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: Some(1),
        metadata_template: Some(String::from_str(&env, "meta")),
    };
    client.create_template(&issuer, &template_id, &template);

    let attestation_id = client.create_attestation_from_template(
        &issuer,
        &template_id,
        &subject,
        &None,
        &None,
    );

    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.claim_type, String::from_str(&env, "KYC"));
    assert_eq!(attestation.metadata, Some(String::from_str(&env, "meta")));
    assert_eq!(attestation.expiration, Some(timestamp + 86_400));
}

#[test]
fn test_create_attestation_from_template_with_overrides() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);

    let timestamp = 1_000_000u64;
    env.ledger().with_mut(|li| li.timestamp = timestamp);

    let template_id = String::from_str(&env, "OVERRIDE_TMPL");
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: Some(30),
        metadata_template: Some(String::from_str(&env, "default_meta")),
    };
    client.create_template(&issuer, &template_id, &template);

    let override_expiration = timestamp + 999_999;
    let override_metadata = String::from_str(&env, "override_meta");

    let attestation_id = client.create_attestation_from_template(
        &issuer,
        &template_id,
        &subject,
        &Some(override_expiration),
        &Some(override_metadata.clone()),
    );

    let attestation = client.get_attestation(&attestation_id);
    assert_eq!(attestation.expiration, Some(override_expiration));
    assert_eq!(attestation.metadata, Some(override_metadata));
}

#[test]
fn test_create_attestation_from_template_unknown_template() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);
    let unknown_id = String::from_str(&env, "GHOST_TEMPLATE");

    let result = client.try_create_attestation_from_template(
        &issuer,
        &unknown_id,
        &subject,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(types::Error::NotFound)));
}

#[test]
fn test_create_attestation_from_template_stale_expiration() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);

    let timestamp = 1_000_000u64;
    env.ledger().with_mut(|li| li.timestamp = timestamp);

    let template_id = String::from_str(&env, "STALE_TMPL");
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };
    client.create_template(&issuer, &template_id, &template);

    let result = client.try_create_attestation_from_template(
        &issuer,
        &template_id,
        &subject,
        &Some(999_999),
        &None,
    );
    assert_eq!(result, Err(Ok(types::Error::InvalidExpiration)));
}

#[test]
fn test_create_attestation_from_template_metadata_override_too_long() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let subject = Address::generate(&env);

    let template_id = String::from_str(&env, "META_TMPL");
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };
    client.create_template(&issuer, &template_id, &template);

    let result = client.try_create_attestation_from_template(
        &issuer,
        &template_id,
        &subject,
        &None,
        &Some(String::from_bytes(&env, &[b'a'; 257])),
    );
    assert_eq!(result, Err(Ok(types::Error::MetadataTooLong)));
}

#[test]
fn test_create_attestation_from_template_two_issuers_isolated() {
    let env = Env::default();
    env.mock_all_auths();

    let (admin, issuer_a, client) = setup(&env);
    let issuer_b = Address::generate(&env);
    client.register_issuer(&admin, &issuer_b);

    let template_id = String::from_str(&env, "SHARED_ID");

    let tmpl_a = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: Some(10),
        metadata_template: Some(String::from_str(&env, "issuer_a_meta")),
    };
    let tmpl_b = types::AttestationTemplate {
        claim_type: String::from_str(&env, "AML"),
        default_expiration_days: Some(20),
        metadata_template: Some(String::from_str(&env, "issuer_b_meta")),
    };

    client.create_template(&issuer_a, &template_id, &tmpl_a);
    client.create_template(&issuer_b, &template_id, &tmpl_b);

    let retrieved_a = client.get_template(&issuer_a, &template_id);
    let retrieved_b = client.get_template(&issuer_b, &template_id);

    assert_eq!(retrieved_a, tmpl_a);
    assert_eq!(retrieved_b, tmpl_b);
    assert_ne!(retrieved_a, retrieved_b);
}

#[test]
fn test_create_template_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, issuer, client) = setup(&env);
    let template_id = String::from_str(&env, "EVENT_TMPL");
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };

    client.create_template(&issuer, &template_id, &template);

    let events = env.events().all();
    let mut found = false;
    for (_, topics, _) in events.iter() {
        let topic0: soroban_sdk::Symbol =
            soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        if topic0 == soroban_sdk::symbol_short!("tmpl_crt") {
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
    let subject = Address::generate(&env);

    env.ledger().with_mut(|li| li.timestamp = 1_000_000);

    let template_id = String::from_str(&env, "EVT_TMPL");
    let template = types::AttestationTemplate {
        claim_type: String::from_str(&env, "KYC"),
        default_expiration_days: None,
        metadata_template: None,
    };
    client.create_template(&issuer, &template_id, &template);

    client.create_attestation_from_template(&issuer, &template_id, &subject, &None, &None);

    let events = env.events().all();
    let mut found = false;
    for (_, topics, _) in events.iter() {
        let topic0: soroban_sdk::Symbol =
            soroban_sdk::TryFromVal::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
        if topic0 == soroban_sdk::symbol_short!("created") {
            found = true;
            break;
        }
    }
    assert!(found, "attestation_created event not found");
}
