#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, Address, Env, String};
use trustlink::{TrustLinkContract, TrustLinkContractClient};

const MERCHANT_VERIFIED: &str = "MERCHANT_VERIFIED";
const FINANCIAL_INSTITUTION_LICENSED: &str = "FINANCIAL_INSTITUTION_LICENSED";

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TradeFinanceError {
    InvalidAmount = 1,
    MerchantNotVerified = 2,
    FinancialInstitutionNotLicensed = 3,
}

#[contract]
pub struct TradeFinanceContract;

/// Trade finance example: Gate trade credit issuance on merchant and financial institution verification
#[contractimpl]
impl TradeFinanceContract {
    /// Issue trade credit to a verified merchant from a verified financial institution
    pub fn issue_trade_credit(
        env: Env,
        merchant: Address,
        financial_institution: Address,
        trustlink_id: Address,
        amount: i128,
    ) -> Result<bool, TradeFinanceError> {
        merchant.require_auth();

        if amount <= 0 {
            return Err(TradeFinanceError::InvalidAmount);
        }

        let trustlink = TrustLinkContractClient::new(&env, &trustlink_id);
        let merchant_verified = trustlink.has_valid_claim(
            &merchant,
            &String::from_str(&env, MERCHANT_VERIFIED),
        );
        let institution_licensed = trustlink.has_valid_claim(
            &financial_institution,
            &String::from_str(&env, FINANCIAL_INSTITUTION_LICENSED),
        );

        if !merchant_verified {
            return Err(TradeFinanceError::MerchantNotVerified);
        }
        if !institution_licensed {
            return Err(TradeFinanceError::FinancialInstitutionNotLicensed);
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_trade_credit_issuance() {
        let env = Env::default();
        env.mock_all_auths();

        let trustlink_id = env.register_contract(None, TrustLinkContract);
        let trustlink = TrustLinkContractClient::new(&env, &trustlink_id);
        let trade_finance_id = env.register_contract(None, TradeFinanceContract);
        let trade_finance = TradeFinanceContractClient::new(&env, &trade_finance_id);

        let admin = Address::generate(&env);
        let merchant_issuer = Address::generate(&env);
        let regulator = Address::generate(&env);
        let merchant = Address::generate(&env);
        let financial_institution = Address::generate(&env);

        trustlink.initialize(&admin, &None).unwrap();
        trustlink.register_issuer(&admin, &merchant_issuer).unwrap();
        trustlink.register_issuer(&admin, &regulator).unwrap();

        // Non-positive credit amounts are rejected even when credentials are absent.
        assert_eq!(
            trade_finance.try_issue_trade_credit(
                &merchant,
                &financial_institution,
                &trustlink_id,
                &0,
            ),
            Ok(Err(TradeFinanceError::InvalidAmount)),
        );

        // Neither attestation is present, so issuance is denied.
        assert!(matches!(
            trade_finance.try_issue_trade_credit(
                &merchant,
                &financial_institution,
                &trustlink_id,
                &100,
            ),
            Ok(Err(TradeFinanceError::MerchantNotVerified))
        ));

        trustlink
            .create_attestation(
                &merchant_issuer,
                &merchant,
                &String::from_str(&env, MERCHANT_VERIFIED),
                &None,
                &None,
                &None,
                &None,
            )
            .unwrap();

        // Merchant verification alone is insufficient.
        assert!(matches!(
            trade_finance.try_issue_trade_credit(
                &merchant,
                &financial_institution,
                &trustlink_id,
                &100,
            ),
            Ok(Err(TradeFinanceError::FinancialInstitutionNotLicensed))
        ));

        let institution_attestation = trustlink
            .create_attestation(
                &regulator,
                &financial_institution,
                &String::from_str(&env, FINANCIAL_INSTITUTION_LICENSED),
                &None,
                &None,
                &None,
                &None,
            )
            .unwrap();

        // Issuance succeeds only once both subjects have their valid claims.
        assert_eq!(
            trade_finance.try_issue_trade_credit(
                &merchant,
                &financial_institution,
                &trustlink_id,
                &100,
            ),
            Ok(Ok(true)),
        );

        trustlink
            .revoke_attestation(&regulator, &institution_attestation, &None)
            .unwrap();
        assert!(matches!(
            trade_finance.try_issue_trade_credit(
                &merchant,
                &financial_institution,
                &trustlink_id,
                &100,
            ),
            Ok(Err(TradeFinanceError::FinancialInstitutionNotLicensed))
        ));
    }
}
