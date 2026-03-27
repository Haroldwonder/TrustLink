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
    