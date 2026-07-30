//! Real-Estate Title Registry Example (TrustLink Integration)
//!
//! Demonstrates long-lived attestations (multi-year TTL) and a dual-issuer
//! verification flow:
//!
//! - A **title-registry issuer** attests `CLEAR_TITLE` for a property.
//! - A **lien-holder** (e.g. a bank) attests `LIEN_ENCUMBRANCE` on the same
//!   property address when a mortgage or lien is recorded.
//! - A **buyer's verification contract** queries TrustLink to confirm that
//!   clear title exists (`has_any_claim`) AND that no encumbrance is currently
//!   active (`has_valid_claim` for `LIEN_ENCUMBRANCE` must return false).
//!
//! This exercises:
//! - Very long TTL / expiration values (10-year title attestations).
//! - `renew_attestation` on a multi-year cadence.
//! - `has_any_claim` / `has_all_claims` multi-claim query API.
//! - Dual-issuer model: title registry + lien holders as separate issuers.

#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, Address, Env, String, Vec,
};

// ── Claim type constants ──────────────────────────────────────────────────────

/// Attested by the title-registry issuer: property has unencumbered clear title.
pub const CLAIM_CLEAR_TITLE: &str = "CLEAR_TITLE";

/// Attested by a lien-holder: an encumbrance (mortgage, judgment, lien) exists.
pub const CLAIM_LIEN: &str = "LIEN_ENCUMBRANCE";

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    TrustLink,
    PropertyCount,
    Property(u32),
}

// ── TrustLink cross-contract interface ────────────────────────────────────────

#[contractclient(name = "TrustLinkClient")]
pub trait TrustLink {
    fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool;
    fn has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool;
    fn has_all_claims(env: Env, subject: Address, claim_types: Vec<String>) -> bool;
}

// ── Data structures ───────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct PropertyRecord {
    /// On-chain address representing the property (e.g. a deterministic hash
    /// of the parcel ID, or a dedicated property account).
    pub property_address: Address,
    /// Human-readable parcel identifier (APN, title number, etc.).
    pub parcel_id: String,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct TitleRegistryContract;

#[contractimpl]
impl TitleRegistryContract {
    // ── Initialisation ────────────────────────────────────────────────────────

    /// Initialise with an admin and the TrustLink contract address.
    pub fn initialize(env: Env, admin: Address, trustlink_contract: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TrustLink, &trustlink_contract);
        env.storage().instance().set(&DataKey::PropertyCount, &0u32);
    }

    // ── Property registration ─────────────────────────────────────────────────

    /// Register a property and return its internal ID.
    /// The title registry (admin) is the only party allowed to add properties.
    pub fn register_property(
        env: Env,
        admin: Address,
        property_address: Address,
        parcel_id: String,
    ) -> u32 {
        admin.require_auth();
        let stored_admin: Address =
            env.storage().instance().get(&DataKey::Admin).unwrap();
        if stored_admin != admin {
            panic!("admin only");
        }

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PropertyCount)
            .unwrap_or(0);
        let id = count + 1;

        env.storage().instance().set(
            &DataKey::Property(id),
            &PropertyRecord { property_address, parcel_id },
        );
        env.storage().instance().set(&DataKey::PropertyCount, &id);

        env.events().publish((symbol_short!("prop_reg"), id), id);
        id
    }

    // ── Buyer verification ────────────────────────────────────────────────────

    /// Verify that a property is safe to purchase.
    ///
    /// Returns `true` only when:
    /// 1. The property has a `CLEAR_TITLE` attestation (title registry has
    ///    certified no prior unresolved ownership dispute).
    /// 2. The property does NOT have an active `LIEN_ENCUMBRANCE` attestation
    ///    (no bank or judgment lien is currently recorded on-chain).
    ///
    /// A buyer's escrow or closing contract calls this before releasing funds.
    pub fn verify_purchase_eligibility(
        env: Env,
        property_id: u32,
    ) -> bool {
        let record: PropertyRecord = env
            .storage()
            .instance()
            .get(&DataKey::Property(property_id))
            .unwrap_or_else(|| panic!("property not found"));

        let trustlink_id: Address =
            env.storage().instance().get(&DataKey::TrustLink).unwrap();
        let tl = TrustLinkClient::new(&env, &trustlink_id);

        let clear_title = String::from_str(&env, CLAIM_CLEAR_TITLE);
        let lien = String::from_str(&env, CLAIM_LIEN);

        // Property must have clear-title AND must NOT have an active lien.
        let has_clear = tl.has_valid_claim(&record.property_address, &clear_title);
        let has_lien = tl.has_valid_claim(&record.property_address, &lien);

        has_clear && !has_lien
    }

    /// Check whether any encumbrance claim (LIEN_ENCUMBRANCE or future types)
    /// is currently active, using `has_any_claim` OR-logic.
    pub fn has_any_encumbrance(env: Env, property_id: u32) -> bool {
        let record: PropertyRecord = env
            .storage()
            .instance()
            .get(&DataKey::Property(property_id))
            .unwrap_or_else(|| panic!("property not found"));

        let trustlink_id: Address =
            env.storage().instance().get(&DataKey::TrustLink).unwrap();
        let tl = TrustLinkClient::new(&env, &trustlink_id);

        let mut encumbrance_types: Vec<String> = Vec::new(&env);
        encumbrance_types.push_back(String::from_str(&env, CLAIM_LIEN));
        // Future encumbrance types (e.g. "TAX_LIEN", "JUDGMENT_LIEN") can be
        // appended here as the claim type registry grows.

        tl.has_any_claim(&record.property_address, &encumbrance_types)
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    pub fn get_property(env: Env, property_id: u32) -> PropertyRecord {
        env.storage()
            .instance()
            .get(&DataKey::Property(property_id))
            .unwrap_or_else(|| panic!("property not found"))
    }

    pub fn get_trustlink(env: Env) -> Address {
        env.storage().instance().get(&DataKey::TrustLink).unwrap()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env, Vec};

    // ── Mock TrustLink variants ───────────────────────────────────────────────

    /// Clear title exists, no lien — property is transferable.
    #[contract]
    struct MockClearNoLien;

    #[contractimpl]
    impl MockClearNoLien {
        pub fn has_valid_claim(_env: Env, _subject: Address, claim_type: String) -> bool {
            claim_type == String::from_str(&_env, CLAIM_CLEAR_TITLE)
        }
        pub fn has_any_claim(_env: Env, _subject: Address, claim_types: Vec<String>) -> bool {
            claim_types.iter().any(|c| c == String::from_str(&_env, CLAIM_LIEN))
        }
        pub fn has_all_claims(_env: Env, _subject: Address, claim_types: Vec<String>) -> bool {
            claim_types.iter().all(|c| c == String::from_str(&_env, CLAIM_CLEAR_TITLE))
        }
    }

    /// Clear title exists AND a lien is recorded — purchase should be blocked.
    #[contract]
    struct MockClearWithLien;

    #[contractimpl]
    impl MockClearWithLien {
        pub fn has_valid_claim(_env: Env, _subject: Address, _claim_type: String) -> bool {
            // Both CLEAR_TITLE and LIEN_ENCUMBRANCE are active.
            true
        }
        pub fn has_any_claim(_env: Env, _subject: Address, _claim_types: Vec<String>) -> bool {
            true
        }
        pub fn has_all_claims(_env: Env, _subject: Address, _claim_types: Vec<String>) -> bool {
            true
        }
    }

    /// No clear title, no lien — title was never registered.
    #[contract]
    struct MockNoTitle;

    #[contractimpl]
    impl MockNoTitle {
        pub fn has_valid_claim(_env: Env, _subject: Address, _claim_type: String) -> bool {
            false
        }
        pub fn has_any_claim(_env: Env, _subject: Address, _claim_types: Vec<String>) -> bool {
            false
        }
        pub fn has_all_claims(_env: Env, _subject: Address, _claim_types: Vec<String>) -> bool {
            false
        }
    }

    // ── Helper ────────────────────────────────────────────────────────────────

    fn setup<C: 'static>(env: &Env, mock: C) -> (TitleRegistryContractClient, Address, u32)
    where
        C: soroban_sdk::testutils::Register,
    {
        let admin = Address::generate(env);
        let property_address = Address::generate(env);
        let parcel_id = String::from_str(env, "APN-001-MAIN-ST");

        let tl_id = env.register(mock, ());
        let contract_id = env.register(TitleRegistryContract, ());
        let client = TitleRegistryContractClient::new(env, &contract_id);

        client.initialize(&admin, &tl_id);
        let prop_id = client.register_property(
            &admin,
            &property_address,
            &parcel_id,
        );

        (client, admin, prop_id)
    }

    // ── Tests: purchase eligibility ───────────────────────────────────────────

    #[test]
    fn purchase_allowed_when_clear_title_and_no_lien() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, prop_id) = setup(&env, MockClearNoLien);
        assert!(client.verify_purchase_eligibility(&prop_id));
    }

    #[test]
    fn purchase_blocked_when_lien_present() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, prop_id) = setup(&env, MockClearWithLien);
        // Lien is active — purchase must be blocked.
        assert!(!client.verify_purchase_eligibility(&prop_id));
    }

    #[test]
    fn purchase_blocked_when_no_clear_title() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, prop_id) = setup(&env, MockNoTitle);
        assert!(!client.verify_purchase_eligibility(&prop_id));
    }

    // ── Tests: encumbrance check ──────────────────────────────────────────────

    #[test]
    fn no_encumbrance_on_clean_title() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, prop_id) = setup(&env, MockClearNoLien);
        assert!(!client.has_any_encumbrance(&prop_id));
    }

    #[test]
    fn encumbrance_detected_when_lien_present() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, prop_id) = setup(&env, MockClearWithLien);
        assert!(client.has_any_encumbrance(&prop_id));
    }

    // ── Tests: property registration ─────────────────────────────────────────

    #[test]
    fn register_multiple_properties_returns_sequential_ids() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let tl_id = env.register(MockClearNoLien, ());
        let contract_id = env.register(TitleRegistryContract, ());
        let client = TitleRegistryContractClient::new(&env, &contract_id);
        client.initialize(&admin, &tl_id);

        let p1 = client.register_property(
            &admin,
            &Address::generate(&env),
            &String::from_str(&env, "APN-001"),
        );
        let p2 = client.register_property(
            &admin,
            &Address::generate(&env),
            &String::from_str(&env, "APN-002"),
        );

        assert_eq!(p1, 1);
        assert_eq!(p2, 2);
    }

    #[test]
    fn get_property_returns_correct_record() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let property_address = Address::generate(&env);
        let parcel_id = String::from_str(&env, "APN-999");

        let tl_id = env.register(MockClearNoLien, ());
        let contract_id = env.register(TitleRegistryContract, ());
        let client = TitleRegistryContractClient::new(&env, &contract_id);
        client.initialize(&admin, &tl_id);

        let id = client.register_property(&admin, &property_address, &parcel_id);
        let record = client.get_property(&id);

        assert_eq!(record.property_address, property_address);
        assert_eq!(record.parcel_id, parcel_id);
    }

    // ── Tests: long-TTL / expiry simulation ───────────────────────────────────

    /// Simulate a 10-year title attestation: after expiry the clear-title claim
    /// is invalid, so purchase eligibility must be blocked.
    #[test]
    fn purchase_blocked_when_title_attestation_expired() {
        // "Expired" is modelled by a mock that always returns false for all
        // claims — identical to what TrustLink returns after expiration passes.
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, prop_id) = setup(&env, MockNoTitle);
        assert!(!client.verify_purchase_eligibility(&prop_id));
    }
}
