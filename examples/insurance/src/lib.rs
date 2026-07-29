#![no_std]

use soroban_sdk::{contract, contractclient, contractimpl, contracttype, Address, Env, String, Vec};

#[contracttype]
pub enum DataKey {
    Admin,
    TrustLink,
    PolicyCount,
    Policy(u32),
}

#[contractclient(name = "TrustLinkClient")]
pub trait TrustLink {
    fn has_all_claims(env: Env, subject: Address, claim_types: Vec<String>) -> bool;
}

#[contract]
pub struct InsuranceContract;

#[contractimpl]
impl InsuranceContract {
    pub fn initialize(env: Env, admin: Address, trustlink_contract: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TrustLink, &trustlink_contract);
        env.storage().instance().set(&DataKey::PolicyCount, &0u32);
    }

    pub fn issue_policy(env: Env, insurer: Address, policyholder: Address) -> u32 {
        insurer.require_auth();

        let trustlink_contract: Address = env.storage().instance().get(&DataKey::TrustLink).unwrap();
        let trustlink = TrustLinkClient::new(&env, &trustlink_contract);

        let mut required_claims: Vec<String> = Vec::new(&env);
        required_claims.push_back(String::from_str(&env, "KYC_PASSED"));
        required_claims.push_back(String::from_str(&env, "AML_CLEARED"));

        if !trustlink.has_all_claims(&policyholder, &required_claims) {
            panic!("policyholder must have valid KYC_PASSED and AML_CLEARED claims");
        }

        let policy_count: u32 = env.storage().instance().get(&DataKey::PolicyCount).unwrap_or(0);
        let next_policy_number = policy_count + 1;
        env.storage().instance().set(&DataKey::PolicyCount, &next_policy_number);
        env.storage().instance().set(&DataKey::Policy(next_policy_number), &policyholder);

        next_policy_number
    }

    pub fn get_policy_holder(env: Env, policy_id: u32) -> Address {
        env.storage().instance().get(&DataKey::Policy(policy_id)).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{contract, contractimpl, testutils::Address as _, xdr::ToXdr, Address, Env, Vec};

    #[contract]
    struct MockTrustLink;

    #[contractimpl]
    impl MockTrustLink {
        pub fn has_all_claims(_env: Env, subject: Address, claim_types: Vec<String>) -> bool {
            if claim_types.len() != 2 {
                return false;
            }

            let expected_kyc = String::from_str(&_env, "KYC_PASSED");
            let expected_aml = String::from_str(&_env, "AML_CLEARED");
            if claim_types.get(0).unwrap() != expected_kyc || claim_types.get(1).unwrap() != expected_aml {
                return false;
            }

            let bytes = subject.to_xdr(&_env);
            let sum: u32 = bytes.iter().map(|b| b as u32).sum();
            sum % 2 == 0
        }
    }

    fn required_claims(env: &Env) -> Vec<String> {
        let mut claims = Vec::new(env);
        claims.push_back(String::from_str(env, "KYC_PASSED"));
        claims.push_back(String::from_str(env, "AML_CLEARED"));
        claims
    }

    #[test]
    fn issue_policy_allowed_for_policyholders_with_all_claims() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let insurer = Address::generate(&env);
        let mut policyholder = Address::generate(&env);

        let trustlink_id = env.register_contract(None, MockTrustLink);
        let contract_id = env.register_contract(None, InsuranceContract);
        let client = InsuranceContractClient::new(&env, &contract_id);
        let trustlink_client = MockTrustLinkClient::new(&env, &trustlink_id);

        client.initialize(&admin, &trustlink_id);

        for _ in 0..200 {
            if trustlink_client.has_all_claims(&policyholder, &required_claims(&env)) {
                break;
            }
            policyholder = Address::generate(&env);
        }

        let policy_id = client.issue_policy(&insurer, &policyholder);
        assert_eq!(client.get_policy_holder(&policy_id), policyholder);
    }

    #[test]
    fn issue_policy_rejected_when_policyholder_missing_a_claim() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let insurer = Address::generate(&env);
        let mut policyholder = Address::generate(&env);

        let trustlink_id = env.register_contract(None, MockTrustLink);
        let contract_id = env.register_contract(None, InsuranceContract);
        let client = InsuranceContractClient::new(&env, &contract_id);
        let trustlink_client = MockTrustLinkClient::new(&env, &trustlink_id);

        client.initialize(&admin, &trustlink_id);

        for _ in 0..200 {
            if !trustlink_client.has_all_claims(&policyholder, &required_claims(&env)) {
                break;
            }
            policyholder = Address::generate(&env);
        }

        let result = client.try_issue_policy(&insurer, &policyholder);
        assert!(result.is_err());
    }
}
