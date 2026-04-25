#![cfg(test)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, testutils::Address as _, Address, Env,
    String,
};

use trustlink::{TrustLinkContract, TrustLinkContractClient};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum LendingError {
    KYCRequired = 1,
    InsufficientCollateral = 2,
}

#[contracttype]
#[derive(Clone)]
pub struct LoanRequest {
    pub borrower: Address,
    pub amount: i128,
    pub collateral: i128,
}

#[contract]
pub struct LendingContract;

#[contractimpl]
impl LendingContract {
    pub fn request_loan(
        env: Env,
        borrower: Address,
        trustlink_contract: Address,
        amount: i128,
        collateral: i128,
    ) -> Result<(), LendingError> {
        borrower.require_auth();

        let trustlink = TrustLinkContractClient::new(&env, &trustlink_contract);
        let kyc_claim = String::from_str(&env, "KYC_PASSED");

        if !trustlink.has_valid_claim(&borrower, &kyc_claim) {
            return Err(LendingError::KYCRequired);
        }

        if collateral < amount / 2 {
            return Err(LendingError::InsufficientCollateral);
        }

        let loan = LoanRequest {
            borrower: borrower.clone(),
            amount,
            collateral,
        };

        env.storage().instance().set(&borrower, &loan);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Ledger;

    fn setup_trustlink(env: &Env) -> (TrustLinkContractClient, Address, Address, Address) {
        let trustlink_id = env.register_contract(None, TrustLinkContract);
        let trustlink = TrustLinkContractClient::new(env, &trustlink_id);

        let admin = Address::generate(env);
        let issuer = Address::generate(env);
        let borrower = Address::generate(env);

        trustlink.initialize(&admin, &None);
        trustlink.register_issuer(&admin, &issuer);

        (trustlink, admin, issuer, borrower)
    }

    #[test]
    fn test_loan_denied_without_kyc() {
        let env = Env::default();
        env.mock_all_auths();

        let (trustlink, _admin, _issuer, borrower) = setup_trustlink(&env);
        let trustlink_id = trustlink.address.clone();

        let lending_id = env.register_contract(None, LendingContract);
        let lending = LendingContractClient::new(&env, &lending_id);

        let result = lending.try_request_loan(&borrower, &trustlink_id, &1_000, &500);
        assert!(result.is_err());
    }

    #[test]
    fn test_loan_approved_with_kyc() {
        let env = Env::default();
        env.mock_all_auths();

        let (trustlink, admin, issuer, borrower) = setup_trustlink(&env);
        let trustlink_id = trustlink.address.clone();
        let kyc_claim = String::from_str(&env, "KYC_PASSED");

        let lending_id = env.register_contract(None, LendingContract);
        let lending = LendingContractClient::new(&env, &lending_id);

        env.ledger().with_mut(|li| li.timestamp = 5_000);
        trustlink.import_attestation(&admin, &issuer, &borrower, &kyc_claim, &1_000, &None);

        let result = lending.try_request_loan(&borrower, &trustlink_id, &1_000, &500);
        assert!(result.is_ok());
    }

    #[test]
    fn test_loan_denied_after_kyc_revocation() {
        let env = Env::default();
        env.mock_all_auths();

        let (trustlink, admin, issuer, borrower) = setup_trustlink(&env);
        let trustlink_id = trustlink.address.clone();
        let kyc_claim = String::from_str(&env, "KYC_PASSED");

        let lending_id = env.register_contract(None, LendingContract);
        let lending = LendingContractClient::new(&env, &lending_id);

        env.ledger().with_mut(|li| li.timestamp = 5_000);
        let attestation_id =
            trustlink.import_attestation(&admin, &issuer, &borrower, &kyc_claim, &1_000, &None);

        let approved = lending.try_request_loan(&borrower, &trustlink_id, &1_000, &500);
        assert!(approved.is_ok());

        trustlink.revoke_attestation(&issuer, &attestation_id, &None);

        let denied = lending.try_request_loan(&borrower, &trustlink_id, &1_000, &500);
        assert!(denied.is_err());
    }

    #[test]
    fn test_loan_denied_after_kyc_expiration() {
        let env = Env::default();
        env.mock_all_auths();

        let (trustlink, admin, issuer, borrower) = setup_trustlink(&env);
        let trustlink_id = trustlink.address.clone();
        let kyc_claim = String::from_str(&env, "KYC_PASSED");

        let lending_id = env.register_contract(None, LendingContract);
        let lending = LendingContractClient::new(&env, &lending_id);

        env.ledger().with_mut(|li| li.timestamp = 5_000);
        // expiration = 10_000
        trustlink.import_attestation(
            &admin,
            &issuer,
            &borrower,
            &kyc_claim,
            &1_000,
            &Some(10_000),
        );

        let approved = lending.try_request_loan(&borrower, &trustlink_id, &1_000, &500);
        assert!(approved.is_ok());

        // advance past expiration
        env.ledger().with_mut(|li| li.timestamp = 10_001);

        let denied = lending.try_request_loan(&borrower, &trustlink_id, &1_000, &500);
        assert!(denied.is_err());
    }

    #[test]
    fn test_50_attestations_rapid_succession() {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();

        let (trustlink, admin, issuer, subject) = setup_trustlink(&env);

        env.ledger().with_mut(|li| li.timestamp = 10_000);

        // create 50 attestations with unique claim types for the same subject
        let mut ids = std::vec![];
        for i in 0u32..50 {
            let claim_type = String::from_str(&env, &std::format!("CLAIM_{i}"));
            let id =
                trustlink.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
            ids.push(id);
        }

        // all 50 stored — no duplicates or collisions
        assert_eq!(ids.len(), 50, "all 50 IDs should be unique");

        // pagination: fetch all 50 in one page
        let page = trustlink.get_subject_attestations(&subject, &0, &50);
        assert_eq!(page.len(), 50);

        // pagination: two pages of 25
        let page1 = trustlink.get_subject_attestations(&subject, &0, &25);
        let page2 = trustlink.get_subject_attestations(&subject, &25, &25);
        assert_eq!(page1.len(), 25);
        assert_eq!(page2.len(), 25);

        // has_valid_claim works for a claim that exists
        let known_claim = String::from_str(&env, "CLAIM_0");
        assert!(trustlink.has_valid_claim(&subject, &known_claim));

        // has_valid_claim returns false for a claim that was never issued
        let unknown_claim = String::from_str(&env, "CLAIM_99");
        assert!(!trustlink.has_valid_claim(&subject, &unknown_claim));

        // every stored attestation is individually retrievable
        for id in &ids {
            let attestation = trustlink.get_attestation(id);
            assert_eq!(attestation.subject, subject);
            assert_eq!(attestation.issuer, issuer);
        }
    }

    #[test]
    fn test_imported_attestation_allows_cross_contract_verification() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, TrustLinkContract);
        let trustlink = TrustLinkContractClient::new(&env, &trustlink_id);

        let lending_id = env.register_contract(None, LendingContract);
        let lending = LendingContractClient::new(&env, &lending_id);

        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let borrower = Address::generate(&env);
        let kyc_claim = String::from_str(&env, "KYC_PASSED");

        trustlink.initialize(&admin, &None);
        trustlink.register_issuer(&admin, &issuer);

        let denied = lending.try_request_loan(&borrower, &trustlink_id, &1_000, &500);
        assert!(denied.is_err());

        env.ledger().with_mut(|li| li.timestamp = 5_000);
        trustlink.import_attestation(&admin, &issuer, &borrower, &kyc_claim, &1_000, &None);

        let approved = lending.try_request_loan(&borrower, &trustlink_id, &1_000, &500);
        assert!(approved.is_ok());
    }
}
    

// =============================================================================
// Comprehensive KYC-gated lending scenario
//
// This module adds a richer mock lending contract and a full end-to-end
// scenario that walks through every KYC state transition in sequence:
//
//   1. Borrower has no KYC              → loan denied  (KYCRequired)
//   2. Borrower receives KYC            → loan approved
//   3. KYC revoked mid-loan             → subsequent borrow denied (KYCRequired)
//   4. New KYC issued but expires       → subsequent borrow denied (KYCRequired)
//
// The mock lending contract is intentionally minimal: it checks KYC on every
// call so that state changes in TrustLink are immediately visible.
// =============================================================================

/// A second, independent lending contract used only by this module so that
/// the existing `LendingContract` above is not disturbed.
#[contract]
pub struct KycLendingContract;

#[contractimpl]
impl KycLendingContract {
    /// Attempt to open a loan position.
    ///
    /// Checks `KYC_PASSED` on every call — no cached state — so that any
    /// change to the TrustLink attestation is reflected immediately.
    ///
    /// # Errors
    /// - [`LendingError::KYCRequired`]           — borrower has no valid KYC.
    /// - [`LendingError::InsufficientCollateral`] — collateral < amount / 2.
    pub fn open_loan(
        env: Env,
        borrower: Address,
        trustlink_id: Address,
        amount: i128,
        collateral: i128,
    ) -> Result<(), LendingError> {
        borrower.require_auth();

        let trustlink = TrustLinkContractClient::new(&env, &trustlink_id);
        let kyc_claim = String::from_str(&env, "KYC_PASSED");

        if !trustlink.has_valid_claim(&borrower, &kyc_claim) {
            return Err(LendingError::KYCRequired);
        }

        if collateral < amount / 2 {
            return Err(LendingError::InsufficientCollateral);
        }

        // Record the open position keyed by borrower.
        env.storage().instance().set(&borrower, &amount);
        Ok(())
    }

    /// Return the outstanding loan amount for `borrower`, or 0 if none.
    pub fn loan_balance(env: Env, borrower: Address) -> i128 {
        env.storage()
            .instance()
            .get::<Address, i128>(&borrower)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod kyc_lending_scenario {
    use super::*;
    use soroban_sdk::testutils::Ledger;

    // -------------------------------------------------------------------------
    // Shared setup
    // -------------------------------------------------------------------------

    /// Deploy TrustLink + KycLendingContract and return ready-to-use clients
    /// together with the key actors.
    ///
    /// Returns `(trustlink, lending, trustlink_id, admin, issuer, borrower)`.
    fn deploy(
        env: &Env,
    ) -> (
        TrustLinkContractClient<'_>,
        KycLendingContractClient<'_>,
        Address, // trustlink contract address
        Address, // admin
        Address, // issuer
        Address, // borrower
    ) {
        let trustlink_id = env.register_contract(None, TrustLinkContract);
        let trustlink = TrustLinkContractClient::new(env, &trustlink_id);

        let lending_id = env.register_contract(None, KycLendingContract);
        let lending = KycLendingContractClient::new(env, &lending_id);

        let admin = Address::generate(env);
        let issuer = Address::generate(env);
        let borrower = Address::generate(env);

        // Initialise TrustLink at a non-zero timestamp so that native
        // attestations (which use the current ledger timestamp as their own
        // timestamp) are always in the past relative to any future expiration.
        env.ledger().with_mut(|li| li.timestamp = 1_000);
        trustlink.initialize(&admin, &None);
        trustlink.register_issuer(&admin, &issuer);

        (trustlink, lending, trustlink_id, admin, issuer, borrower)
    }

    /// Issue a native KYC attestation for `borrower` at the current ledger
    /// timestamp, optionally with an expiration.
    fn issue_kyc(
        env: &Env,
        trustlink: &TrustLinkContractClient<'_>,
        issuer: &Address,
        borrower: &Address,
        expiration: Option<u64>,
    ) -> String {
        let kyc_claim = String::from_str(env, "KYC_PASSED");
        trustlink.create_attestation(issuer, borrower, &kyc_claim, &expiration, &None, &None)
    }

    // -------------------------------------------------------------------------
    // Comprehensive end-to-end scenario
    // -------------------------------------------------------------------------

    /// Walk through all four KYC state transitions in a single scenario so
    /// that the causal chain is explicit and easy to follow in CI output.
    #[test]
    fn test_kyc_lending_full_lifecycle() {
        let env = Env::default();
        env.mock_all_auths();

        let (trustlink, lending, trustlink_id, _admin, issuer, borrower) = deploy(&env);

        // ------------------------------------------------------------------
        // Phase 1: no KYC → loan denied
        // ------------------------------------------------------------------
        let result = lending.try_open_loan(&borrower, &trustlink_id, &1_000, &500);
        assert_eq!(
            result,
            Err(Ok(LendingError::KYCRequired)),
            "Phase 1: borrower without KYC must be denied"
        );
        assert_eq!(
            lending.loan_balance(&borrower),
            0,
            "Phase 1: no loan position must be recorded"
        );

        // ------------------------------------------------------------------
        // Phase 2: KYC issued → loan approved
        // ------------------------------------------------------------------
        // Advance time so the attestation timestamp is strictly in the past
        // relative to any future expiration we set later.
        env.ledger().with_mut(|li| li.timestamp = 5_000);
        let attestation_id = issue_kyc(&env, &trustlink, &issuer, &borrower, None);

        let result = lending.try_open_loan(&borrower, &trustlink_id, &1_000, &500);
        assert_eq!(
            result,
            Ok(Ok(())),
            "Phase 2: borrower with valid KYC must be approved"
        );
        assert_eq!(
            lending.loan_balance(&borrower),
            1_000,
            "Phase 2: loan position must be recorded"
        );

        // Verify the attestation is genuinely valid in TrustLink.
        let kyc_claim = String::from_str(&env, "KYC_PASSED");
        assert!(
            trustlink.has_valid_claim(&borrower, &kyc_claim),
            "Phase 2: has_valid_claim must return true"
        );

        // ------------------------------------------------------------------
        // Phase 3: KYC revoked mid-loan → subsequent borrow denied
        // ------------------------------------------------------------------
        trustlink.revoke_attestation(&issuer, &attestation_id, &Some(String::from_str(&env, "Compliance hold")));

        // TrustLink must immediately reflect the revocation.
        assert!(
            !trustlink.has_valid_claim(&borrower, &kyc_claim),
            "Phase 3: has_valid_claim must return false after revocation"
        );

        let result = lending.try_open_loan(&borrower, &trustlink_id, &2_000, &1_000);
        assert_eq!(
            result,
            Err(Ok(LendingError::KYCRequired)),
            "Phase 3: revoked KYC must cause loan denial"
        );

        // The existing loan position is unchanged — the contract only gates
        // new borrows, it does not auto-liquidate on KYC loss.
        assert_eq!(
            lending.loan_balance(&borrower),
            1_000,
            "Phase 3: existing loan position must be unchanged after revocation"
        );

        // ------------------------------------------------------------------
        // Phase 4: new KYC issued with expiration → expires → borrow denied
        // ------------------------------------------------------------------
        // Advance time and issue a fresh KYC that expires at 20_000.
        env.ledger().with_mut(|li| li.timestamp = 10_000);
        let _new_attestation_id =
            issue_kyc(&env, &trustlink, &issuer, &borrower, Some(20_000));

        // While KYC is valid a new borrow succeeds.
        let result = lending.try_open_loan(&borrower, &trustlink_id, &500, &250);
        assert_eq!(
            result,
            Ok(Ok(())),
            "Phase 4: fresh KYC must allow a new borrow before expiration"
        );

        // Advance past the expiration timestamp.
        env.ledger().with_mut(|li| li.timestamp = 20_001);

        assert!(
            !trustlink.has_valid_claim(&borrower, &kyc_claim),
            "Phase 4: has_valid_claim must return false after expiration"
        );

        let result = lending.try_open_loan(&borrower, &trustlink_id, &500, &250);
        assert_eq!(
            result,
            Err(Ok(LendingError::KYCRequired)),
            "Phase 4: expired KYC must cause loan denial"
        );
    }

    // -------------------------------------------------------------------------
    // Focused individual tests (each isolated, no shared state)
    // -------------------------------------------------------------------------

    #[test]
    fn test_borrower_without_kyc_is_denied() {
        let env = Env::default();
        env.mock_all_auths();

        let (_trustlink, lending, trustlink_id, _admin, _issuer, borrower) = deploy(&env);

        let result = lending.try_open_loan(&borrower, &trustlink_id, &1_000, &500);
        assert_eq!(
            result,
            Err(Ok(LendingError::KYCRequired)),
            "A borrower with no attestation must receive KYCRequired"
        );
    }

    #[test]
    fn test_borrower_with_kyc_is_approved() {
        let env = Env::default();
        env.mock_all_auths();

        let (trustlink, lending, trustlink_id, _admin, issuer, borrower) = deploy(&env);

        env.ledger().with_mut(|li| li.timestamp = 5_000);
        issue_kyc(&env, &trustlink, &issuer, &borrower, None);

        let result = lending.try_open_loan(&borrower, &trustlink_id, &1_000, &500);
        assert_eq!(
            result,
            Ok(Ok(())),
            "A borrower with a valid KYC attestation must be approved"
        );
        assert_eq!(lending.loan_balance(&borrower), 1_000);
    }

    #[test]
    fn test_kyc_revoked_mid_loan_denies_subsequent_borrow() {
        let env = Env::default();
        env.mock_all_auths();

        let (trustlink, lending, trustlink_id, _admin, issuer, borrower) = deploy(&env);

        // Issue KYC and open an initial loan.
        env.ledger().with_mut(|li| li.timestamp = 5_000);
        let attestation_id = issue_kyc(&env, &trustlink, &issuer, &borrower, None);

        lending
            .try_open_loan(&borrower, &trustlink_id, &1_000, &500)
            .expect("initial loan must succeed");

        // Revoke KYC with an explicit reason.
        trustlink.revoke_attestation(
            &issuer,
            &attestation_id,
            &Some(String::from_str(&env, "AML flag")),
        );

        // Verify the attestation record reflects the revocation.
        let att = trustlink.get_attestation(&attestation_id);
        assert!(att.revoked, "attestation must be marked revoked");
        assert_eq!(
            att.revocation_reason,
            Some(String::from_str(&env, "AML flag")),
            "revocation reason must be stored"
        );

        // Subsequent borrow attempt must be denied.
        let result = lending.try_open_loan(&borrower, &trustlink_id, &500, &250);
        assert_eq!(
            result,
            Err(Ok(LendingError::KYCRequired)),
            "revoked KYC must deny subsequent borrows"
        );
    }

    #[test]
    fn test_kyc_expiration_denies_subsequent_borrow() {
        let env = Env::default();
        env.mock_all_auths();

        let (trustlink, lending, trustlink_id, _admin, issuer, borrower) = deploy(&env);

        // Issue KYC that expires at timestamp 15_000.
        env.ledger().with_mut(|li| li.timestamp = 5_000);
        let attestation_id = issue_kyc(&env, &trustlink, &issuer, &borrower, Some(15_000));

        // Loan succeeds while KYC is valid.
        lending
            .try_open_loan(&borrower, &trustlink_id, &1_000, &500)
            .expect("loan must succeed before KYC expiration");

        // Confirm the attestation has the expected expiration.
        let att = trustlink.get_attestation(&attestation_id);
        assert_eq!(att.expiration, Some(15_000));

        // Advance time to exactly the expiration boundary — still valid at
        // timestamp == expiration - 1.
        env.ledger().with_mut(|li| li.timestamp = 14_999);
        let result = lending.try_open_loan(&borrower, &trustlink_id, &100, &50);
        assert_eq!(
            result,
            Ok(Ok(())),
            "loan must still be approved one ledger before expiration"
        );

        // Advance past expiration.
        env.ledger().with_mut(|li| li.timestamp = 15_001);

        let kyc_claim = String::from_str(&env, "KYC_PASSED");
        assert!(
            !trustlink.has_valid_claim(&borrower, &kyc_claim),
            "has_valid_claim must return false after expiration"
        );

        let result = lending.try_open_loan(&borrower, &trustlink_id, &500, &250);
        assert_eq!(
            result,
            Err(Ok(LendingError::KYCRequired)),
            "expired KYC must deny subsequent borrows"
        );
    }

    #[test]
    fn test_insufficient_collateral_is_distinct_from_kyc_failure() {
        // Ensures the two error paths are independent: a borrower with valid
        // KYC but insufficient collateral gets InsufficientCollateral, not
        // KYCRequired.
        let env = Env::default();
        env.mock_all_auths();

        let (trustlink, lending, trustlink_id, _admin, issuer, borrower) = deploy(&env);

        env.ledger().with_mut(|li| li.timestamp = 5_000);
        issue_kyc(&env, &trustlink, &issuer, &borrower, None);

        // collateral (100) < amount / 2 (500) → InsufficientCollateral
        let result = lending.try_open_loan(&borrower, &trustlink_id, &1_000, &100);
        assert_eq!(
            result,
            Err(Ok(LendingError::InsufficientCollateral)),
            "under-collateralised loan must return InsufficientCollateral, not KYCRequired"
        );
    }

    #[test]
    fn test_second_borrower_unaffected_by_first_borrowers_kyc_revocation() {
        // Revocation of one borrower's KYC must not affect another borrower
        // who holds their own independent attestation.
        let env = Env::default();
        env.mock_all_auths();

        let (trustlink, lending, trustlink_id, _admin, issuer, borrower_a) = deploy(&env);
        let borrower_b = Address::generate(&env);

        env.ledger().with_mut(|li| li.timestamp = 5_000);
        let id_a = issue_kyc(&env, &trustlink, &issuer, &borrower_a, None);

        env.ledger().with_mut(|li| li.timestamp = 6_000);
        issue_kyc(&env, &trustlink, &issuer, &borrower_b, None);

        // Both borrowers can open loans.
        lending
            .try_open_loan(&borrower_a, &trustlink_id, &1_000, &500)
            .expect("borrower_a initial loan must succeed");
        lending
            .try_open_loan(&borrower_b, &trustlink_id, &1_000, &500)
            .expect("borrower_b initial loan must succeed");

        // Revoke only borrower_a's KYC.
        trustlink.revoke_attestation(&issuer, &id_a, &None);

        // borrower_a is now denied.
        let result_a = lending.try_open_loan(&borrower_a, &trustlink_id, &500, &250);
        assert_eq!(
            result_a,
            Err(Ok(LendingError::KYCRequired)),
            "borrower_a must be denied after their KYC is revoked"
        );

        // borrower_b is still approved — their attestation is independent.
        let result_b = lending.try_open_loan(&borrower_b, &trustlink_id, &500, &250);
        assert_eq!(
            result_b,
            Ok(Ok(())),
            "borrower_b must remain approved; their KYC was not revoked"
        );
    }
}
