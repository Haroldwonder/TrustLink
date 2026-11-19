//! Single-use, non-transferable event tickets built with TrustLink attestations.
//!
//! Run with: `cargo test --example event_ticketing`

use soroban_sdk::{testutils::Address as _, Address, Env, String};
use trustlink::{types::Error, TrustLinkContract, TrustLinkContractClient};

const TICKET_VALID: &str = "TICKET_VALID";

/// Venue check-in validates the ticket against the caller's address, then the
/// authorized ticket issuer revokes it in the same flow. The subject address is
/// part of the attestation and cannot be changed, so the ticket cannot be moved
/// to a different wallet after purchase.
fn check_in(
    client: &TrustLinkContractClient,
    issuer: &Address,
    attendee: &Address,
    ticket_id: &String,
) -> Result<(), Error> {
    let claim_type = String::from_str(&client.env, TICKET_VALID);
    if !client.has_valid_claim(attendee, &claim_type) {
        return Err(Error::NotFound);
    }

    client.revoke_attestation(issuer, ticket_id);
    Ok(())
}

#[test]
fn event_ticket_is_bound_to_its_holder_and_single_use() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let ticket_issuer = Address::generate(&env);
    let ticket_holder = Address::generate(&env);
    let attempted_recipient = Address::generate(&env);
    let claim_type = String::from_str(&env, TICKET_VALID);

    client.initialize(&admin);
    client.register_issuer(&admin, &ticket_issuer);

    // Ticket purchase: issue an attestation to the purchaser's wallet.
    let ticket_id = client.create_attestation(
        &ticket_issuer,
        &ticket_holder,
        &claim_type,
        &None,
        &Some(String::from_str(&env, "concert-2026-general-admission")),
    );

    assert!(client.has_valid_claim(&ticket_holder, &claim_type));
    // There is no transfer operation; a different wallet cannot present this ticket.
    assert!(!client.has_valid_claim(&attempted_recipient, &claim_type));

    // Venue check-in consumes the ticket by revoking its attestation.
    assert_eq!(check_in(&client, &ticket_issuer, &ticket_holder, &ticket_id), Ok(()));
    assert!(!client.has_valid_claim(&ticket_holder, &claim_type));
    assert!(client.get_attestation(&ticket_id).revoked);

    // A used ticket cannot be checked in twice.
    assert_eq!(
        check_in(&client, &ticket_issuer, &ticket_holder, &ticket_id),
        Err(Error::NotFound)
    );
}

fn main() {}
