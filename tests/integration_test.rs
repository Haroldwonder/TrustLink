//! Integration test demonstrating cross-contract verification
//! 
//! This shows how another contract would use TrustLink to verify attestations

#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl, contracterror, contracttype,
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

// Import from the library crate
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

/// Example lending contract that requires KYC verification via TrustLink
#[contract]
pub struct LendingContract;

#[contractimpl]
impl LendingContract {
    /// Request a loan - requires valid KYC attestation from TrustLink
    pub fn request_loan(
        env: Env,
        borrower: Address,
        trustlink_contract: Address,
        amount: i128,
        collateral: i128,
    ) -> Result<(), LendingError> {
        borrower.require_auth();
        
        // Create TrustLink client
        let trustlink = TrustLinkContractClient::new(&env, &trustlink_contract);
        
        // Verify borrower has valid KYC
        let kyc_claim = String::from_str(&env, "KYC_PASSED");
        let has_kyc = trustlink.has_valid_claim(&borrower, &kyc_claim);
        
        if !has_kyc {
            return Err(LendingError::KYCRequired);
        }
        
        // Verify sufficient collateral (simplified)
        if collateral < amount / 2 {
            return Err(LendingError::InsufficientCollateral);
        }
        
        // Store loan request
        let loan = LoanRequest {
            borrower: borrower.clone(),
            amount,
            collateral,
        };
        
        env.storage().instance().set(&borrower, &loan);
        
        // Emit event
        env.events().publish(
            (soroban_sdk::symbol_short!("loan_req"), borrower),
            (amount, collateral),
        );
        
        Ok(())
    }
    
    /// Check if address can borrow (has valid KYC)
    pub fn can_borrow(
        env: Env,
        address: Address,
        trustlink_contract: Address,
    ) -> bool {
        let trustlink = TrustLinkContractClient::new(&env, &trustlink_contract);
        let kyc_claim = String::from_str(&env, "KYC_PASSED");
        trustlink.has_valid_claim(&address, &kyc_claim)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;
    
    #[test]
    fn test_cross_contract_kyc_verification() {
        let env = Env::default();
        env.mock_all_auths();
        
        // Deploy TrustLink
        let trustlink_id = env.register_contract(None, TrustLinkContract);
        let trustlink = TrustLinkContractClient::new(&env, &trustlink_id);
        
        // Deploy Lending contract
        let lending_id = env.register_contract(None, LendingContract);
        let lending = LendingContractClient::new(&env, &lending_id);
        
        // Setup: Initialize TrustLink
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let borrower = Address::generate(&env);
        
        trustlink.initialize(&admin);
        trustlink.register_issuer(&admin, &issuer);
        
        // Test 1: Loan request without KYC should fail
        let result = lending.try_request_loan(
            &borrower,
            &trustlink_id,
            &1000,
            &500,
        );
        assert!(result.is_err());
        
        // Test 2: Issue KYC attestation
        let kyc_claim = String::from_str(&env, "KYC_PASSED");
        trustlink.create_attestation(&issuer, &borrower, &kyc_claim, &None, &None);
        
        // Test 3: Loan request with KYC should succeed
        let result = lending.try_request_loan(
            &borrower,
            &trustlink_id,
            &1000,
            &500,
        );
        assert!(result.is_ok());
        
        // Test 4: Check borrowing eligibility
        let can_borrow = lending.can_borrow(&borrower, &trustlink_id);
        assert!(can_borrow);
        
        // Test 5: Revoke KYC
        let attestation_ids = trustlink.get_subject_attestations(&borrower, &0, &10);
        let attestation_id = attestation_ids.get(0).unwrap();
        trustlink.revoke_attestation(&issuer, &attestation_id);
        
        // Test 6: After revocation, borrowing should be denied
        let can_borrow = lending.can_borrow(&borrower, &trustlink_id);
        assert!(!can_borrow);
    }

    #[test]
    fn test_time_locked_attestation_cross_contract() {
        let env = Env::default();
        env.mock_all_auths();

        // Set a known starting ledger time
        let start_time: u64 = 1_000;
        env.ledger().with_mut(|l| l.timestamp = start_time);

        // Deploy TrustLink
        let trustlink_id = env.register_contract(None, TrustLinkContract);
        let trustlink = TrustLinkContractClient::new(&env, &trustlink_id);

        // Setup
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);

        trustlink.initialize(&admin);
        trustlink.register_issuer(&admin, &issuer);

        // Create a time-locked attestation with a future valid_from
        let claim_type = String::from_str(&env, "ACCREDITED_INVESTOR");
        let valid_from = start_time + 500;
        let attestation_id =
            trustlink.create_attestation(&issuer, &subject, &claim_type, &None, &Some(valid_from));

        // Assert status is Pending and has_valid_claim returns false
        let status = trustlink.get_attestation_status(&attestation_id);
        assert_eq!(status, trustlink::types::AttestationStatus::Pending);
        assert!(!trustlink.has_valid_claim(&subject, &claim_type));

        // Advance ledger time past valid_from
        env.ledger().with_mut(|l| l.timestamp = valid_from + 1);

        // Assert status is Valid and has_valid_claim returns true
        let status = trustlink.get_attestation_status(&attestation_id);
        assert_eq!(status, trustlink::types::AttestationStatus::Valid);
        assert!(trustlink.has_valid_claim(&subject, &claim_type));
    }
}
