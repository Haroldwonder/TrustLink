// Property-based fuzz tests for attestation ID collision resistance (#320)
//
// Verifies three invariants using proptest:
//   1. Different (issuer, subject, claim_type, timestamp) tuples → different IDs
//   2. Same inputs → same ID (determinism)
//   3. IDs are always exactly 64 lowercase hex characters

#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};
use trustlink::types::Attestation;

fn gen_id(env: &Env, issuer: &Address, subject: &Address, claim_type: &str, ts: u64) -> String {
    Attestation::generate_id(env, issuer, subject, &String::from_str(env, claim_type), ts)
}

fn to_std(env: &Env, s: &String) -> std::string::String {
    let mut buf = vec![0u8; s.len() as usize];
    s.copy_into_slice(&mut buf);
    std::string::String::from_utf8(buf).unwrap()
}

// Claim types are bounded to valid ASCII identifiers (1–64 chars).
fn claim_type_strategy() -> impl Strategy<Value = std::string::String> {
    "[A-Z_]{1,64}"
}

// ---------------------------------------------------------------------------
// 1. Collision resistance — different inputs must produce different IDs
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_different_timestamps_produce_different_ids(
        ts_a in 0u64..u64::MAX - 1,
        ts_b in 0u64..u64::MAX - 1,
    ) {
        prop_assume!(ts_a != ts_b);
        let env = Env::default();
        let issuer  = Address::generate(&env);
        let subject = Address::generate(&env);
        let id_a = gen_id(&env, &issuer, &subject, "KYC_PASSED", ts_a);
        let id_b = gen_id(&env, &issuer, &subject, "KYC_PASSED", ts_b);
        prop_assert_ne!(to_std(&env, &id_a), to_std(&env, &id_b));
    }

    #[test]
    fn prop_different_claim_types_produce_different_ids(
        claim_a in claim_type_strategy(),
        claim_b in claim_type_strategy(),
    ) {
        prop_assume!(claim_a != claim_b);
        let env = Env::default();
        let issuer  = Address::generate(&env);
        let subject = Address::generate(&env);
        let ts = 1_700_000_000u64;
        let id_a = gen_id(&env, &issuer, &subject, &claim_a, ts);
        let id_b = gen_id(&env, &issuer, &subject, &claim_b, ts);
        prop_assert_ne!(to_std(&env, &id_a), to_std(&env, &id_b));
    }

    #[test]
    fn prop_swapped_issuer_subject_produces_different_id(ts in 0u64..u64::MAX) {
        let env = Env::default();
        let a = Address::generate(&env);
        let b = Address::generate(&env);
        // Only meaningful when the two addresses differ (always true for generate).
        let id_ab = gen_id(&env, &a, &b, "KYC_PASSED", ts);
        let id_ba = gen_id(&env, &b, &a, "KYC_PASSED", ts);
        prop_assert_ne!(to_std(&env, &id_ab), to_std(&env, &id_ba));
    }
}

// ---------------------------------------------------------------------------
// 2. Determinism — same inputs always produce the same ID
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_id_is_deterministic(
        claim in claim_type_strategy(),
        ts in 0u64..u64::MAX,
    ) {
        let env = Env::default();
        let issuer  = Address::generate(&env);
        let subject = Address::generate(&env);
        let id1 = gen_id(&env, &issuer, &subject, &claim, ts);
        let id2 = gen_id(&env, &issuer, &subject, &claim, ts);
        prop_assert_eq!(to_std(&env, &id1), to_std(&env, &id2));
    }
}

// ---------------------------------------------------------------------------
// 3. Output format — always exactly 64 lowercase hex characters
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_id_is_64_char_lowercase_hex(
        claim in claim_type_strategy(),
        ts in 0u64..u64::MAX,
    ) {
        let env = Env::default();
        let issuer  = Address::generate(&env);
        let subject = Address::generate(&env);
        let id = gen_id(&env, &issuer, &subject, &claim, ts);
        let s  = to_std(&env, &id);
        prop_assert_eq!(s.len(), 64, "expected 64 chars, got {}", s.len());
        prop_assert!(
            s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "ID is not lowercase hex: {s}"
        );
    }
}
