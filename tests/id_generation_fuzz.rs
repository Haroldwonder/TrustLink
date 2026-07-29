// Fuzz-style collision and edge-case tests for TrustLink ID generation (#81)
//
// ID Generation Algorithm
// =======================
// `Attestation::generate_id` concatenates the XDR-encoded bytes of
// (issuer, subject, claim_type, timestamp) and feeds the result into
// SHA-256 via `env.crypto().sha256()`. The 32-byte digest is hex-encoded
// to produce a 64-character string ID.
//
// Collision Probability Analysis (Birthday Paradox)
// ==================================================
// SHA-256 produces a 256-bit output space (2^256 possible values).
// For n = 1 000 randomly-chosen inputs the birthday-paradox collision
// probability is approximately:
//
//   P ≈ n² / (2 × 2^256)
//     = 1 000 000 / (2 × 1.16 × 10^77)
//     ≈ 4.3 × 10^-72
//
// This is astronomically small — effectively zero for any practical
// number of attestations. The tests below empirically confirm this
// property and additionally probe edge-case inputs that could
// theoretically cause hash collisions if the implementation were
// naive (e.g. missing field delimiters). Because XDR encoding is
// used for every field, each value carries its own length prefix,
// making length-extension and boundary-confusion attacks impossible.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String};
use trustlink::types::Attestation;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn gen_id(env: &Env, issuer: &Address, subject: &Address, claim_type: &str, ts: u64) -> String {
    Attestation::generate_id(env, issuer, subject, &String::from_str(env, claim_type), ts)
}

fn gen_bridge_id(
    env: &Env,
    bridge: &Address,
    subject: &Address,
    claim_type: &str,
    source_chain: &str,
    source_tx: &str,
    ts: u64,
) -> String {
    Attestation::generate_bridge_id(
        env,
        bridge,
        subject,
        &String::from_str(env, claim_type),
        &String::from_str(env, source_chain),
        &String::from_str(env, source_tx),
        ts,
    )
}

/// Convert a `soroban_sdk::String` to a `std::string::String` for use in a HashSet.
fn to_std_string(_env: &Env, s: &String) -> std::string::String {
    let len = s.len() as usize;
    let mut buf = vec![0u8; len];
    s.copy_into_slice(&mut buf);
    std::string::String::from_utf8(buf).expect("ID must be valid UTF-8")
}

// ---------------------------------------------------------------------------
// 1. Collision test — 1 000 unique inputs must produce 1 000 unique IDs
// ---------------------------------------------------------------------------

#[test]
fn test_no_collisions_across_1000_unique_inputs() {
    let env = Env::default();
    let mut ids: std::collections::HashSet<std::string::String> = std::collections::HashSet::new();

    let claim_types = [
        "KYC_PASSED",
        "ACCREDITED_INVESTOR",
        "MERCHANT_VERIFIED",
        "AML_CLEARED",
        "SANCTIONS_CHECKED",
    ];

    for i in 0u64..1_000 {
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let claim_type = claim_types[(i % 5) as usize];
        let ts = 1_700_000_000u64 + i * 1_000;
        let id = gen_id(&env, &issuer, &subject, claim_type, ts);
        let key = to_std_string(&env, &id);
        assert!(ids.insert(key), "collision detected at iteration {i}");
    }

    assert_eq!(ids.len(), 1_000);
}

// ---------------------------------------------------------------------------
// 2. Same timestamp, different addresses — must not collide
// ---------------------------------------------------------------------------

#[test]
fn test_same_timestamp_different_addresses_no_collision() {
    let env = Env::default();
    let ts = 1_700_000_000u64;
    let mut ids: std::collections::HashSet<std::string::String> = std::collections::HashSet::new();

    for i in 0u64..200 {
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let id = gen_id(&env, &issuer, &subject, "KYC_PASSED", ts);
        let key = to_std_string(&env, &id);
        assert!(ids.insert(key), "timestamp-race collision at iteration {i}");
    }
    assert_eq!(ids.len(), 200);
}

// ---------------------------------------------------------------------------
// 3. Edge cases — minimal / maximal / boundary inputs
// ---------------------------------------------------------------------------

#[test]
fn test_edge_case_single_char_claim_type() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let id = gen_id(&env, &issuer, &subject, "A", 0);
    assert_eq!(id.len(), 64);
}

#[test]
fn test_edge_case_max_length_claim_type() {
    // 256-character claim type string.
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let long_claim = "X".repeat(256);
    let id = gen_id(&env, &issuer, &subject, &long_claim, 1_000_000);
    assert_eq!(id.len(), 64);
}

#[test]
fn test_edge_case_zero_timestamp() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let id = gen_id(&env, &issuer, &subject, "KYC_PASSED", 0);
    assert_eq!(id.len(), 64);
}

#[test]
fn test_edge_case_max_u64_timestamp() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let id = gen_id(&env, &issuer, &subject, "KYC_PASSED", u64::MAX);
    assert_eq!(id.len(), 64);
}

#[test]
fn test_edge_case_same_addresses_different_claim_types_no_collision() {
    // Same (issuer, subject, timestamp) but different claim types must differ.
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let ts = 1_700_000_000u64;

    let id_a = gen_id(&env, &issuer, &subject, "KYC_PASSED", ts);
    let id_b = gen_id(&env, &issuer, &subject, "AML_CLEARED", ts);
    assert_ne!(id_a, id_b);
}

#[test]
fn test_edge_case_swapped_issuer_subject_produces_different_id() {
    // Swapping issuer and subject must yield a different ID (no commutativity).
    let env = Env::default();
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let ts = 1_700_000_000u64;

    let id_ab = gen_id(&env, &a, &b, "KYC_PASSED", ts);
    let id_ba = gen_id(&env, &b, &a, "KYC_PASSED", ts);
    assert_ne!(id_ab, id_ba);
}

// ---------------------------------------------------------------------------
// 4. Bridge ID — collision test across 200 unique bridge inputs
// ---------------------------------------------------------------------------

#[test]
fn test_bridge_id_no_collisions_across_200_inputs() {
    let env = Env::default();
    let mut ids: std::collections::HashSet<std::string::String> = std::collections::HashSet::new();

    for i in 0u64..200 {
        let bridge = Address::generate(&env);
        let subject = Address::generate(&env);
        let source_tx = format!("0x{i:064x}");
        let id = gen_bridge_id(
            &env,
            &bridge,
            &subject,
            "KYC_PASSED",
            "ethereum",
            &source_tx,
            1_700_000_000 + i,
        );
        let key = to_std_string(&env, &id);
        assert!(ids.insert(key), "bridge ID collision at iteration {i}");
    }
    assert_eq!(ids.len(), 200);
}

// ---------------------------------------------------------------------------
// 5. Determinism — same inputs always produce the same ID
// ---------------------------------------------------------------------------

#[test]
fn test_id_is_deterministic() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let ts = 1_700_000_000u64;

    let id1 = gen_id(&env, &issuer, &subject, "KYC_PASSED", ts);
    let id2 = gen_id(&env, &issuer, &subject, "KYC_PASSED", ts);
    assert_eq!(id1, id2);
}

// ---------------------------------------------------------------------------
// 6. Output format — IDs must be 64-character lowercase hex strings
// ---------------------------------------------------------------------------

#[test]
fn test_id_output_is_64_char_lowercase_hex() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let id = gen_id(&env, &issuer, &subject, "KYC_PASSED", 1_700_000_000);

    assert_eq!(id.len(), 64, "ID length must be 64");
    let s = to_std_string(&env, &id);
    assert!(
        s.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "ID must be lowercase hex, got: {s}"
    );
}
