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
    client.initialize(&admin);
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

    client.initialize(&admin);
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

    client.initialize(&admin);
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

    client.initialize(&admin);

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

    client.initialize(&admin);
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
    client.initialize(&admin);

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
