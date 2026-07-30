// Property-based tests: OR/AND-logic consistency across the has_valid_claim family
//
// The functions has_valid_claim, has_any_claim, has_all_claims, and
// has_valid_claim_from_issuer are logically related:
// - has_valid_claim(subject, claim) = single-claim validation
// - has_any_claim(subject, [a, b, ...]) = OR-across-claims
// - has_all_claims(subject, [a, b, ...]) = AND-across-claims
// - has_valid_claim_from_issuer(subject, claim, issuer) = single-issuer-scoped
//
// This test suite asserts that these functions obey their logical relationships
// across randomly generated attestation sets.

#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Vec,
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

/// Generate a random claim type string.
fn random_claim_type(env: &Env, idx: u32) -> String {
    String::from_str(env, &format!("CLAIM_TYPE_{}", idx))
}

/// Generate a random issuer address.
fn random_issuer(env: &Env, idx: u32) -> Address {
    Address::generate(env)
}

// ── Logical consistency tests ────────────────────────────────────────────────

proptest! {
    // ── Single-claim consistency ────────────────────────────────────────────

    /// has_valid_claim(subject, claim) should equal has_any_claim(subject, [claim])
    /// Invariant: Single-claim OR with one element equals the basic check.
    #[test]
    fn prop_single_claim_or_equals_basic_check(
        state in attestation_state_strategy(),
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

        match state {
            AttestationState::Valid => {
                client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None)
            }
            AttestationState::Revoked => {
                let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
                client.revoke_attestation(&issuer, &id, &None);
                id
            }
            AttestationState::Expired => {
                let saved = env.ledger().timestamp();
                env.ledger().with_mut(|l| l.timestamp = 5_000);
                let id = client.create_attestation(
                    &issuer, &subject, &claim_type, &Some(9_000), &None, &None,
                );
                env.ledger().with_mut(|l| l.timestamp = saved);
            }
            AttestationState::Pending => {
                let valid_from = now + 10_000;
                client.create_attestation_valid_from(
                    &issuer, &subject, &claim_type, &None, &None, &None, &valid_from,
                )
            }
        };

        let basic_check = client.has_valid_claim(&subject, &claim_type);

        let claim_vec: Vec<String> = Vec::new(&env);
        let claim_vec = claim_vec.push_back(claim_type.clone());
        let any_claim = client.has_any_claim(&subject, claim_vec);

        prop_assert_eq!(
            basic_check, any_claim,
            "has_valid_claim({}) != has_any_claim([{}]): basic={}, any={}",
            basic_check, claim_type, basic_check, any_claim
        );
    }

    // ── OR-logic consistency ────────────────────────────────────────────────

    /// has_any_claim(subject, [a, b]) should return true iff
    /// has_valid_claim(subject, a) OR has_valid_claim(subject, b) is true.
    #[test]
    fn prop_any_claim_or_consistency(
        states_a in prop::collection::vec(attestation_state_strategy(), 1..3),
        states_b in prop::collection::vec(attestation_state_strategy(), 1..3),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let client = deploy(&env);
        let admin   = Address::generate(&env);
        let issuer  = Address::generate(&env);
        let subject = Address::generate(&env);
        let claim_a = String::from_str(&env, "CLAIM_A");
        let claim_b = String::from_str(&env, "CLAIM_B");

        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);

        let now: u64 = 10_000;

        // Create attestations for claim A
        let mut any_a_valid = false;
        for state in &states_a {
            env.ledger().with_mut(|l| l.timestamp += 1);
            let ts = env.ledger().timestamp();

            let id = match state {
                AttestationState::Valid => {
                    client.create_attestation(&issuer, &subject, &claim_a, &None, &None, &None)
                }
                AttestationState::Revoked => {
                    let id = client.create_attestation(&issuer, &subject, &claim_a, &None, &None, &None);
                    client.revoke_attestation(&issuer, &id, &None);
                    id
                }
                AttestationState::Expired => {
                    let saved = env.ledger().timestamp();
                    let create_ts = saved - 5_000;
                    let exp = saved - 1_000;
                    env.ledger().with_mut(|l| l.timestamp = create_ts);
                    let id = client.create_attestation(
                        &issuer, &subject, &claim_a, &Some(exp), &None, &None,
                    );
                    env.ledger().with_mut(|l| l.timestamp = saved);
                    id
                }
                AttestationState::Pending => {
                    let valid_from = ts + 10_000;
                    client.create_attestation_valid_from(
                        &issuer, &subject, &claim_a, &None, &None, &None, &valid_from,
                    )
                }
            };
            let status = client.get_attestation_status(&id);
            if status == trustlink::types::AttestationStatus::Valid {
                any_a_valid = true;
            }
        }

        // Create attestations for claim B
        let mut any_b_valid = false;
        for state in &states_b {
            env.ledger().with_mut(|l| l.timestamp += 1);
            let ts = env.ledger().timestamp();

            let id = match state {
                AttestationState::Valid => {
                    client.create_attestation(&issuer, &subject, &claim_b, &None, &None, &None)
                }
                AttestationState::Revoked => {
                    let id = client.create_attestation(&issuer, &subject, &claim_b, &None, &None, &None);
                    client.revoke_attestation(&issuer, &id, &None);
                    id
                }
                AttestationState::Expired => {
                    let saved = env.ledger().timestamp();
                    let create_ts = saved - 5_000;
                    let exp = saved - 1_000;
                    env.ledger().with_mut(|l| l.timestamp = create_ts);
                    let id = client.create_attestation(
                        &issuer, &subject, &claim_b, &Some(exp), &None, &None,
                    );
                    env.ledger().with_mut(|l| l.timestamp = saved);
                    id
                }
                AttestationState::Pending => {
                    let valid_from = ts + 10_000;
                    client.create_attestation_valid_from(
                        &issuer, &subject, &claim_b, &None, &None, &None, &valid_from,
                    )
                }
            };
            let status = client.get_attestation_status(&id);
            if status == trustlink::types::AttestationStatus::Valid {
                any_b_valid = true;
            }
        }

        // Reset to base time for the final query.
        env.ledger().with_mut(|l| l.timestamp = 10_000);

        // Expected: A OR B
        let expected_any = any_a_valid || any_b_valid;

        let claim_vec: Vec<String> = Vec::new(&env);
        let claim_vec = claim_vec.push_back(claim_a.clone()).push_back(claim_b.clone());
        let actual_any = client.has_any_claim(&subject, claim_vec);

        prop_assert_eq!(
            actual_any, expected_any,
            "has_any_claim([A, B])={}, expected A({}) OR B({})={}",
            actual_any, any_a_valid, any_b_valid, expected_any
        );
    }

    // ── AND-logic consistency ───────────────────────────────────────────────

    /// has_all_claims(subject, [a, b]) should return true iff
    /// has_valid_claim(subject, a) AND has_valid_claim(subject, b) are both true.
    #[test]
    fn prop_all_claims_and_consistency(
        states_a in prop::collection::vec(attestation_state_strategy(), 1..3),
        states_b in prop::collection::vec(attestation_state_strategy(), 1..3),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let client = deploy(&env);
        let admin   = Address::generate(&env);
        let issuer  = Address::generate(&env);
        let subject = Address::generate(&env);
        let claim_a = String::from_str(&env, "CLAIM_A");
        let claim_b = String::from_str(&env, "CLAIM_B");

        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);

        let now: u64 = 10_000;

        // Create attestations for claim A
        let mut any_a_valid = false;
        for state in &states_a {
            env.ledger().with_mut(|l| l.timestamp += 1);
            let ts = env.ledger().timestamp();

            let id = match state {
                AttestationState::Valid => {
                    client.create_attestation(&issuer, &subject, &claim_a, &None, &None, &None)
                }
                AttestationState::Revoked => {
                    let id = client.create_attestation(&issuer, &subject, &claim_a, &None, &None, &None);
                    client.revoke_attestation(&issuer, &id, &None);
                    id
                }
                AttestationState::Expired => {
                    let saved = env.ledger().timestamp();
                    let create_ts = saved - 5_000;
                    let exp = saved - 1_000;
                    env.ledger().with_mut(|l| l.timestamp = create_ts);
                    let id = client.create_attestation(
                        &issuer, &subject, &claim_a, &Some(exp), &None, &None,
                    );
                    env.ledger().with_mut(|l| l.timestamp = saved);
                    id
                }
                AttestationState::Pending => {
                    let valid_from = ts + 10_000;
                    client.create_attestation_valid_from(
                        &issuer, &subject, &claim_a, &None, &None, &None, &valid_from,
                    )
                }
            };
            let status = client.get_attestation_status(&id);
            if status == trustlink::types::AttestationStatus::Valid {
                any_a_valid = true;
            }
        }

        // Create attestations for claim B
        let mut any_b_valid = false;
        for state in &states_b {
            env.ledger().with_mut(|l| l.timestamp += 1);
            let ts = env.ledger().timestamp();

            let id = match state {
                AttestationState::Valid => {
                    client.create_attestation(&issuer, &subject, &claim_b, &None, &None, &None)
                }
                AttestationState::Revoked => {
                    let id = client.create_attestation(&issuer, &subject, &claim_b, &None, &None, &None);
                    client.revoke_attestation(&issuer, &id, &None);
                    id
                }
                AttestationState::Expired => {
                    let saved = env.ledger().timestamp();
                    let create_ts = saved - 5_000;
                    let exp = saved - 1_000;
                    env.ledger().with_mut(|l| l.timestamp = create_ts);
                    let id = client.create_attestation(
                        &issuer, &subject, &claim_b, &Some(exp), &None, &None,
                    );
                    env.ledger().with_mut(|l| l.timestamp = saved);
                    id
                }
                AttestationState::Pending => {
                    let valid_from = ts + 10_000;
                    client.create_attestation_valid_from(
                        &issuer, &subject, &claim_b, &None, &None, &None, &valid_from,
                    )
                }
            };
            let status = client.get_attestation_status(&id);
            if status == trustlink::types::AttestationStatus::Valid {
                any_b_valid = true;
            }
        }

        // Reset to base time for the final query.
        env.ledger().with_mut(|l| l.timestamp = 10_000);

        // Expected: A AND B
        let expected_all = any_a_valid && any_b_valid;

        let claim_vec: Vec<String> = Vec::new(&env);
        let claim_vec = claim_vec.push_back(claim_a.clone()).push_back(claim_b.clone());
        let actual_all = client.has_all_claims(&subject, claim_vec);

        prop_assert_eq!(
            actual_all, expected_all,
            "has_all_claims([A, B])={}, expected A({}) AND B({})={}",
            actual_all, any_a_valid, any_b_valid, expected_all
        );
    }

    // ── Implication: ALL implies ANY ────────────────────────────────────────

    /// has_all_claims(subject, [a, b]) => has_any_claim(subject, [a, b])
    /// If all claims are present, then at least one must be present.
    /// This is the key cross-function invariant that would have caught
    /// the performance-inconsistency issue.
    #[test]
    fn prop_all_claims_implies_any_claim(
        states_a in prop::collection::vec(attestation_state_strategy(), 1..3),
        states_b in prop::collection::vec(attestation_state_strategy(), 1..3),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let client = deploy(&env);
        let admin   = Address::generate(&env);
        let issuer  = Address::generate(&env);
        let subject = Address::generate(&env);
        let claim_a = String::from_str(&env, "CLAIM_A");
        let claim_b = String::from_str(&env, "CLAIM_B");

        client.initialize(&admin, &None);
        client.register_issuer(&admin, &issuer);

        let now: u64 = 10_000;

        // Create attestations for claim A
        let mut any_a_valid = false;
        for state in &states_a {
            env.ledger().with_mut(|l| l.timestamp += 1);
            let ts = env.ledger().timestamp();

            let id = match state {
                AttestationState::Valid => {
                    client.create_attestation(&issuer, &subject, &claim_a, &None, &None, &None)
                }
                AttestationState::Revoked => {
                    let id = client.create_attestation(&issuer, &subject, &claim_a, &None, &None, &None);
                    client.revoke_attestation(&issuer, &id, &None);
                    id
                }
                AttestationState::Expired => {
                    let saved = env.ledger().timestamp();
                    let create_ts = saved - 5_000;
                    let exp = saved - 1_000;
                    env.ledger().with_mut(|l| l.timestamp = create_ts);
                    let id = client.create_attestation(
                        &issuer, &subject, &claim_a, &Some(exp), &None, &None,
                    );
                    env.ledger().with_mut(|l| l.timestamp = saved);
                    id
                }
                AttestationState::Pending => {
                    let valid_from = ts + 10_000;
                    client.create_attestation_valid_from(
                        &issuer, &subject, &claim_a, &None, &None, &None, &valid_from,
                    )
                }
            };
            let status = client.get_attestation_status(&id);
            if status == trustlink::types::AttestationStatus::Valid {
                any_a_valid = true;
            }
        }

        // Create attestations for claim B
        let mut any_b_valid = false;
        for state in &states_b {
            env.ledger().with_mut(|l| l.timestamp += 1);
            let ts = env.ledger().timestamp();

            let id = match state {
                AttestationState::Valid => {
                    client.create_attestation(&issuer, &subject, &claim_b, &None, &None, &None)
                }
                AttestationState::Revoked => {
                    let id = client.create_attestation(&issuer, &subject, &claim_b, &None, &None, &None);
                    client.revoke_attestation(&issuer, &id, &None);
                    id
                }
                AttestationState::Expired => {
                    let saved = env.ledger().timestamp();
                    let create_ts = saved - 5_000;
                    let exp = saved - 1_000;
                    env.ledger().with_mut(|l| l.timestamp = create_ts);
                    let id = client.create_attestation(
                        &issuer, &subject, &claim_b, &Some(exp), &None, &None,
                    );
                    env.ledger().with_mut(|l| l.timestamp = saved);
                    id
                }
                AttestationState::Pending => {
                    let valid_from = ts + 10_000;
                    client.create_attestation_valid_from(
                        &issuer, &subject, &claim_b, &None, &None, &None, &valid_from,
                    )
                }
            };
            let status = client.get_attestation_status(&id);
            if status == trustlink::types::AttestationStatus::Valid {
                any_b_valid = true;
            }
        }

        // Reset to base time for the final query.
        env.ledger().with_mut(|l| l.timestamp = 10_000);

        let claim_vec: Vec<String> = Vec::new(&env);
        let claim_vec = claim_vec.push_back(claim_a.clone()).push_back(claim_b.clone());

        let has_all = client.has_all_claims(&subject, claim_vec.clone());
        let has_any = client.has_any_claim(&subject, claim_vec);

        // Implication: if ALL is true, then ANY must also be true
        prop_assert!(
            !has_all || has_any,
            "has_all_claims([A, B])={} but has_any_claim([A, B])={} - implication violated",
            has_all, has_any
        );
    }

    // ── Single-issuer consistency ───────────────────────────────────────────

    /// has_valid_claim_from_issuer(subject, claim, issuer) should be true
    /// only if that specific issuer has a valid attestation for the claim.
    /// This should imply has_valid_claim(subject, claim) if the issuer is valid.
    #[test]
    fn prop_issuer_scoped_implies_global_valid(
        issuer_state in attestation_state_strategy(),
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

        let id = match issuer_state {
            AttestationState::Valid => {
                client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None)
            }
            AttestationState::Revoked => {
                let id = client.create_attestation(&issuer, &subject, &claim_type, &None, &None, &None);
                client.revoke_attestation(&issuer, &id, &None);
                id
            }
            AttestationState::Expired => {
                let saved = env.ledger().timestamp();
                env.ledger().with_mut(|l| l.timestamp = 5_000);
                let id = client.create_attestation(
                    &issuer, &subject, &claim_type, &Some(9_000), &None, &None,
                );
                env.ledger().with_mut(|l| l.timestamp = saved);
            }
            AttestationState::Pending => {
                let valid_from = now + 10_000;
                client.create_attestation_valid_from(
                    &issuer, &subject, &claim_type, &None, &None, &None, &valid_from,
                )
            }
        };

        let issuer_check = client.has_valid_claim_from_issuer(&subject, &claim_type, &issuer);
        let global_check = client.has_valid_claim(&subject, &claim_type);

        // If the issuer-specific check is true, the global check must also be true
        prop_assert!(
            !issuer_check || global_check,
            "has_valid_claim_from_issuer(issuer={})={} but has_valid_claim={}",
            issuer, issuer_check, global_check
        );

        // The reverse is also true when there's only one issuer
        prop_assert_eq!(
            issuer_check, global_check,
            "With single issuer, issuer-specific and global should match: issuer={}, global={}",
            issuer_check, global_check
        );
    }
}
