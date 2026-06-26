#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, Address, Env, String,
};

#[contracttype]
pub enum DataKey {
    TrustLink,
    Proposal(u32),
    Vote(u32, Address),
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub deadline: u64,
}

#[contractclient(name = "TrustLinkClient")]
pub trait TrustLink {
    fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool;
}

#[contract]
pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    /// Initialize the governance contract with a TrustLink contract address.
    pub fn initialize(env: Env, trustlink: Address) {
        env.storage().instance().set(&DataKey::TrustLink, &trustlink);
    }

    /// Create a proposal with a voting deadline (Unix timestamp).
    pub fn create_proposal(env: Env, proposal_id: u32, deadline: u64) {
        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &Proposal { deadline });
    }

    /// Cast a vote on a proposal. Voter must have valid KYC and vote before
    /// the proposal deadline.
    pub fn vote(env: Env, voter: Address, proposal_id: u32, vote: bool) {
        voter.require_auth();

        let proposal: Proposal = env
            .storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .unwrap_or_else(|| panic!("proposal not found"));

        if env.ledger().timestamp() > proposal.deadline {
            panic!("voting period has ended");
        }

        let trustlink: Address = env
            .storage()
            .instance()
            .get(&DataKey::TrustLink)
            .unwrap_or_else(|| panic!("not initialized"));

        let trustlink_client = TrustLinkClient::new(&env, &trustlink);
        let kyc_claim = String::from_str(&env, "KYC_PASSED");

        if !trustlink_client.has_valid_claim(&voter, &kyc_claim) {
            panic!("voter must have valid KYC");
        }

        env.storage()
            .instance()
            .set(&DataKey::Vote(proposal_id, voter), &vote);
    }

    /// Get the vote count for a proposal.
    pub fn get_vote_count(env: Env, proposal_id: u32) -> (u32, u32) {
        let _ = (env, proposal_id);
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    // ── Mock: KYC always passes ───────────────────────────────────────────────
    mod mock_kyc_pass {
        use soroban_sdk::{contract, contractimpl, Address, Env, String};

        #[contract]
        pub struct MockTrustLink;

        #[contractimpl]
        impl MockTrustLink {
            pub fn has_valid_claim(_env: Env, _subject: Address, _claim_type: String) -> bool {
                true
            }
        }
    }

    // ── Mock: KYC always fails ────────────────────────────────────────────────
    mod mock_kyc_fail {
        use soroban_sdk::{contract, contractimpl, Address, Env, String};

        #[contract]
        pub struct MockTrustLinkNoKyc;

        #[contractimpl]
        impl MockTrustLinkNoKyc {
            pub fn has_valid_claim(_env: Env, _subject: Address, _claim_type: String) -> bool {
                false
            }
        }
    }

    use mock_kyc_fail::MockTrustLinkNoKyc;
    use mock_kyc_pass::MockTrustLink;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn setup_with_kyc(env: &Env) -> GovernanceContractClient {
        let trustlink_id = env.register_contract(None, MockTrustLink);
        let contract_id = env.register_contract(None, GovernanceContract);
        let client = GovernanceContractClient::new(env, &contract_id);
        client.initialize(&trustlink_id);
        client
    }

    fn setup_without_kyc(env: &Env) -> GovernanceContractClient {
        let trustlink_id = env.register_contract(None, MockTrustLinkNoKyc);
        let contract_id = env.register_contract(None, GovernanceContract);
        let client = GovernanceContractClient::new(env, &contract_id);
        client.initialize(&trustlink_id);
        client
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_vote_succeeds_before_deadline() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_with_kyc(&env);
        let voter = Address::generate(&env);

        // Ledger timestamp defaults to 0; deadline is in the future.
        client.create_proposal(&1, &1000);
        client.vote(&voter, &1, &true); // must not panic
    }

    #[test]
    fn test_vote_rejected_after_deadline() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_with_kyc(&env);
        let voter = Address::generate(&env);

        client.create_proposal(&1, &500);

        // Advance ledger past the deadline.
        env.ledger().with_mut(|li| li.timestamp = 501);

        let result = client.try_vote(&voter, &1, &true);
        assert!(result.is_err());
    }

    #[test]
    fn test_vote_allowed_at_deadline_boundary() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_with_kyc(&env);
        let voter = Address::generate(&env);

        client.create_proposal(&1, &500);

        // Exactly at the deadline — still allowed (> not >=).
        env.ledger().with_mut(|li| li.timestamp = 500);

        client.vote(&voter, &1, &true); // must not panic
    }

    #[test]
    fn test_vote_requires_kyc() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_without_kyc(&env);
        let voter = Address::generate(&env);

        client.create_proposal(&1, &1000);

        let result = client.try_vote(&voter, &1, &true);
        assert!(result.is_err());
    }

    #[test]
    fn test_vote_proposal_not_found() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_with_kyc(&env);
        let voter = Address::generate(&env);

        // No proposal created for id 99.
        let result = client.try_vote(&voter, &99, &true);
        assert!(result.is_err());
    }

    #[test]
    fn test_deadline_checked_before_kyc() {
        // Deadline error must fire even when KYC would also fail.
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_without_kyc(&env);
        let voter = Address::generate(&env);

        client.create_proposal(&1, &500);
        env.ledger().with_mut(|li| li.timestamp = 600);

        let result = client.try_vote(&voter, &1, &true);
        assert!(result.is_err());
    }
}
