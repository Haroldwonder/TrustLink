//! Lending Pool Reference Integration (TrustLink IssuerTier Gating)
//!
//! Demonstrates a minimal collateralized lending pool that queries TrustLink
//! for the borrower's credit attestation, resolves the attesting issuer's
//! `IssuerTier`, and adjusts loan-to-value (LTV) and liquidation thresholds
//! accordingly.
//!
//! Tier → risk parameters (basis points, 10_000 = 100%):
//!
//! | IssuerTier | Max LTV | Liquidation threshold |
//! |------------|---------|------------------------|
//! | Basic      | 5_000   | 6_000                  |
//! | Verified   | 6_500   | 7_500                  |
//! | Premium    | 8_000   | 9_000                  |
//!
//! Borrowers without a valid `CREDITWORTHY` attestation cannot borrow.

#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, Address, Env, String,
};

/// Claim type required for borrowing — issued by a TrustLink issuer.
pub const CLAIM_CREDITWORTHY: &str = "CREDITWORTHY";

const BPS_DENOMINATOR: i128 = 10_000;

// ── Storage ───────────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    TrustLink,
    PoolLiquidity,
    Position(Address),
}

// ── TrustLink types (cross-contract) ──────────────────────────────────────────

/// Mirrors TrustLink's `IssuerTier` for cross-contract encoding.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum IssuerTier {
    Basic = 0,
    Verified = 1,
    Premium = 2,
}

/// Minimal attestation fields needed to resolve the issuer for tier lookup.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestationRef {
    pub issuer: Address,
    pub subject: Address,
    pub claim_type: String,
    pub revoked: bool,
}

#[contractclient(name = "TrustLinkClient")]
pub trait TrustLink {
    fn get_attestation_by_type(
        env: Env,
        subject: Address,
        claim_type: String,
    ) -> Option<AttestationRef>;

    fn get_issuer_tier(env: Env, issuer: Address) -> Option<IssuerTier>;
}

// ── Pool types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    pub collateral: i128,
    pub debt: i128,
    /// IssuerTier rank used when the loan was opened (0=Basic, 1=Verified, 2=Premium).
    pub tier_rank: u32,
    pub max_ltv_bps: u32,
    pub liquidation_threshold_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TierParams {
    pub max_ltv_bps: u32,
    pub liquidation_threshold_bps: u32,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct LendingPoolContract;

#[contractimpl]
impl LendingPoolContract {
    /// Initialise the pool with an admin and the TrustLink contract address.
    pub fn initialize(env: Env, admin: Address, trustlink_contract: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::TrustLink, &trustlink_contract);
        env.storage().instance().set(&DataKey::PoolLiquidity, &0i128);
    }

    /// Deposit borrowable liquidity into the pool (lenders / market makers).
    pub fn deposit(env: Env, lender: Address, amount: i128) {
        lender.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }
        let liquidity: i128 = env
            .storage()
            .instance()
            .get(&DataKey::PoolLiquidity)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::PoolLiquidity, &(liquidity + amount));
        env.events()
            .publish((symbol_short!("deposit"), lender), amount);
    }

    /// Post collateral against a future (or existing) borrow position.
    pub fn deposit_collateral(env: Env, borrower: Address, amount: i128) {
        borrower.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let mut position = Self::load_or_default(&env, &borrower);
        position.collateral += amount;
        env.storage()
            .instance()
            .set(&DataKey::Position(borrower.clone()), &position);

        env.events()
            .publish((symbol_short!("coll_dep"), borrower), amount);
    }

    /// Borrow against collateral. LTV is gated by the attesting issuer's tier.
    pub fn borrow(env: Env, borrower: Address, amount: i128) {
        borrower.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let liquidity: i128 = env
            .storage()
            .instance()
            .get(&DataKey::PoolLiquidity)
            .unwrap_or(0);
        if amount > liquidity {
            panic!("insufficient pool liquidity");
        }

        let (tier, params) = Self::resolve_borrower_params(&env, &borrower);
        let mut position = Self::load_or_default(&env, &borrower);

        let new_debt = position.debt + amount;
        if position.collateral == 0 {
            panic!("no collateral");
        }
        // Enforce max LTV: debt / collateral <= max_ltv_bps / 10_000
        let max_debt = position.collateral * (params.max_ltv_bps as i128) / BPS_DENOMINATOR;
        if new_debt > max_debt {
            panic!("exceeds max LTV for issuer tier");
        }

        position.debt = new_debt;
        position.tier_rank = tier as u32;
        position.max_ltv_bps = params.max_ltv_bps;
        position.liquidation_threshold_bps = params.liquidation_threshold_bps;

        env.storage()
            .instance()
            .set(&DataKey::Position(borrower.clone()), &position);
        env.storage()
            .instance()
            .set(&DataKey::PoolLiquidity, &(liquidity - amount));

        env.events()
            .publish((symbol_short!("borrow"), borrower, tier as u32), amount);
    }

    /// Liquidate an undercollateralized position.
    ///
    /// A position is liquidatable when:
    /// `debt / collateral > liquidation_threshold_bps / 10_000`
    /// (equivalently: collateral value has fallen relative to debt below the
    /// tier's liquidation threshold). For this example assets are 1:1 priced.
    pub fn liquidate(env: Env, liquidator: Address, borrower: Address) -> i128 {
        liquidator.require_auth();

        let position: Position = env
            .storage()
            .instance()
            .get(&DataKey::Position(borrower.clone()))
            .unwrap_or_else(|| panic!("no position"));

        if position.debt == 0 {
            panic!("no debt to liquidate");
        }
        if !Self::is_liquidatable(&position) {
            panic!("position is healthy");
        }

        let seized = position.collateral;
        let repaid = position.debt;

        // Clear the position; return seized collateral accounting to liquidator
        // and restore repaid debt to pool liquidity (simplified settlement).
        env.storage()
            .instance()
            .remove(&DataKey::Position(borrower.clone()));

        let liquidity: i128 = env
            .storage()
            .instance()
            .get(&DataKey::PoolLiquidity)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::PoolLiquidity, &(liquidity + repaid));

        env.events().publish(
            (symbol_short!("liquidate"), liquidator, borrower),
            (seized, repaid),
        );
        seized
    }

    /// Read a borrower's open position, if any.
    pub fn get_position(env: Env, borrower: Address) -> Option<Position> {
        env.storage()
            .instance()
            .get(&DataKey::Position(borrower))
    }

    /// Current pool liquidity available to borrow.
    pub fn get_pool_liquidity(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::PoolLiquidity)
            .unwrap_or(0)
    }

    /// Resolve the IssuerTier of the issuer who attested the borrower as CREDITWORTHY.
    pub fn get_borrower_tier(env: Env, borrower: Address) -> Option<IssuerTier> {
        Self::lookup_borrower_tier(&env, &borrower)
    }

    /// Risk parameters for a given IssuerTier.
    pub fn get_tier_params(env: Env, tier: IssuerTier) -> TierParams {
        let _ = env;
        Self::params_for_tier(tier)
    }

    /// Whether a borrower's position is currently liquidatable.
    pub fn is_position_liquidatable(env: Env, borrower: Address) -> bool {
        match env
            .storage()
            .instance()
            .get::<_, Position>(&DataKey::Position(borrower))
        {
            Some(position) => Self::is_liquidatable(&position),
            None => false,
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn load_or_default(env: &Env, borrower: &Address) -> Position {
        env.storage()
            .instance()
            .get(&DataKey::Position(borrower.clone()))
            .unwrap_or(Position {
                collateral: 0,
                debt: 0,
                tier_rank: 0,
                max_ltv_bps: 0,
                liquidation_threshold_bps: 0,
            })
    }

    fn params_for_tier(tier: IssuerTier) -> TierParams {
        match tier {
            IssuerTier::Basic => TierParams {
                max_ltv_bps: 5_000,
                liquidation_threshold_bps: 6_000,
            },
            IssuerTier::Verified => TierParams {
                max_ltv_bps: 6_500,
                liquidation_threshold_bps: 7_500,
            },
            IssuerTier::Premium => TierParams {
                max_ltv_bps: 8_000,
                liquidation_threshold_bps: 9_000,
            },
        }
    }

    fn lookup_borrower_tier(env: &Env, borrower: &Address) -> Option<IssuerTier> {
        let trustlink_id: Address = env.storage().instance().get(&DataKey::TrustLink).unwrap();
        let trustlink = TrustLinkClient::new(env, &trustlink_id);
        let claim = String::from_str(env, CLAIM_CREDITWORTHY);

        let attestation = trustlink.get_attestation_by_type(borrower, &claim)?;
        if attestation.revoked {
            return None;
        }
        trustlink.get_issuer_tier(&attestation.issuer)
    }

    fn resolve_borrower_params(env: &Env, borrower: &Address) -> (IssuerTier, TierParams) {
        let tier = Self::lookup_borrower_tier(env, borrower)
            .unwrap_or_else(|| panic!("borrower must have CREDITWORTHY attestation from a tiered issuer"));
        (tier, Self::params_for_tier(tier))
    }

    fn is_liquidatable(position: &Position) -> bool {
        if position.debt == 0 || position.collateral == 0 {
            return position.debt > 0;
        }
        // Liquidatable when debt/collateral > threshold/10_000
        // <=> debt * 10_000 > collateral * threshold
        position.debt * BPS_DENOMINATOR
            > position.collateral * (position.liquidation_threshold_bps as i128)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, String};

    // Mock TrustLink: stores CREDITWORTHY attestations and issuer tiers.
    #[contract]
    struct MockTrustLink;

    #[contracttype]
    enum MockKey {
        Attestation(Address),
        Tier(Address),
    }

    #[contractimpl]
    impl MockTrustLink {
        pub fn set_attestation(env: Env, subject: Address, issuer: Address, revoked: bool) {
            let claim = String::from_str(&env, CLAIM_CREDITWORTHY);
            env.storage().instance().set(
                &MockKey::Attestation(subject.clone()),
                &AttestationRef {
                    issuer,
                    subject,
                    claim_type: claim,
                    revoked,
                },
            );
        }

        pub fn set_issuer_tier(env: Env, issuer: Address, tier: IssuerTier) {
            env.storage()
                .instance()
                .set(&MockKey::Tier(issuer), &tier);
        }

        pub fn get_attestation_by_type(
            env: Env,
            subject: Address,
            claim_type: String,
        ) -> Option<AttestationRef> {
            let expected = String::from_str(&env, CLAIM_CREDITWORTHY);
            if claim_type != expected {
                return None;
            }
            env.storage()
                .instance()
                .get(&MockKey::Attestation(subject))
        }

        pub fn get_issuer_tier(env: Env, issuer: Address) -> Option<IssuerTier> {
            env.storage().instance().get(&MockKey::Tier(issuer))
        }
    }

    fn setup(env: &Env) -> (LendingPoolContractClient<'_>, MockTrustLinkClient<'_>, Address) {
        env.mock_all_auths();
        let admin = Address::generate(env);
        let trustlink_id = env.register_contract(None, MockTrustLink);
        let pool_id = env.register_contract(None, LendingPoolContract);
        let pool = LendingPoolContractClient::new(env, &pool_id);
        let trustlink = MockTrustLinkClient::new(env, &trustlink_id);
        pool.initialize(&admin, &trustlink_id);
        (pool, trustlink, admin)
    }

    fn attest(
        trustlink: &MockTrustLinkClient,
        subject: &Address,
        issuer: &Address,
        tier: IssuerTier,
    ) {
        trustlink.set_issuer_tier(issuer, &tier);
        trustlink.set_attestation(subject, issuer, &false);
    }

    #[test]
    fn tier_params_match_spec() {
        let env = Env::default();
        let (pool, _, _) = setup(&env);

        let basic = pool.get_tier_params(&IssuerTier::Basic);
        assert_eq!(basic.max_ltv_bps, 5_000);
        assert_eq!(basic.liquidation_threshold_bps, 6_000);

        let verified = pool.get_tier_params(&IssuerTier::Verified);
        assert_eq!(verified.max_ltv_bps, 6_500);
        assert_eq!(verified.liquidation_threshold_bps, 7_500);

        let premium = pool.get_tier_params(&IssuerTier::Premium);
        assert_eq!(premium.max_ltv_bps, 8_000);
        assert_eq!(premium.liquidation_threshold_bps, 9_000);
    }

    #[test]
    fn borrow_rejected_without_attestation() {
        let env = Env::default();
        let (pool, _, _) = setup(&env);
        let lender = Address::generate(&env);
        let borrower = Address::generate(&env);

        pool.deposit(&lender, &10_000);
        pool.deposit_collateral(&borrower, &1_000);

        let result = pool.try_borrow(&borrower, &100);
        assert!(result.is_err());
    }

    #[test]
    fn basic_tier_enforces_50_percent_ltv() {
        let env = Env::default();
        let (pool, trustlink, _) = setup(&env);
        let lender = Address::generate(&env);
        let borrower = Address::generate(&env);
        let issuer = Address::generate(&env);

        attest(&trustlink, &borrower, &issuer, IssuerTier::Basic);
        assert_eq!(
            pool.get_borrower_tier(&borrower),
            Some(IssuerTier::Basic)
        );

        pool.deposit(&lender, &10_000);
        pool.deposit_collateral(&borrower, &1_000);

        // 50% of 1000 = 500 — allowed
        pool.borrow(&borrower, &500);
        let position = pool.get_position(&borrower).unwrap();
        assert_eq!(position.debt, 500);
        assert_eq!(position.max_ltv_bps, 5_000);

        // One more unit exceeds Basic max LTV
        let over = pool.try_borrow(&borrower, &1);
        assert!(over.is_err());
    }

    #[test]
    fn premium_tier_allows_higher_ltv_than_basic() {
        let env = Env::default();
        let (pool, trustlink, _) = setup(&env);
        let lender = Address::generate(&env);
        let borrower = Address::generate(&env);
        let issuer = Address::generate(&env);

        attest(&trustlink, &borrower, &issuer, IssuerTier::Premium);
        pool.deposit(&lender, &10_000);
        pool.deposit_collateral(&borrower, &1_000);

        // Premium max LTV = 80% → 800
        pool.borrow(&borrower, &800);
        let position = pool.get_position(&borrower).unwrap();
        assert_eq!(position.debt, 800);
        assert_eq!(position.tier_rank, IssuerTier::Premium as u32);
        assert_eq!(position.max_ltv_bps, 8_000);
    }

    #[test]
    fn verified_tier_ltv_between_basic_and_premium() {
        let env = Env::default();
        let (pool, trustlink, _) = setup(&env);
        let lender = Address::generate(&env);
        let borrower = Address::generate(&env);
        let issuer = Address::generate(&env);

        attest(&trustlink, &borrower, &issuer, IssuerTier::Verified);
        pool.deposit(&lender, &10_000);
        pool.deposit_collateral(&borrower, &1_000);

        // 65% of 1000 = 650
        pool.borrow(&borrower, &650);
        assert!(pool.try_borrow(&borrower, &1).is_err());
    }

    #[test]
    fn liquidate_rejects_healthy_position() {
        let env = Env::default();
        let (pool, trustlink, _) = setup(&env);
        let lender = Address::generate(&env);
        let borrower = Address::generate(&env);
        let liquidator = Address::generate(&env);
        let issuer = Address::generate(&env);

        attest(&trustlink, &borrower, &issuer, IssuerTier::Verified);
        pool.deposit(&lender, &10_000);
        pool.deposit_collateral(&borrower, &1_000);
        // Debt 500 / coll 1000 = 50% < 75% liquidation threshold
        pool.borrow(&borrower, &500);

        assert!(!pool.is_position_liquidatable(&borrower));
        assert!(pool.try_liquidate(&liquidator, &borrower).is_err());
    }

    #[test]
    fn full_deposit_borrow_liquidate_flow() {
        let env = Env::default();
        let (pool, trustlink, _) = setup(&env);

        let lender = Address::generate(&env);
        let borrower = Address::generate(&env);
        let liquidator = Address::generate(&env);
        let issuer = Address::generate(&env);

        // 1. Attest borrower via a Basic-tier issuer (liq threshold = 60%)
        attest(&trustlink, &borrower, &issuer, IssuerTier::Basic);

        // 2. Deposit liquidity
        pool.deposit(&lender, &5_000);
        assert_eq!(pool.get_pool_liquidity(), 5_000);

        // 3. Post collateral and borrow at Basic max LTV (50%)
        pool.deposit_collateral(&borrower, &1_000);
        pool.borrow(&borrower, &500);
        assert_eq!(pool.get_pool_liquidity(), 4_500);

        let position = pool.get_position(&borrower).unwrap();
        assert_eq!(position.collateral, 1_000);
        assert_eq!(position.debt, 500);
        assert_eq!(position.liquidation_threshold_bps, 6_000);
        assert!(!pool.is_position_liquidatable(&borrower));

        // 4. Simulate collateral drop: clear and re-post reduced collateral so
        //    debt/collateral = 500/800 = 62.5% > 60% Basic liquidation threshold.
        //    (In production an oracle would reprice; here we adjust units 1:1.)
        env.as_contract(&pool.address, || {
            env.storage()
                .instance()
                .set(
                    &DataKey::Position(borrower.clone()),
                    &Position {
                        collateral: 800,
                        debt: 500,
                        tier_rank: position.tier_rank,
                        max_ltv_bps: position.max_ltv_bps,
                        liquidation_threshold_bps: position.liquidation_threshold_bps,
                    },
                );
        });

        assert!(pool.is_position_liquidatable(&borrower));

        // 5. Liquidate
        let seized = pool.liquidate(&liquidator, &borrower);
        assert_eq!(seized, 800);
        assert!(pool.get_position(&borrower).is_none());
        // Repaid debt restored to pool: 4_500 + 500 = 5_000
        assert_eq!(pool.get_pool_liquidity(), 5_000);
    }

    #[test]
    fn premium_survives_price_drop_that_liquidates_basic() {
        let env = Env::default();
        let (pool, trustlink, _) = setup(&env);

        let lender = Address::generate(&env);
        let basic_borrower = Address::generate(&env);
        let premium_borrower = Address::generate(&env);
        let basic_issuer = Address::generate(&env);
        let premium_issuer = Address::generate(&env);

        attest(
            &trustlink,
            &basic_borrower,
            &basic_issuer,
            IssuerTier::Basic,
        );
        attest(
            &trustlink,
            &premium_borrower,
            &premium_issuer,
            IssuerTier::Premium,
        );

        pool.deposit(&lender, &10_000);
        pool.deposit_collateral(&basic_borrower, &1_000);
        pool.deposit_collateral(&premium_borrower, &1_000);

        // Both borrow 500 (50% LTV — within both tiers)
        pool.borrow(&basic_borrower, &500);
        pool.borrow(&premium_borrower, &500);

        // Drop collateral to 800: ratio = 62.5%
        // Basic liq threshold 60% → liquidatable
        // Premium liq threshold 90% → healthy
        for (borrower, tier_rank, max_ltv, liq) in [
            (basic_borrower.clone(), 0u32, 5_000u32, 6_000u32),
            (premium_borrower.clone(), 2u32, 8_000u32, 9_000u32),
        ] {
            env.as_contract(&pool.address, || {
                env.storage().instance().set(
                    &DataKey::Position(borrower),
                    &Position {
                        collateral: 800,
                        debt: 500,
                        tier_rank,
                        max_ltv_bps: max_ltv,
                        liquidation_threshold_bps: liq,
                    },
                );
            });
        }

        assert!(pool.is_position_liquidatable(&basic_borrower));
        assert!(!pool.is_position_liquidatable(&premium_borrower));
    }

    #[test]
    fn revoked_attestation_blocks_borrow() {
        let env = Env::default();
        let (pool, trustlink, _) = setup(&env);
        let lender = Address::generate(&env);
        let borrower = Address::generate(&env);
        let issuer = Address::generate(&env);

        trustlink.set_issuer_tier(&issuer, &IssuerTier::Verified);
        trustlink.set_attestation(&borrower, &issuer, &true); // revoked

        pool.deposit(&lender, &10_000);
        pool.deposit_collateral(&borrower, &1_000);
        assert!(pool.try_borrow(&borrower, &100).is_err());
    }

    #[test]
    fn borrow_requires_collateral() {
        let env = Env::default();
        let (pool, trustlink, _) = setup(&env);
        let lender = Address::generate(&env);
        let borrower = Address::generate(&env);
        let issuer = Address::generate(&env);

        attest(&trustlink, &borrower, &issuer, IssuerTier::Premium);
        pool.deposit(&lender, &10_000);
        assert!(pool.try_borrow(&borrower, &100).is_err());
    }
}
