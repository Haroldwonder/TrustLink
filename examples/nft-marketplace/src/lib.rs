//! NFT Marketplace Example (TrustLink Integration)
//!
//! Demonstrates a compliance-gated NFT marketplace where listing and bidding
//! both require:
//!
//! 1. A valid `KYC_PASSED` attestation (`has_valid_claim`)
//! 2. At least one attestation tagged for the marketplace's allowed jurisdiction
//!    (`get_attestations_by_jurisdiction`)
//!
//! This mirrors regulated NFT marketplace requirements where wallets must pass
//! KYC and reside in an eligible jurisdiction before participating.

#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, Address, Env, String, Vec,
};

/// Claim type required for marketplace participation.
pub const CLAIM_KYC_PASSED: &str = "KYC_PASSED";

#[contracttype]
pub enum DataKey {
    Admin,
    TrustLink,
    AllowedJurisdiction,
    ListingCount,
    Listing(u32),
    HighestBid(u32),
}

#[contracttype]
#[derive(Clone)]
pub struct Listing {
    pub seller: Address,
    pub token_id: String,
    pub price: i128,
    pub active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct Bid {
    pub bidder: Address,
    pub amount: i128,
}

#[contractclient(name = "TrustLinkClient")]
pub trait TrustLink {
    fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool;
    fn get_attestations_by_jurisdiction(
        env: Env,
        subject: Address,
        jurisdiction: String,
        start: u32,
        limit: u32,
    ) -> Vec<String>;
}

#[contract]
pub struct NftMarketplaceContract;

#[contractimpl]
impl NftMarketplaceContract {
    /// Initialise the marketplace with TrustLink and the allowed ISO jurisdiction code.
    pub fn initialize(
        env: Env,
        admin: Address,
        trustlink_contract: Address,
        allowed_jurisdiction: String,
    ) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::TrustLink, &trustlink_contract);
        env.storage()
            .instance()
            .set(&DataKey::AllowedJurisdiction, &allowed_jurisdiction);
        env.storage().instance().set(&DataKey::ListingCount, &0u32);
    }

    /// List an NFT for sale. Seller must have KYC and jurisdiction eligibility.
    pub fn list_item(env: Env, seller: Address, token_id: String, price: i128) -> u32 {
        seller.require_auth();

        if price <= 0 {
            panic!("price must be positive");
        }

        Self::require_marketplace_eligibility(&env, &seller);

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::ListingCount)
            .unwrap_or(0);
        let listing_id = count + 1;

        env.storage().instance().set(
            &DataKey::Listing(listing_id),
            &Listing {
                seller: seller.clone(),
                token_id: token_id.clone(),
                price,
                active: true,
            },
        );
        env.storage()
            .instance()
            .set(&DataKey::ListingCount, &listing_id);

        env.events()
            .publish((symbol_short!("listed"), listing_id, seller), token_id);

        listing_id
    }

    /// Place a bid on an active listing. Bidder must have KYC and jurisdiction eligibility.
    pub fn place_bid(env: Env, bidder: Address, listing_id: u32, amount: i128) {
        bidder.require_auth();

        if amount <= 0 {
            panic!("bid amount must be positive");
        }

        let listing: Listing = env
            .storage()
            .instance()
            .get(&DataKey::Listing(listing_id))
            .unwrap_or_else(|| panic!("listing not found"));

        if !listing.active {
            panic!("listing is not active");
        }

        if amount < listing.price {
            panic!("bid below listing price");
        }

        if let Some(existing) = env
            .storage()
            .instance()
            .get::<DataKey, Bid>(&DataKey::HighestBid(listing_id))
        {
            if amount <= existing.amount {
                panic!("bid must exceed current highest bid");
            }
        }

        Self::require_marketplace_eligibility(&env, &bidder);

        env.storage().instance().set(
            &DataKey::HighestBid(listing_id),
            &Bid {
                bidder: bidder.clone(),
                amount,
            },
        );

        env.events()
            .publish((symbol_short!("bid"), listing_id, bidder), amount);
    }

    pub fn get_listing(env: Env, listing_id: u32) -> Listing {
        env.storage()
            .instance()
            .get(&DataKey::Listing(listing_id))
            .unwrap_or_else(|| panic!("listing not found"))
    }

    pub fn get_highest_bid(env: Env, listing_id: u32) -> Option<Bid> {
        env.storage()
            .instance()
            .get(&DataKey::HighestBid(listing_id))
    }

    pub fn get_allowed_jurisdiction(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::AllowedJurisdiction)
            .unwrap()
    }

    pub fn get_trustlink(env: Env) -> Address {
        env.storage().instance().get(&DataKey::TrustLink).unwrap()
    }

    /// Enforce KYC_PASSED + jurisdiction eligibility via TrustLink.
    fn require_marketplace_eligibility(env: &Env, wallet: &Address) {
        let trustlink_id: Address = env.storage().instance().get(&DataKey::TrustLink).unwrap();
        let trustlink = TrustLinkClient::new(env, &trustlink_id);

        let kyc_claim = String::from_str(env, CLAIM_KYC_PASSED);
        if !trustlink.has_valid_claim(wallet, &kyc_claim) {
            panic!("KYC_PASSED attestation required");
        }

        let allowed: String = env
            .storage()
            .instance()
            .get(&DataKey::AllowedJurisdiction)
            .unwrap();
        let jurisdiction_hits =
            trustlink.get_attestations_by_jurisdiction(wallet, &allowed, &0, &1);
        if jurisdiction_hits.is_empty() {
            panic!("jurisdiction eligibility required");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    // ── Mock: fully eligible (KYC + matching jurisdiction) ────────────────────
    mod mock_eligible {
        use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

        #[contract]
        pub struct MockEligible;

        #[contractimpl]
        impl MockEligible {
            pub fn has_valid_claim(_env: Env, _subject: Address, claim_type: String) -> bool {
                claim_type == String::from_str(&_env, "KYC_PASSED")
            }

            pub fn get_attestations_by_jurisdiction(
                env: Env,
                _subject: Address,
                jurisdiction: String,
                _start: u32,
                _limit: u32,
            ) -> Vec<String> {
                let mut ids = Vec::new(&env);
                if jurisdiction == String::from_str(&env, "US") {
                    ids.push_back(String::from_str(&env, "att_us_eligible"));
                }
                ids
            }
        }
    }

    // ── Mock: no KYC, but jurisdiction match ──────────────────────────────────
    mod mock_no_kyc {
        use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

        #[contract]
        pub struct MockNoKyc;

        #[contractimpl]
        impl MockNoKyc {
            pub fn has_valid_claim(_env: Env, _subject: Address, _claim_type: String) -> bool {
                false
            }

            pub fn get_attestations_by_jurisdiction(
                env: Env,
                _subject: Address,
                _jurisdiction: String,
                _start: u32,
                _limit: u32,
            ) -> Vec<String> {
                let mut ids = Vec::new(&env);
                ids.push_back(String::from_str(&env, "att_us_eligible"));
                ids
            }
        }
    }

    // ── Mock: KYC ok, wrong / missing jurisdiction ────────────────────────────
    mod mock_wrong_jurisdiction {
        use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

        #[contract]
        pub struct MockWrongJurisdiction;

        #[contractimpl]
        impl MockWrongJurisdiction {
            pub fn has_valid_claim(_env: Env, _subject: Address, claim_type: String) -> bool {
                claim_type == String::from_str(&_env, "KYC_PASSED")
            }

            pub fn get_attestations_by_jurisdiction(
                env: Env,
                _subject: Address,
                _jurisdiction: String,
                _start: u32,
                _limit: u32,
            ) -> Vec<String> {
                Vec::new(&env)
            }
        }
    }

    // ── Hybrid mock: KYC when byte-sum even; jurisdiction when sum % 3 != 0 ───
    mod mock_seedable {
        use soroban_sdk::{contract, contractimpl, xdr::ToXdr, Address, Env, String, Vec};

        #[contract]
        pub struct MockSeedable;

        #[contractimpl]
        impl MockSeedable {
            pub fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool {
                if claim_type != String::from_str(&env, "KYC_PASSED") {
                    return false;
                }
                let bytes = subject.to_xdr(&env);
                let sum: u32 = bytes.iter().map(|b| b as u32).sum();
                sum % 2 == 0
            }

            pub fn get_attestations_by_jurisdiction(
                env: Env,
                subject: Address,
                jurisdiction: String,
                _start: u32,
                _limit: u32,
            ) -> Vec<String> {
                let mut ids = Vec::new(&env);
                if jurisdiction != String::from_str(&env, "US") {
                    return ids;
                }
                let bytes = subject.to_xdr(&env);
                let sum: u32 = bytes.iter().map(|b| b as u32).sum();
                if sum % 3 != 0 {
                    ids.push_back(String::from_str(&env, "att_us_eligible"));
                }
                ids
            }
        }
    }

    use mock_eligible::MockEligible;
    use mock_no_kyc::MockNoKyc;
    use mock_seedable::{MockSeedable, MockSeedableClient};
    use mock_wrong_jurisdiction::MockWrongJurisdiction;

    fn us_jurisdiction(env: &Env) -> String {
        String::from_str(env, "US")
    }

    fn setup_marketplace<'a>(
        env: &'a Env,
        trustlink_id: &Address,
    ) -> NftMarketplaceContractClient<'a> {
        let admin = Address::generate(env);
        let contract_id = env.register_contract(None, NftMarketplaceContract);
        let client = NftMarketplaceContractClient::new(env, &contract_id);
        client.initialize(&admin, trustlink_id, &us_jurisdiction(env));
        client
    }

    fn find_eligible(env: &Env, trustlink: &MockSeedableClient) -> Address {
        let kyc = String::from_str(env, "KYC_PASSED");
        let us = String::from_str(env, "US");
        for _ in 0..400 {
            let addr = Address::generate(env);
            if trustlink.has_valid_claim(&addr, &kyc)
                && !trustlink
                    .get_attestations_by_jurisdiction(&addr, &us, &0, &1)
                    .is_empty()
            {
                return addr;
            }
        }
        panic!("could not find eligible address");
    }

    fn find_no_kyc_but_jurisdiction(env: &Env, trustlink: &MockSeedableClient) -> Address {
        let kyc = String::from_str(env, "KYC_PASSED");
        let us = String::from_str(env, "US");
        for _ in 0..400 {
            let addr = Address::generate(env);
            if !trustlink.has_valid_claim(&addr, &kyc)
                && !trustlink
                    .get_attestations_by_jurisdiction(&addr, &us, &0, &1)
                    .is_empty()
            {
                return addr;
            }
        }
        panic!("could not find no-KYC address with jurisdiction");
    }

    fn find_kyc_wrong_jurisdiction(env: &Env, trustlink: &MockSeedableClient) -> Address {
        let kyc = String::from_str(env, "KYC_PASSED");
        let us = String::from_str(env, "US");
        for _ in 0..400 {
            let addr = Address::generate(env);
            if trustlink.has_valid_claim(&addr, &kyc)
                && trustlink
                    .get_attestations_by_jurisdiction(&addr, &us, &0, &1)
                    .is_empty()
            {
                return addr;
            }
        }
        panic!("could not find KYC address without jurisdiction");
    }

    #[test]
    fn list_allowed_when_kyc_and_jurisdiction_ok() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, MockEligible);
        let client = setup_marketplace(&env, &trustlink_id);
        let seller = Address::generate(&env);
        let token_id = String::from_str(&env, "nft-001");

        let listing_id = client.list_item(&seller, &token_id, &1_000);
        assert_eq!(listing_id, 1);

        let listing = client.get_listing(&listing_id);
        assert_eq!(listing.seller, seller);
        assert_eq!(listing.token_id, token_id);
        assert_eq!(listing.price, 1_000);
        assert!(listing.active);
    }

    #[test]
    fn list_rejected_without_kyc() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, MockNoKyc);
        let client = setup_marketplace(&env, &trustlink_id);
        let seller = Address::generate(&env);

        let result = client.try_list_item(&seller, &String::from_str(&env, "nft-001"), &500);
        assert!(result.is_err());
    }

    #[test]
    fn list_rejected_without_jurisdiction_eligibility() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, MockWrongJurisdiction);
        let client = setup_marketplace(&env, &trustlink_id);
        let seller = Address::generate(&env);

        let result = client.try_list_item(&seller, &String::from_str(&env, "nft-001"), &500);
        assert!(result.is_err());
    }

    #[test]
    fn bid_allowed_when_kyc_and_jurisdiction_ok() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, MockEligible);
        let client = setup_marketplace(&env, &trustlink_id);
        let seller = Address::generate(&env);
        let bidder = Address::generate(&env);

        let listing_id = client.list_item(&seller, &String::from_str(&env, "nft-002"), &1_000);
        client.place_bid(&bidder, &listing_id, &1_250);

        let bid = client.get_highest_bid(&listing_id).unwrap();
        assert_eq!(bid.bidder, bidder);
        assert_eq!(bid.amount, 1_250);
    }

    #[test]
    fn bid_rejected_without_kyc_seedable() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, MockSeedable);
        let trustlink = MockSeedableClient::new(&env, &trustlink_id);
        let client = setup_marketplace(&env, &trustlink_id);

        let seller = find_eligible(&env, &trustlink);
        let bidder = find_no_kyc_but_jurisdiction(&env, &trustlink);

        let listing_id = client.list_item(&seller, &String::from_str(&env, "nft-003"), &1_000);
        let result = client.try_place_bid(&bidder, &listing_id, &1_100);
        assert!(result.is_err());
    }

    #[test]
    fn bid_rejected_without_jurisdiction_eligibility() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, MockSeedable);
        let trustlink = MockSeedableClient::new(&env, &trustlink_id);
        let client = setup_marketplace(&env, &trustlink_id);

        let seller = find_eligible(&env, &trustlink);
        let bidder = find_kyc_wrong_jurisdiction(&env, &trustlink);

        let listing_id = client.list_item(&seller, &String::from_str(&env, "nft-004"), &1_000);
        let result = client.try_place_bid(&bidder, &listing_id, &1_100);
        assert!(result.is_err());
    }

    #[test]
    fn bid_allowed_with_seedable_eligible_parties() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, MockSeedable);
        let trustlink = MockSeedableClient::new(&env, &trustlink_id);
        let client = setup_marketplace(&env, &trustlink_id);

        let seller = find_eligible(&env, &trustlink);
        let bidder = find_eligible(&env, &trustlink);

        let listing_id = client.list_item(&seller, &String::from_str(&env, "nft-005"), &2_000);
        client.place_bid(&bidder, &listing_id, &2_500);

        let bid = client.get_highest_bid(&listing_id).unwrap();
        assert_eq!(bid.bidder, bidder);
        assert_eq!(bid.amount, 2_500);
    }

    #[test]
    fn list_rejected_without_kyc_seedable() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, MockSeedable);
        let trustlink = MockSeedableClient::new(&env, &trustlink_id);
        let client = setup_marketplace(&env, &trustlink_id);

        let seller = find_no_kyc_but_jurisdiction(&env, &trustlink);
        let result = client.try_list_item(&seller, &String::from_str(&env, "nft-006"), &100);
        assert!(result.is_err());
    }

    #[test]
    fn list_rejected_without_jurisdiction_seedable() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, MockSeedable);
        let trustlink = MockSeedableClient::new(&env, &trustlink_id);
        let client = setup_marketplace(&env, &trustlink_id);

        let seller = find_kyc_wrong_jurisdiction(&env, &trustlink);
        let result = client.try_list_item(&seller, &String::from_str(&env, "nft-007"), &100);
        assert!(result.is_err());
    }

    #[test]
    fn accessors_return_configured_values() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, MockEligible);
        let client = setup_marketplace(&env, &trustlink_id);

        assert_eq!(client.get_trustlink(), trustlink_id);
        assert_eq!(client.get_allowed_jurisdiction(), us_jurisdiction(&env));
        assert!(client.get_highest_bid(&1).is_none());
    }
}
