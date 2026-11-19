//! Multi-tenant SaaS Seat-License Verification Example (TrustLink Integration)
//!
//! Demonstrates the B2B commercial pattern where a SaaS company (tenant / org)
//! issues per-seat `SEAT_LICENSED` attestations to employee wallet addresses,
//! a resource contract gates feature access with `has_valid_claim`, and the
//! company admin revokes the seat attestation when an employee is offboarded.
//!
//! ## Lifecycle
//!
//! 1. **Issuance** — Org issuer calls TrustLink `create_attestation` with
//!    claim type `SEAT_LICENSED` for the employee's address.
//! 2. **Access gate** — The SaaS resource contract calls
//!    `has_valid_claim` / `has_valid_claim_from_issuer` before granting access.
//! 3. **Offboarding** — Org issuer calls TrustLink `revoke_attestation`;
//!    subsequent access checks fail because the claim is no longer valid.
//!
//! Multi-tenancy: each SaaS customer is a distinct TrustLink issuer. The gated
//! resource stores its org issuer and prefers issuer-scoped checks so seats
//! from another tenant are not accepted.

#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, Address, Env, String,
};

// ── Claim type constants ──────────────────────────────────────────────────────

/// Issued by a SaaS organization: employee holds an active licensed seat.
pub const CLAIM_SEAT_LICENSED: &str = "SEAT_LICENSED";

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    TrustLink,
    /// The organization (tenant) whose seat attestations are accepted.
    OrgIssuer,
    AccessCount,
}

// ── TrustLink cross-contract interface ────────────────────────────────────────

#[contractclient(name = "TrustLinkClient")]
pub trait TrustLink {
    fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool;
    fn has_valid_claim_from_issuer(
        env: Env,
        subject: Address,
        claim_type: String,
        issuer: Address,
    ) -> bool;
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct SeatLicensedResource;

#[contractimpl]
impl SeatLicensedResource {
    /// Initialise with an admin, TrustLink contract, and the org's issuer address.
    pub fn initialize(
        env: Env,
        admin: Address,
        trustlink_contract: Address,
        org_issuer: Address,
    ) {
        // Auth first, before any storage read (FINDING-001 pattern).
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
            .set(&DataKey::OrgIssuer, &org_issuer);
        env.storage().instance().set(&DataKey::AccessCount, &0u32);
    }

    /// Return whether `employee` currently holds a valid seat from this org.
    ///
    /// Uses issuer-scoped verification so seats issued by other tenants are
    /// rejected — the multi-tenant isolation property.
    pub fn has_active_seat(env: Env, employee: Address) -> bool {
        let tl = Self::trustlink_client(&env);
        let org_issuer: Address = env.storage().instance().get(&DataKey::OrgIssuer).unwrap();
        let claim = String::from_str(&env, CLAIM_SEAT_LICENSED);
        tl.has_valid_claim_from_issuer(&employee, &claim, &org_issuer)
    }

    /// Gate a premium SaaS feature behind a valid `SEAT_LICENSED` claim.
    ///
    /// Panics if the employee has no active seat from this organization
    /// (never issued, expired, or revoked during offboarding).
    pub fn access_premium_feature(env: Env, employee: Address) -> u32 {
        employee.require_auth();

        if !Self::has_active_seat(env.clone(), employee.clone()) {
            panic!("active SEAT_LICENSED claim required");
        }

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::AccessCount)
            .unwrap_or(0);
        let next = count + 1;
        env.storage().instance().set(&DataKey::AccessCount, &next);

        env.events()
            .publish((symbol_short!("seat_ok"), employee), next);
        next
    }

    /// Convenience check using the global `has_valid_claim` API (any issuer).
    ///
    /// Prefer `has_active_seat` in production multi-tenant deployments so one
    /// customer's seats cannot unlock another customer's resources.
    pub fn has_any_seat_claim(env: Env, employee: Address) -> bool {
        let tl = Self::trustlink_client(&env);
        let claim = String::from_str(&env, CLAIM_SEAT_LICENSED);
        tl.has_valid_claim(&employee, &claim)
    }

    pub fn get_org_issuer(env: Env) -> Address {
        env.storage().instance().get(&DataKey::OrgIssuer).unwrap()
    }

    pub fn get_trustlink(env: Env) -> Address {
        env.storage().instance().get(&DataKey::TrustLink).unwrap()
    }

    pub fn get_access_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::AccessCount)
            .unwrap_or(0)
    }

    pub fn set_org_issuer(env: Env, admin: Address, org_issuer: Address) {
        admin.require_auth();
        let current: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if current != admin {
            panic!("admin only");
        }
        env.storage()
            .instance()
            .set(&DataKey::OrgIssuer, &org_issuer);
    }

    fn trustlink_client(env: &Env) -> TrustLinkClient<'_> {
        let id: Address = env.storage().instance().get(&DataKey::TrustLink).unwrap();
        TrustLinkClient::new(env, &id)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        contract, contractimpl, contracttype, symbol_short, testutils::Address as _, Address, Env,
        String,
    };

    // ── Stateful mock TrustLink (issue / check / revoke) ──────────────────────

    #[contracttype]
    enum MockKey {
        /// Maps (subject, claim_type) → issuer that granted the seat.
        Seat(Address, String),
    }

    /// Mock that can issue and revoke `SEAT_LICENSED` attestations so tests
    /// exercise the full onboarding → access → offboarding lifecycle.
    #[contract]
    struct MockTrustLink;

    #[contractimpl]
    impl MockTrustLink {
        /// Simulate org issuer calling TrustLink `create_attestation`
        /// with claim type `SEAT_LICENSED`.
        pub fn issue_seat(env: Env, issuer: Address, employee: Address) {
            issuer.require_auth();
            let claim = String::from_str(&env, CLAIM_SEAT_LICENSED);
            env.storage()
                .persistent()
                .set(&MockKey::Seat(employee.clone(), claim), &issuer);
            env.events()
                .publish((symbol_short!("issued"), employee), issuer);
        }

        /// Simulate org issuer calling TrustLink `revoke_attestation`
        /// during employee offboarding.
        pub fn revoke_seat(env: Env, issuer: Address, employee: Address) {
            issuer.require_auth();
            let claim = String::from_str(&env, CLAIM_SEAT_LICENSED);
            let key = MockKey::Seat(employee.clone(), claim);
            let current: Option<Address> = env.storage().persistent().get(&key);
            match current {
                Some(granted_by) if granted_by == issuer => {
                    env.storage().persistent().remove(&key);
                    env.events()
                        .publish((symbol_short!("revoked"), employee), issuer);
                }
                _ => panic!("no seat from this issuer to revoke"),
            }
        }

        pub fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool {
            env.storage()
                .persistent()
                .has(&MockKey::Seat(subject, claim_type))
        }

        pub fn has_valid_claim_from_issuer(
            env: Env,
            subject: Address,
            claim_type: String,
            issuer: Address,
        ) -> bool {
            let key = MockKey::Seat(subject, claim_type);
            match env.storage().persistent().get::<_, Address>(&key) {
                Some(granted_by) => granted_by == issuer,
                None => false,
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn setup(env: &Env) -> (
        SeatLicensedResourceClient<'_>,
        MockTrustLinkClient<'_>,
        Address,
        Address,
    ) {
        let admin = Address::generate(env);
        let org_issuer = Address::generate(env);

        let trustlink_id = env.register_contract(None, MockTrustLink);
        let resource_id = env.register_contract(None, SeatLicensedResource);
        let resource = SeatLicensedResourceClient::new(env, &resource_id);
        let trustlink = MockTrustLinkClient::new(env, &trustlink_id);

        resource.initialize(&admin, &trustlink_id, &org_issuer);
        (resource, trustlink, admin, org_issuer)
    }

    // ── Issuance ──────────────────────────────────────────────────────────────

    #[test]
    fn seat_issuance_makes_claim_valid() {
        let env = Env::default();
        env.mock_all_auths();

        let (resource, trustlink, _admin, org_issuer) = setup(&env);
        let employee = Address::generate(&env);

        assert!(!resource.has_active_seat(&employee));
        assert!(!resource.has_any_seat_claim(&employee));

        trustlink.issue_seat(&org_issuer, &employee);

        assert!(trustlink.has_valid_claim(
            &employee,
            &String::from_str(&env, CLAIM_SEAT_LICENSED)
        ));
        assert!(resource.has_active_seat(&employee));
        assert!(resource.has_any_seat_claim(&employee));
    }

    // ── Access gating ─────────────────────────────────────────────────────────

    #[test]
    fn access_granted_with_valid_seat() {
        let env = Env::default();
        env.mock_all_auths();

        let (resource, trustlink, _admin, org_issuer) = setup(&env);
        let employee = Address::generate(&env);

        trustlink.issue_seat(&org_issuer, &employee);

        let count = resource.access_premium_feature(&employee);
        assert_eq!(count, 1);
        assert_eq!(resource.get_access_count(), 1);

        let count2 = resource.access_premium_feature(&employee);
        assert_eq!(count2, 2);
    }

    #[test]
    fn access_denied_without_seat() {
        let env = Env::default();
        env.mock_all_auths();

        let (resource, _trustlink, _admin, _org_issuer) = setup(&env);
        let employee = Address::generate(&env);

        let result = resource.try_access_premium_feature(&employee);
        assert!(result.is_err());
        assert_eq!(resource.get_access_count(), 0);
    }

    #[test]
    fn access_denied_for_foreign_tenant_seat() {
        let env = Env::default();
        env.mock_all_auths();

        let (resource, trustlink, _admin, _org_issuer) = setup(&env);
        let other_tenant = Address::generate(&env);
        let employee = Address::generate(&env);

        // Seat issued by a different SaaS tenant must not unlock this resource.
        trustlink.issue_seat(&other_tenant, &employee);

        assert!(resource.has_any_seat_claim(&employee));
        assert!(!resource.has_active_seat(&employee));

        let result = resource.try_access_premium_feature(&employee);
        assert!(result.is_err());
    }

    // ── Offboarding / revocation ──────────────────────────────────────────────

    #[test]
    fn revocation_on_offboarding_blocks_access() {
        let env = Env::default();
        env.mock_all_auths();

        let (resource, trustlink, _admin, org_issuer) = setup(&env);
        let employee = Address::generate(&env);

        // Onboard: issue seat, access succeeds.
        trustlink.issue_seat(&org_issuer, &employee);
        assert_eq!(resource.access_premium_feature(&employee), 1);

        // Offboard: revoke seat; subsequent access must fail.
        trustlink.revoke_seat(&org_issuer, &employee);

        assert!(!trustlink.has_valid_claim(
            &employee,
            &String::from_str(&env, CLAIM_SEAT_LICENSED)
        ));
        assert!(!resource.has_active_seat(&employee));

        let result = resource.try_access_premium_feature(&employee);
        assert!(result.is_err());
        // Access count unchanged after failed attempt.
        assert_eq!(resource.get_access_count(), 1);
    }

    #[test]
    fn revoke_seat_requires_matching_issuer() {
        let env = Env::default();
        env.mock_all_auths();

        let (_resource, trustlink, _admin, org_issuer) = setup(&env);
        let outsider = Address::generate(&env);
        let employee = Address::generate(&env);

        trustlink.issue_seat(&org_issuer, &employee);

        let result = trustlink.try_revoke_seat(&outsider, &employee);
        assert!(result.is_err());

        // Original issuer can still revoke.
        trustlink.revoke_seat(&org_issuer, &employee);
        assert!(!trustlink.has_valid_claim(
            &employee,
            &String::from_str(&env, CLAIM_SEAT_LICENSED)
        ));
    }

    #[test]
    fn reissue_after_offboarding_restores_access() {
        let env = Env::default();
        env.mock_all_auths();

        let (resource, trustlink, _admin, org_issuer) = setup(&env);
        let employee = Address::generate(&env);

        trustlink.issue_seat(&org_issuer, &employee);
        trustlink.revoke_seat(&org_issuer, &employee);

        // Re-hired / seat re-purchased.
        trustlink.issue_seat(&org_issuer, &employee);
        assert!(resource.has_active_seat(&employee));
        assert_eq!(resource.access_premium_feature(&employee), 1);
    }

    #[test]
    fn initialize_stores_org_issuer_and_trustlink() {
        let env = Env::default();
        env.mock_all_auths();

        let (resource, _trustlink, _admin, org_issuer) = setup(&env);
        assert_eq!(resource.get_org_issuer(), org_issuer);
    }

    #[test]
    fn double_initialize_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let org_issuer = Address::generate(&env);
        let trustlink_id = env.register_contract(None, MockTrustLink);
        let resource_id = env.register_contract(None, SeatLicensedResource);
        let resource = SeatLicensedResourceClient::new(&env, &resource_id);

        resource.initialize(&admin, &trustlink_id, &org_issuer);
        let result = resource.try_initialize(&admin, &trustlink_id, &org_issuer);
        assert!(result.is_err());
    }
}
