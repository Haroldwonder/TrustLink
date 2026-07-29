# Implementation Plan: Attestation ID Stability

## Overview

Purely additive changes: add inline doc-comments to the three ID-generation functions in
`src/types.rs`, write regression (example) tests and property-based tests in `src/test.rs`,
and add a CHANGELOG entry. No production logic is modified.

## Tasks

- [x] 1. Add inline doc-comments to `hash_payload`, `generate_id`, and `generate_bridge_id`
  - Open `src/types.rs` and add `///` doc-comments above each of the three functions
  - `hash_payload`: describe the algorithm (SHA-256 → first 16 bytes → lowercase hex)
  - `generate_id`: list XDR field order: `issuer | subject | claim_type | timestamp`
  - `generate_bridge_id`: list XDR field order: `bridge | subject | claim_type | source_chain | source_tx | timestamp`
  - _Requirements: 3.1, 3.2_

- [ ] 2. Add `proptest` dev-dependency and write property-based tests
  - Add `proptest = "1"` to `[dev-dependencies]` in `Cargo.toml`
  - In `src/test.rs`, add four property tests using `proptest!` macro, each with a comment in the format: `// Feature: attestation-id-stability, Property N: ...`
  - [ ]* 2.1 Write property test `test_generate_id_output_format`
    - **Property 1: Output format invariant**
    - Generate random byte vecs for issuer/subject bytes, claim_type string, and timestamp
    - Assert result is exactly 32 chars and matches `^[0-9a-f]{32}$`
    - **Validates: Requirements 1.1, 2.1**
  - [ ]* 2.2 Write property test `test_generate_bridge_id_output_format`
    - **Property 1: Output format invariant**
    - Same as above but for `generate_bridge_id` with bridge/subject/claim_type/source_chain/source_tx/timestamp
    - **Validates: Requirements 1.1, 2.1**
  - [ ]* 2.3 Write property test `test_generate_id_determinism`
    - **Property 2: Determinism**
    - Call `generate_id` twice with identical inputs, assert both results are equal
    - **Validates: Requirements 1.2, 2.2**
  - [ ]* 2.4 Write property test `test_generate_bridge_id_determinism`
    - **Property 2: Determinism**
    - Call `generate_bridge_id` twice with identical inputs, assert both results are equal
    - **Validates: Requirements 1.2, 2.2**

- [x] 3. Write regression (example) tests
  - In `src/test.rs`, add two `#[test]` functions with hard-coded inputs and expected hex strings
  - Each test must include a comment stating the inputs and expected hash value (Requirement 3.3)
  - [x] 3.1 Write `test_generate_id_stability`
    - Construct a deterministic `Env::default()`, fixed `Address` values from known byte slices, fixed claim_type string, and fixed timestamp
    - Run `Attestation::generate_id` and capture the actual output as the hard-coded expected value
    - Assert `result == expected_hex`
    - _Requirements: 1.3, 1.4_
  - [ ]* 3.2 Write `test_generate_bridge_id_stability`
    - Same approach for `generate_bridge_id` with bridge/subject/claim_type/source_chain/source_tx/timestamp
    - **Example 4: Bridge attestation ID regression**
    - **Validates: Requirements 2.3, 2.4**

- [x] 4. Checkpoint — ensure all tests pass
  - Run `cargo test` and confirm all existing tests plus the new stability and property tests pass.
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. Add CHANGELOG entry
  - In `CHANGELOG.md`, under `[Unreleased]` → `Added`, append an entry documenting that
    `Attestation::generate_id` and `Attestation::generate_bridge_id` now carry a stability
    guarantee enforced by regression tests
  - Reference the regression tests (`test_generate_id_stability`, `test_generate_bridge_id_stability`) as the enforcement mechanism
  - _Requirements: 4.1, 4.2_

- [ ] 6. Final checkpoint — ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- The regression tests (task 3) are the primary correctness mechanism; property tests (task 2) provide universal coverage
- No production logic changes — all modifications are additive (comments, tests, changelog)
- Property tests require `proptest = "1"` in `[dev-dependencies]`; run with `cargo test --features testutils`
