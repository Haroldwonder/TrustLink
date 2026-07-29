//! Mutation testing suite for core contract validation and attestation logic.
//!
//! This module contains tests designed to catch subtle authorization bugs that
//! unit test coverage alone might miss. Tests are structured to verify that
//! mutation testing tools (e.g., cargo-mutants) would catch common security
//! vulnerabilities like:
//!
//! - Inverted authorization checks (changing `!=` to `==`)
//! - Removed authorization checks (deleting lines)
//! - Short-circuit logic changes (changing `||` to `&&`)
//! - Return value inversions

#[cfg(test)]
mod mutation_tests {
    use soroban_sdk::{testutils::Address as _, Address, Env, String};
    use trustlink::{Client, Contract};

    #[test]
    fn test_require_admin_inverted_check_would_fail() {
        // This test validates that if the admin check were inverted (allowing
        // non-admins), it would fail. A mutation that changes the authorization
        // logic should cause this test to fail.
        let env = Env::default();
        let contract = Contract {};
        let client = Client::new(&env, &contract);

        let admin = Address::random(&env);
        let unauthorized = Address::random(&env);

        admin.require_auth();
        contract.initialize(&env, &admin, &None);

        // Attempt by unauthorized address should fail
        unauthorized.require_auth();
        let result = contract.register_issuer(&env, &unauthorized, &unauthorized);
        assert!(result.is_err(), "Unauthorized user should not be able to register issuer");
    }

    #[test]
    fn test_require_issuer_must_return_error() {
        // Verifies that creating an attestation without being a registered issuer
        // consistently fails. A mutation that removes the issuer check would cause
        // this test to fail.
        let env = Env::default();
        let contract = Contract {};
        let client = Client::new(&env, &contract);

        let admin = Address::random(&env);
        let issuer = Address::random(&env);
        let subject = Address::random(&env);

        admin.require_auth();
        contract.initialize(&env, &admin, &None);

        // Attempt to create attestation without being registered as issuer
        issuer.require_auth();
        let result = contract.create_attestation(
            &env,
            &issuer,
            &subject,
            &String::from_str(&env, "KYC_PASSED"),
            &None,
            &None,
        );
        assert!(
            result.is_err(),
            "Unregistered issuer should not be able to create attestation"
        );
    }

    #[test]
    fn test_require_admin_short_circuits_on_first_unauthorized() {
        // This test ensures that authorization is checked before any state mutations.
        // A mutation that removes the authorization check would allow state changes
        // by unauthorized users.
        let env = Env::default();
        let contract = Contract {};

        let admin = Address::random(&env);
        let unauthorized = Address::random(&env);

        admin.require_auth();
        contract.initialize(&env, &admin, &None);

        let issuer = Address::random(&env);
        admin.require_auth();
        contract.register_issuer(&env, &admin, &issuer);

        // Unauthorized user attempts to remove issuer
        unauthorized.require_auth();
        let result = contract.remove_issuer(&env, &unauthorized, &issuer);
        assert!(
            result.is_err(),
            "Unauthorized user should not be able to remove issuer"
        );

        // Verify issuer is still registered after failed removal attempt
        assert!(
            contract.is_issuer(&env, &issuer),
            "Issuer should remain registered after unauthorized removal attempt"
        );
    }

    #[test]
    fn test_require_authorized_creator_accepts_either_issuer_or_bridge() {
        // Tests that `require_authorized_creator` properly uses OR logic.
        // A mutation changing `||` to `&&` would cause both branches to fail.
        let env = Env::default();
        let contract = Contract {};

        let admin = Address::random(&env);
        let issuer = Address::random(&env);
        let bridge = Address::random(&env);
        let subject = Address::random(&env);

        admin.require_auth();
        contract.initialize(&env, &admin, &None);

        // Register issuer and bridge separately
        admin.require_auth();
        contract.register_issuer(&env, &admin, &issuer);
        admin.require_auth();
        contract.register_bridge(&env, &admin, &bridge);

        // Issuer should be able to create attestation
        issuer.require_auth();
        let result1 = contract.create_attestation(
            &env,
            &issuer,
            &subject,
            &String::from_str(&env, "KYC_PASSED"),
            &None,
            &None,
        );
        assert!(result1.is_ok(), "Registered issuer should create attestation");

        // Bridge should be able to create attestation
        let subject2 = Address::random(&env);
        bridge.require_auth();
        let result2 = contract.bridge_attestation(
            &env,
            &bridge,
            &subject2,
            &String::from_str(&env, "KYC_PASSED"),
            &String::from_str(&env, "ethereum"),
            &String::from_str(&env, "0xabc"),
        );
        assert!(result2.is_ok(), "Registered bridge should create attestation");
    }

    #[test]
    fn test_require_not_paused_blocks_operations() {
        // Verifies that paused state is enforced. A mutation that removes the pause
        // check would allow operations on a paused contract.
        let env = Env::default();
        let contract = Contract {};

        let admin = Address::random(&env);
        let issuer = Address::random(&env);
        let subject = Address::random(&env);

        admin.require_auth();
        contract.initialize(&env, &admin, &None);

        admin.require_auth();
        contract.register_issuer(&env, &admin, &issuer);

        admin.require_auth();
        contract.pause(&env, &admin);

        // Attempt to create attestation while paused
        issuer.require_auth();
        let result = contract.create_attestation(
            &env,
            &issuer,
            &subject,
            &String::from_str(&env, "KYC_PASSED"),
            &None,
            &None,
        );
        assert!(
            result.is_err(),
            "Creating attestation on paused contract should fail"
        );
    }

    #[test]
    fn test_validate_claim_type_boundary_64_chars() {
        // Tests exact boundary condition: 64 characters should pass,
        // 65 should fail. A mutation changing `len > 64` to `len >= 64`
        // would cause this test to fail.
        let env = Env::default();
        let contract = Contract {};

        let admin = Address::random(&env);
        admin.require_auth();
        contract.initialize(&env, &admin, &None);

        // Exactly 64 characters - should succeed
        let claim_64 = String::from_str(&env, "EXACTLY_64_CHARS_EXACTLY_64_CHARS_EXACTLY_64_CHARS_EXACTLY_64_CH");
        admin.require_auth();
        let result1 = contract.register_claim_type(
            &env,
            &admin,
            &claim_64,
            &String::from_str(&env, "Description"),
        );
        assert!(
            result1.is_ok(),
            "64-character claim type should be valid"
        );

        // 65 characters - should fail
        let claim_65 = String::from_str(&env, "EXACTLY_64_CHARS_EXACTLY_64_CHARS_EXACTLY_64_CHARS_EXACTLY_64_CHX");
        admin.require_auth();
        let result2 = contract.register_claim_type(
            &env,
            &admin,
            &claim_65,
            &String::from_str(&env, "Description"),
        );
        assert!(
            result2.is_err(),
            "65-character claim type should be invalid"
        );
    }

    #[test]
    fn test_validate_claim_type_disallows_invalid_characters() {
        // Tests that invalid characters are rejected. A mutation that removes
        // the character validation loop would allow invalid characters.
        let env = Env::default();
        let contract = Contract {};

        let admin = Address::random(&env);
        admin.require_auth();
        contract.initialize(&env, &admin, &None);

        // Valid: alphanumeric and underscore
        let valid = String::from_str(&env, "VALID_KYC_123");
        admin.require_auth();
        assert!(
            contract.register_claim_type(&env, &admin, &valid, &String::from_str(&env, "Desc")).is_ok(),
            "Valid claim type should succeed"
        );

        // Invalid: contains space
        let invalid_space = String::from_str(&env, "INVALID CLAIM");
        admin.require_auth();
        assert!(
            contract.register_claim_type(&env, &admin, &invalid_space, &String::from_str(&env, "Desc")).is_err(),
            "Claim type with space should fail"
        );

        // Invalid: contains hyphen
        let invalid_hyphen = String::from_str(&env, "INVALID-CLAIM");
        admin.require_auth();
        assert!(
            contract.register_claim_type(&env, &admin, &invalid_hyphen, &String::from_str(&env, "Desc")).is_err(),
            "Claim type with hyphen should fail"
        );

        // Invalid: contains dot
        let invalid_dot = String::from_str(&env, "INVALID.CLAIM");
        admin.require_auth();
        assert!(
            contract.register_claim_type(&env, &admin, &invalid_dot, &String::from_str(&env, "Desc")).is_err(),
            "Claim type with dot should fail"
        );
    }

    #[test]
    fn test_validate_metadata_boundary_256_chars() {
        // Tests exact boundary: 256 characters should pass, 257 should fail.
        // A mutation changing `len > 256` to `len >= 256` would cause failure.
        let env = Env::default();
        let contract = Contract {};

        let admin = Address::random(&env);
        let issuer = Address::random(&env);
        let subject = Address::random(&env);

        admin.require_auth();
        contract.initialize(&env, &admin, &None);

        admin.require_auth();
        contract.register_issuer(&env, &admin, &issuer);

        // Metadata with exactly 256 characters
        let metadata_256 = String::from_str(&env, &"A".repeat(256));
        issuer.require_auth();
        let result1 = contract.create_attestation(
            &env,
            &issuer,
            &subject,
            &String::from_str(&env, "KYC_PASSED"),
            &None,
            &Some(metadata_256),
        );
        assert!(
            result1.is_ok(),
            "256-character metadata should be valid"
        );

        // Metadata with 257 characters
        let subject2 = Address::random(&env);
        let metadata_257 = String::from_str(&env, &"A".repeat(257));
        issuer.require_auth();
        let result2 = contract.create_attestation(
            &env,
            &issuer,
            &subject2,
            &String::from_str(&env, "KYC_PASSED"),
            &None,
            &Some(metadata_257),
        );
        assert!(
            result2.is_err(),
            "257-character metadata should be invalid"
        );
    }
}
