#![no_std]

use soroban_sdk::{contract, contractclient, contractimpl, Address, Env, String};

#[contractclient(name = "TrustLinkClient")]
pub trait TrustLink {
    fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool;
}

#[contract]
pub struct EventTicketingContract;

/// Event ticketing example: Require valid event organizer attestation before issuing tickets
#[contractimpl]
impl EventTicketingContract {
    /// Issue a ticket to an attendee, requiring event organizer verification via TrustLink
    pub fn issue_ticket(
        env: Env,
        attendee: Address,
        trustlink_id: Address,
        event_id: String,
    ) -> Result<bool, String> {
        attendee.require_auth();

        // Verify attendee has a valid "EVENT_ORGANIZER" attestation from a trusted issuer
        // This ensures only verified event organizers can issue tickets
        let trustlink = TrustLinkClient::new(&env, &trustlink_id);
        let claim = String::from_str(&env, "EVENT_ORGANIZER");

        if !trustlink.has_valid_claim(&attendee, &claim) {
            return Err(String::from_str(&env, "attendee does not have EVENT_ORGANIZER attestation"));
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_ticket_issuance() {
        let env = Env::default();
        env.mock_all_auths();

        let event_ticketing_id = env.register_contract(None, EventTicketingContract);
        let event_ticketing = EventTicketingContractClient::new(&env, &event_ticketing_id);

        let organizer = Address::generate(&env);
        let event_id = String::from_str(&env, "summer-festival-2024");

        // Test: ticket issuance fails when organizer lacks attestation
        assert!(event_ticketing
            .try_issue_ticket(&organizer, &organizer, &event_id)
            .is_err());
    }
}
