// Property-based tests: has_valid_claim is consistent with get_attestation_status (#509)
//
// Invariant: has_valid_claim(subject, claim_type) == true
//            iff at least one non-deleted attestation for (subject, claim_type)
//            has get_attestation_status() == AttestationStatus::Valid.

#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};
use trustlink::{TrustLinkContract, TrustLinkContractClient};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn deploy(env: &Env) -> TrustLinkContractClient {
    let id = env.register_contract(None, TrustLinkContract);
    TrustLinkContractClient::new(env, &id)
}

/// Possible states we can put an attestation into.
#[derive(Debug, Clone)]
enum AttestationState {
    Valid,
    Revoked,
    Expired,
    Pending,
}

fn attestation_state_strategy() -> impl Strategy<Value = AttestationState> {
    prop_oneof![
        Just(AttestationState::Valid),
        Just(AttestationState::Revoked),
        Just(AttestationState::Expired),
        Just(AttestationState::Pending),
    ]
}

// ── Core invariant ────────────────────────────────────────────────────────────

proptest! {
    /// Single attestation: has_valid_claim matches get_attestation_status.
    #[test]
    fn prop_has_valid_claim_consistent_with_status_single(
        state in attestation_state_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        // Base time: 10_000 seconds.
        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let client = deploy(&env);
        let admin   = Address::generate(&env);
        let issuer  = Address::generate(&env);
        let subject = Address::generate(&env);
        let claim_type = String::from_str(&env, "KYC_PASSED");

        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);

        let now: u64 = 10_000;

        let id = match state {
            AttestationState::Valid => {
                // No expiration → always valid.
                client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None)
            }
            AttestationState::Revoked => {
                let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
                client.revoke_attestation(&issuer, &id, &None);
                id
            }
            AttestationState::Expired => {
                // Expiration in the past.
                let exp = now - 1;
                client.create_attestation(&issuer, &subject, &claim_type, &Some(exp), &None, &None)
            }
            AttestationState::Pending => {
                // valid_from in the future.
                let valid_from = now + 10_000;
                client.create_attestation_valid_from(
                    &issuer, &subject, &claim_type, &None, &Some(valid_from), &None,
                )
            }
        };

        let status = client.get_attestation_status(&id);
        let has_valid = client.has_valid_claim(&subject, &claim_type);

        // Invariant: has_valid_claim ↔ at least one Valid attestation exists.
        let status_is_valid = status == trustlink::types::AttestationStatus::Valid;
        prop_assert_eq!(
            has_valid,
            status_is_valid,
            "has_valid_claim={has_valid} but status={status:?}"
        );
    }

    /// Multiple attestations with mixed states: has_valid_claim is true iff
    /// at least one has status Valid.
    #[test]
    fn prop_has_valid_claim_consistent_with_status_multi(
        states in prop::collection::vec(attestation_state_strategy(), 1..5),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let client = deploy(&env);
        let admin   = Address::generate(&env);
        let issuer  = Address::generate(&env);
        let subject = Address::generate(&env);
        let claim_type = String::from_str(&env, "KYC_PASSED");

        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);

        let now: u64 = 10_000;
        let mut any_valid = false;

        for state in &states {
            // Advance time slightly so each attestation gets a unique ID.
            env.ledger().with_mut(|l| l.timestamp += 1);
            let ts = env.ledger().timestamp();

            let id = match state {
                AttestationState::Valid => {
                    client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None)
                }
                AttestationState::Revoked => {
                    let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
                    client.revoke_attestation(&issuer, &id, &None);
                    id
                }
                AttestationState::Expired => {
                    let exp = now - 1;
                    client.create_attestation(&issuer, &subject, &claim_type, &Some(exp), &None, &None)
                }
                AttestationState::Pending => {
                    let valid_from = ts + 10_000;
                    client.create_attestation_valid_from(
                        &issuer, &subject, &claim_type, &None, &Some(valid_from), &None,
                    )
                }
            };

            // Track whether this attestation is valid at query time.
            let status = client.get_attestation_status(&id);
            if status == trustlink::types::AttestationStatus::Valid {
                any_valid = true;
            }
        }

        // Reset to base time for the final query.
        env.ledger().with_mut(|l| l.timestamp = 10_000);
        let has_valid = client.has_valid_claim(&subject, &claim_type);

        prop_assert_eq!(
            has_valid,
            any_valid,
            "has_valid_claim={has_valid} but expected any_valid={any_valid} (states={states:?})"
        );
    }
}
