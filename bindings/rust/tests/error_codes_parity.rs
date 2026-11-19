//! Guards the Rust binding's error table against drift from the contract.
//!
//! `ContractErrorCode` was previously hand-maintained and silently desynced:
//! codes 8 and 10-15 mapped to the wrong names, and 9, 21-30 and 44 were absent
//! altogether, so `rpc::parse_contract_error` reported the wrong variant for
//! any of them. The enum is now generated from `src/errors.rs` alongside the
//! Python and TypeScript tables; this test fails if the committed output stops
//! matching the canonical `error-codes.json`.

use serde::Deserialize;
use std::{fs, path::PathBuf};
use trustlink_client::types::ContractErrorCode;

#[derive(Deserialize)]
struct ErrorEntry {
    code: u32,
    name: String,
}

#[derive(Deserialize)]
struct ErrorTable {
    errors: Vec<ErrorEntry>,
}

/// Read the canonical table from the repository root.
fn canonical_table() -> Vec<ErrorEntry> {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "error-codes.json"]
        .iter()
        .collect();

    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));

    serde_json::from_str::<ErrorTable>(&raw)
        .expect("error-codes.json is not a valid error table")
        .errors
}

#[test]
fn every_canonical_code_maps_to_its_own_variant() {
    for entry in canonical_table() {
        let mapped = ContractErrorCode::from(entry.code);

        assert_ne!(
            mapped,
            ContractErrorCode::Unknown,
            "contract error {} ({}) is missing from the Rust binding — run `make generate`",
            entry.code,
            entry.name,
        );

        assert_eq!(
            mapped.name(),
            entry.name,
            "contract error {} is {} on-chain but {} in the Rust binding",
            entry.code,
            entry.name,
            mapped.name(),
        );

        assert_eq!(
            mapped.code(),
            entry.code,
            "{} round-trips to the wrong numeric code",
            entry.name,
        );
    }
}

#[test]
fn unrecognised_codes_fall_back_to_unknown() {
    // Far outside the allocated range, so it stays valid as the table grows.
    assert_eq!(ContractErrorCode::from(9_999), ContractErrorCode::Unknown);
    assert_eq!(ContractErrorCode::Unknown.name(), "Unknown");
}

#[test]
fn codes_the_binding_previously_got_wrong_are_correct() {
    // Regression pins for the exact mismatches reported in the issue.
    assert_eq!(ContractErrorCode::from(8).name(), "InvalidValidFrom");
    assert_eq!(ContractErrorCode::from(10).name(), "MetadataTooLong");
    assert_eq!(ContractErrorCode::from(11).name(), "InvalidTimestamp");
    assert_eq!(ContractErrorCode::from(12).name(), "InvalidFee");
    assert_eq!(ContractErrorCode::from(13).name(), "FeeTokenRequired");
    assert_eq!(ContractErrorCode::from(14).name(), "TooManyTags");
    assert_eq!(ContractErrorCode::from(15).name(), "TagTooLong");

    // Previously absent entirely.
    assert_eq!(ContractErrorCode::from(9).name(), "InvalidExpiration");
    assert_eq!(ContractErrorCode::from(16).name(), "InvalidThreshold");
    assert_eq!(ContractErrorCode::from(17).name(), "NotRequiredSigner");
    assert_eq!(ContractErrorCode::from(18).name(), "AlreadySigned");
    assert_eq!(ContractErrorCode::from(19).name(), "ProposalFinalized");
    assert_eq!(ContractErrorCode::from(20).name(), "ProposalExpired");
    assert_eq!(ContractErrorCode::from(29).name(), "LimitExceeded");
    assert_eq!(ContractErrorCode::from(44).name(), "InvalidSourceReference");
}

#[test]
fn display_includes_name_and_code() {
    assert_eq!(
        ContractErrorCode::from(24).to_string(),
        "ContractPaused (#24)",
    );
}
