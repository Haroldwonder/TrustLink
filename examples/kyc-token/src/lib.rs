#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, token, Address, Env,
    String,
};

#[contracttype]
pub enum DataKey {
    Admin,
    TrustLink,
}

#[contractclient(name = "TrustLinkClient")]
pub trait TrustLink {
    fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool;
}

#[contract]
pub struct KycTokenContract;

#[contractimpl]
impl KycTokenContract {
    pub fn initialize(env: Env, admin: Address, trustlink_contract: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TrustLink, &trustlink_contract);
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        token::StellarAssetClient::new(&env, &env.current_contract_address()).mint(&to, &amount);
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        let trustlink_id: Address = env.storage().instance().get(&DataKey::TrustLink).unwrap();
        let trustlink = TrustLinkClient::new(&env, &trustlink_id);

        let claim = String::from_str(&env, "KYC_PASSED");
        let from_kyc = trustlink.has_valid_claim(&from, &claim);
        let to_kyc = trustlink.has_valid_claim(&to, &claim);

        if !from_kyc || !to_kyc {
            panic!("kyc required for sender and receiver");
        }

        token::StellarAssetClient::new(&env, &env.current_contract_address()).transfer(
            &from,
            &to,
            &amount,
        );

        env.events().publish(
            (symbol_short!("kyc_xfer"), from, to),
            amount,
        );
    }

    pub fn set_trustlink(env: Env, admin: Address, trustlink_contract: Address) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if current_admin != admin {
            panic!("admin only");
        }
        env.storage().instance().set(&DataKey::TrustLink, &trustlink_contract);
    }

    pub fn get_trustlink(env: Env) -> Address {
        env.storage().instance().get(&DataKey::TrustLink).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{contract, contractimpl, testutils::Address as _, token, Address, Env};

    #[contract]
    struct MockTrustLink;

    #[contractimpl]
    impl MockTrustLink {
        pub fn has_valid_claim(_env: Env, subject: Address, _claim_type: String) -> bool {
            // Deterministic mock: odd byte-sum = KYC true, even = false
            let bytes = subject.to_xdr(&_env);
            let mut sum: u32 = 0;
            for b in bytes.iter() {
                sum += b as u32;
            }
            sum % 2 == 1
        }
    }

    #[test]
    fn transfer_blocked_for_non_kyc_address() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);

        let trustlink_id = env.register(MockTrustLink, ());
        let token_id = env.register(KycTokenContract, ());
        let token_client = KycTokenContractClient::new(&env, &token_id);

        token_client.initialize(&admin, &trustlink_id);
        token_client.mint(&user1, &500);

        let claim = String::from_str(&env, "KYC_PASSED");
        let trustlink = MockTrustLinkClient::new(&env, &trustlink_id);
        let user1_kyc = trustlink.has_valid_claim(&user1, &claim);
        let user2_kyc = trustlink.has_valid_claim(&user2, &claim);

        if user1_kyc && user2_kyc {
            let outsider = Address::generate(&env);
            token_client
                .try_transfer(&user1, &outsider, &100)
                .unwrap_err();
        } else {
            token_client
                .try_transfer(&user1, &user2, &100)
                .unwrap_err();
        }
    }

    #[test]
    fn transfer_allowed_for_kyc_addresses() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);

        let trustlink_id = env.register(MockTrustLink, ());
        let token_id = env.register(KycTokenContract, ());
        let token_client = KycTokenContractClient::new(&env, &token_id);
        let trustlink = MockTrustLinkClient::new(&env, &trustlink_id);

        token_client.initialize(&admin, &trustlink_id);

        // Find two addresses considered KYC=true by the mock.
        let mut from = Address::generate(&env);
        let mut to = Address::generate(&env);
        let claim = String::from_str(&env, "KYC_PASSED");
        for _ in 0..200 {
            if trustlink.has_valid_claim(&from, &claim) && trustlink.has_valid_claim(&to, &claim) {
                break;
            }
            from = Address::generate(&env);
            to = Address::generate(&env);
        }

        token_client.mint(&from, &1000);
        token_client.transfer(&from, &to, &250);

        let stellar = token::StellarAssetClient::new(&env, &token_id);
        assert_eq!(stellar.balance(&from), 750);
        assert_eq!(stellar.balance(&to), 250);
    }
}
