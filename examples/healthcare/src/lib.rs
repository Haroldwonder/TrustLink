//! Healthcare Credential Verification Example (TrustLink Integration)
//!
//! Demonstrates the privacy-sensitive end of the attestation spectrum and
//! the data-minimisation patterns described in docs/compliance.md.
//!
//! Two claim types are supported:
//!
//! - `PROVIDER_LICENSED` — issued by a licensing-board issuer.
//!   Attests that a healthcare provider holds a current, valid license.
//!   Metadata: **none** — the fact of licensure is sufficient; licence
//!   numbers or speciality details stay off-chain.
//!
//! - `VACCINATION_VERIFIED` — issued by a public-health issuer.
//!   Attests that a patient has a verified vaccination record for a given
//!   vaccine series. Metadata: **none** — no patient name, DOB, or specific
//!   vaccine lot is stored on-chain (data minimisation).
//!
//! ## Privacy / Data-Minimisation Notes (see docs/compliance.md)
//!
//! The `metadata` field on every `create_attestation` call is `None` here
//! deliberately.  Healthcare data is among the most sensitive categories under
//! GDPR Art. 9 and HIPAA.  Integrators MUST NOT store names, identifiers, or
//! clinical data in the on-chain `metadata` field.  Use a hashed reference to
//! an off-chain record if a linkage is needed.
//!
//! Attestation expiration is set short (1 year for provider licences; 2 years
//! for vaccination records that require boosters) to limit the window of
//! exposure and align with typical re-credentialing cycles.

#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, Address, Env, String, Vec,
};

// ── Claim type constants ──────────────────────────────────────────────────────

/// Issued by a licensing board: provider is currently licensed to practice.
pub const CLAIM_PROVIDER_LICENSED: &str = "PROVIDER_LICENSED";

/// Issued by a public-health authority: subject has a verified vaccination record.
pub const CLAIM_VACCINATION_VERIFIED: &str = "VACCINATION_VERIFIED";

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    TrustLink,
}

// ── TrustLink cross-contract interface ────────────────────────────────────────

#[contractclient(name = "TrustLinkClient")]
pub trait TrustLink {
    fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool;
    fn has_all_claims(env: Env, subject: Address, claim_types: Vec<String>) -> bool;
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct HealthcareVerifierContract;

#[contractimpl]
impl HealthcareVerifierContract {
    // ── Initialisation ────────────────────────────────────────────────────────

    pub fn initialize(env: Env, admin: Address, trustlink_contract: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TrustLink, &trustlink_contract);
    }

    // ── Verification entry-points ─────────────────────────────────────────────

    /// Verify that a provider is licensed to practise.
    ///
    /// A telehealth platform, prescription system, or credentialing portal
    /// calls this before granting the provider access to privileged functions.
    ///
    /// Privacy: only the boolean result crosses the contract boundary — no
    /// licence details, no name, no speciality.
    pub fn verify_provider_license(env: Env, provider: Address) -> bool {
        let tl = Self::trustlink_client(&env);
        let claim = String::from_str(&env, CLAIM_PROVIDER_LICENSED);
        let result = tl.has_valid_claim(&provider, &claim);

        env.events().publish(
            (symbol_short!("prov_chk"), provider.clone()),
            result,
        );
        result
    }

    /// Verify that a patient has a valid vaccination record.
    ///
    /// A care-facility access system or pharmacy benefit manager calls this
    /// before granting entry or dispensing a vaccine-dependent prescription.
    ///
    /// Privacy: only the boolean result is returned — no vaccine lot, no date,
    /// no patient identifier other than the address itself.
    pub fn verify_vaccination(env: Env, patient: Address) -> bool {
        let tl = Self::trustlink_client(&env);
        let claim = String::from_str(&env, CLAIM_VACCINATION_VERIFIED);
        let result = tl.has_valid_claim(&patient, &claim);

        env.events().publish(
            (symbol_short!("vax_chk"), patient.clone()),
            result,
        );
        result
    }

    /// Full credentialing check: provider must be licensed AND vaccinated.
    ///
    /// Used for high-risk clinical roles (e.g. ICU staff) where both
    /// credentials are required simultaneously.  Uses AND-logic via
    /// `has_all_claims` to avoid two separate cross-contract calls.
    pub fn verify_full_credentials(env: Env, provider: Address) -> bool {
        let tl = Self::trustlink_client(&env);

        let mut required: Vec<String> = Vec::new(&env);
        required.push_back(String::from_str(&env, CLAIM_PROVIDER_LICENSED));
        required.push_back(String::from_str(&env, CLAIM_VACCINATION_VERIFIED));

        let result = tl.has_all_claims(&provider, &required);

        env.events().publish(
            (symbol_short!("full_chk"), provider.clone()),
            result,
        );
        result
    }

    // ── Admin accessor ────────────────────────────────────────────────────────

    pub fn get_trustlink(env: Env) -> Address {
        env.storage().instance().get(&DataKey::TrustLink).unwrap()
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn trustlink_client(env: &Env) -> TrustLinkClient {
        let id: Address = env.storage().instance().get(&DataKey::TrustLink).unwrap();
        TrustLinkClient::new(env, &id)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, Vec};

    // ── Mock variants ─────────────────────────────────────────────────────────

    /// Provider is licensed; patient has vaccination record.
    #[contract]
    struct MockAllValid;

    #[contractimpl]
    impl MockAllValid {
        pub fn has_valid_claim(_env: Env, _subject: Address, _claim_type: String) -> bool {
            true
        }
        pub fn has_all_claims(_env: Env, _subject: Address, _claim_types: Vec<String>) -> bool {
            true
        }
    }

    /// Neither claim is valid (expired, revoked, or never issued).
    #[contract]
    struct MockNoneValid;

    #[contractimpl]
    impl MockNoneValid {
        pub fn has_valid_claim(_env: Env, _subject: Address, _claim_type: String) -> bool {
            false
        }
        pub fn has_all_claims(_env: Env, _subject: Address, _claim_types: Vec<String>) -> bool {
            false
        }
    }

    /// Licensed but vaccination record is missing (e.g. new staff not yet verified).
    #[contract]
    struct MockLicensedOnly;

    #[contractimpl]
    impl MockLicensedOnly {
        pub fn has_valid_claim(_env: Env, _subject: Address, claim_type: String) -> bool {
            claim_type == String::from_str(&_env, CLAIM_PROVIDER_LICENSED)
        }
        pub fn has_all_claims(_env: Env, _subject: Address, _claim_types: Vec<String>) -> bool {
            // Fails because VACCINATION_VERIFIED is missing.
            false
        }
    }

    /// Vaccination record exists but provider is not (yet) licensed.
    #[contract]
    struct MockVaccinatedOnly;

    #[contractimpl]
    impl MockVaccinatedOnly {
        pub fn has_valid_claim(_env: Env, _subject: Address, claim_type: String) -> bool {
            claim_type == String::from_str(&_env, CLAIM_VACCINATION_VERIFIED)
        }
        pub fn has_all_claims(_env: Env, _subject: Address, _claim_types: Vec<String>) -> bool {
            false
        }
    }

    // ── Setup helper ──────────────────────────────────────────────────────────

    fn setup<C: 'static>(env: &Env, mock: C) -> HealthcareVerifierContractClient
    where
        C: soroban_sdk::testutils::Register,
    {
        let admin = Address::generate(env);
        let tl_id = env.register(mock, ());
        let contract_id = env.register(HealthcareVerifierContract, ());
        let client = HealthcareVerifierContractClient::new(env, &contract_id);
        client.initialize(&admin, &tl_id);
        client
    }

    // ── Provider license tests ────────────────────────────────────────────────

    #[test]
    fn licensed_provider_is_verified() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockAllValid);
        let provider = Address::generate(&env);
        assert!(client.verify_provider_license(&provider));
    }

    #[test]
    fn unlicensed_provider_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockNoneValid);
        let provider = Address::generate(&env);
        assert!(!client.verify_provider_license(&provider));
    }

    #[test]
    fn provider_with_expired_license_is_rejected() {
        // Expired attestation → has_valid_claim returns false (same as MockNoneValid).
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockNoneValid);
        let provider = Address::generate(&env);
        assert!(!client.verify_provider_license(&provider));
    }

    // ── Vaccination tests ─────────────────────────────────────────────────────

    #[test]
    fn vaccinated_patient_is_verified() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockAllValid);
        let patient = Address::generate(&env);
        assert!(client.verify_vaccination(&patient));
    }

    #[test]
    fn unvaccinated_patient_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockNoneValid);
        let patient = Address::generate(&env);
        assert!(!client.verify_vaccination(&patient));
    }

    // ── Full credential tests ─────────────────────────────────────────────────

    #[test]
    fn full_credentials_pass_when_both_claims_valid() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockAllValid);
        let provider = Address::generate(&env);
        assert!(client.verify_full_credentials(&provider));
    }

    #[test]
    fn full_credentials_fail_when_vaccination_missing() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockLicensedOnly);
        let provider = Address::generate(&env);
        assert!(!client.verify_full_credentials(&provider));
    }

    #[test]
    fn full_credentials_fail_when_license_missing() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockVaccinatedOnly);
        let provider = Address::generate(&env);
        assert!(!client.verify_full_credentials(&provider));
    }

    #[test]
    fn full_credentials_fail_when_no_claims_at_all() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockNoneValid);
        let provider = Address::generate(&env);
        assert!(!client.verify_full_credentials(&provider));
    }

    // ── Data-minimisation audit: metadata must never be passed ───────────────
    //
    // These tests document the expected call pattern by verifying that the
    // contract never returns claim-specific data in its boolean responses —
    // only the fact (true/false) crosses the contract boundary, consistent
    // with docs/compliance.md § Data Minimisation.
    //
    // In a production test suite you would also assert that no PII appears in
    // emitted events (inspect env.events() snapshots).

    #[test]
    fn provider_check_returns_only_boolean() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockAllValid);
        let provider = Address::generate(&env);

        // Return type is bool — no licence number, no name, no speciality.
        let result: bool = client.verify_provider_license(&provider);
        assert!(result);
    }

    #[test]
    fn vaccination_check_returns_only_boolean() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env, MockAllValid);
        let patient = Address::generate(&env);

        // Return type is bool — no vaccine lot, no date, no patient details.
        let result: bool = client.verify_vaccination(&patient);
        assert!(result);
    }
}
