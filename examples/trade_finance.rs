//! Multi-party bill-of-lading verification for cross-border trade finance.
//!
//! Run with: `cargo test --example trade_finance`

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use trustlink::{TrustLinkContract, TrustLinkContractClient};

const SHIPMENT_CONFIRMED: &str = "SHIPMENT_CONFIRMED";
const CUSTOMS_CLEARED: &str = "CUSTOMS_CLEARED";
const FINANCING_CONDITIONS_MET: &str = "FINANCING_CONDITIONS_MET";

/// A shipment is financeable only while all independent bill-of-lading claims
/// for its shared subject address remain valid.
fn is_trade_finance_clear(client: &TrustLinkContractClient, shipment: &Address) -> bool {
    let env = &client.env;
    let mut required_claims = Vec::new(env);
    required_claims.push_back(String::from_str(env, SHIPMENT_CONFIRMED));
    required_claims.push_back(String::from_str(env, CUSTOMS_CLEARED));
    required_claims.push_back(String::from_str(env, FINANCING_CONDITIONS_MET));
    client.has_all_claims(shipment, &required_claims)
}

#[test]
fn bill_of_lading_requires_all_independent_parties() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let exporter = Address::generate(&env);
    let customs_authority = Address::generate(&env);
    let financing_bank = Address::generate(&env);
    // This address is the shared shipment/bill-of-lading reference.
    let shipment = Address::generate(&env);
    let bill_of_lading = Some(String::from_str(&env, "BOL-2026-0042"));

    client.initialize(&admin);
    client.register_issuer(&admin, &exporter);
    client.register_issuer(&admin, &customs_authority);
    client.register_issuer(&admin, &financing_bank);

    client.create_attestation(
        &exporter,
        &shipment,
        &String::from_str(&env, SHIPMENT_CONFIRMED),
        &None,
        &bill_of_lading,
    );
    assert!(!is_trade_finance_clear(&client, &shipment));

    let customs_attestation = client.create_attestation(
        &customs_authority,
        &shipment,
        &String::from_str(&env, CUSTOMS_CLEARED),
        &None,
        &bill_of_lading,
    );
    assert!(!is_trade_finance_clear(&client, &shipment));

    client.create_attestation(
        &financing_bank,
        &shipment,
        &String::from_str(&env, FINANCING_CONDITIONS_MET),
        &None,
        &bill_of_lading,
    );
    assert!(is_trade_finance_clear(&client, &shipment));

    client.revoke_attestation(&customs_authority, &customs_attestation);
    assert!(!is_trade_finance_clear(&client, &shipment));
}

fn main() {}
