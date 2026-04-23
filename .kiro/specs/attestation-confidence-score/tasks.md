# Implementation Plan: Attestation Confidence Score

## Overview

Add a `confidence: u8` field to the `Attestation` struct, thread it through `create_attestation` with validation, and expose a new `has_claim_with_min_confidence` query function. All changes are additive and backward-compatible.

## Tasks

- [ ] 1. Add `confidence` field and `InvalidConfidence` error to types
  - Add `pub confidence: u8` as the last field of the `Attestation` struct in `src/types.rs`
  - Add `InvalidConfidence = 21` variant to the `Error` enum in `src/types.rs`
  - _Requirements: 1.1, 2.1_

- [ ] 2. Add confidence validation to `validation.rs`
  - [ ] 2.1 Implement `Validation::require_valid_confidence`
    - Add `pub fn require_valid_confidence(confidence: u8) -> Result<(), Error>` to `src/validation.rs`
    - Return `Err(Error::InvalidConfidence)` when `confidence > 100`, `Ok(())` otherwise
    - _Requirements: 2.1, 2.2_

  - [ ]* 2.2 Write property test for `require_valid_confidence` (Property 2 & 3)
    - **Property 2: Invalid confidence rejected** — for any `u8` in `101..=255`, must return `Err(InvalidConfidence)`
    - **Property 3: Valid confidence accepted** — for any `u8` in `0..=100`, must return `Ok(())`
    - **Validates: Requirements 2.1, 2.2**

- [ ] 3. Update `create_attestation` in `src/lib.rs`
  - [ ] 3.1 Add `confidence: Option<u8>` parameter to `create_attestation`
    - Insert `confidence: Option<u8>` as the last parameter
    - Resolve the value: `let confidence = confidence.unwrap_or(100);`
    - Call `Validation::require_valid_confidence(confidence)?` after `validate_native_expiration` and before `charge_attestation_fee`
    - Set `confidence` field in the `Attestation` struct literal
    - _Requirements: 1.2, 1.3, 2.1, 2.3, 4.1_

  - [ ]* 3.2 Write unit tests for `create_attestation` confidence behavior
    - Test default `None` stores `confidence = 100`
    - Test `confidence = 101` returns `Error::InvalidConfidence` with fee collector balance unchanged (validation before fee)
    - Test boundary values: `confidence = 0`, `confidence = 100`, `confidence = 101`
    - Test all other attestation fields are unaffected when confidence is set
    - _Requirements: 1.2, 1.3, 2.1, 2.3, 4.1_

  - [ ]* 3.3 Write property test for confidence round-trip (Property 1)
    - **Property 1: Confidence round-trip** — for any `c` in `0..=100`, create attestation with that confidence and assert `get_attestation` returns `confidence == c`
    - **Validates: Requirements 1.2, 1.4**

- [ ] 4. Update all other `Attestation` construction sites in `src/lib.rs`
  - Add `confidence: 100` to the `Attestation` struct literal in `import_attestation`
  - Add `confidence: 100` to the `Attestation` struct literal in `bridge_attestation`
  - Add `confidence: 100` to the `Attestation` struct literal in `create_attestations_batch`
  - Add `confidence: 100` to the `Attestation` struct literal in `cosign_attestation` (multisig finalization)
  - _Requirements: 4.1, 4.3_

- [ ] 5. Checkpoint — ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 6. Implement `has_claim_with_min_confidence` in `src/lib.rs`
  - [ ] 6.1 Add the `has_claim_with_min_confidence` public function
    - Signature: `pub fn has_claim_with_min_confidence(env: Env, subject: Address, claim_type: String, min_confidence: u8) -> bool`
    - Iterate subject's attestation IDs via `Storage::get_subject_attestations`
    - For each ID, attempt `Storage::get_attestation`; skip on error (backward-compat fallback: treat missing confidence as 100)
    - Return `true` only when `claim_type` matches, `get_status` returns `AttestationStatus::Valid`, and `attestation.confidence >= min_confidence`
    - Return `false` if no matching attestation is found
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 4.3_

  - [ ]* 6.2 Write unit tests for `has_claim_with_min_confidence`
    - Test returns `true` when confidence meets threshold
    - Test returns `false` when confidence is below threshold
    - Test returns `false` for revoked attestation even with `min_confidence = 0`
    - Test returns `false` for expired attestation even with `min_confidence = 0`
    - Test `has_valid_claim` result is unchanged regardless of confidence value
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.2_

  - [ ]* 6.3 Write property test for threshold query (Property 4)
    - **Property 4: has_claim_with_min_confidence threshold** — for any `(c in 0..=100, m in 0..=100)`, result must equal `c >= m`
    - **Validates: Requirements 3.1, 3.2, 3.3**

  - [ ]* 6.4 Write property test for revoked/expired (Property 5)
    - **Property 5: Revoked or expired attestations never satisfy confidence query** — for any `c in 0..=100`, after revoking or expiring the attestation, `has_claim_with_min_confidence` with `min_confidence = 0` returns `false`
    - **Validates: Requirements 3.4**

  - [ ]* 6.5 Write property test for `has_valid_claim` confidence-agnostic (Property 6)
    - **Property 6: has_valid_claim is confidence-agnostic** — for any two confidence values `a, b in 0..=100`, `has_valid_claim` returns the same result for both
    - **Validates: Requirements 4.2**

- [ ] 7. Add `proptest` dependency to `Cargo.toml`
  - Add `proptest = "1"` under `[dev-dependencies]` in `Cargo.toml`
  - _Requirements: (testing infrastructure)_

- [ ] 8. Final checkpoint — ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Property tests use `proptest = "1"` and run a minimum of 100 iterations each
- Each property test must include a comment: `// Feature: attestation-confidence-score, Property N: <property text>`
- The `confidence` field is appended last to `Attestation` to preserve XDR positional encoding for existing records
- Old stored records without a `confidence` field are treated as confidence `100` via the error-fallback in `has_claim_with_min_confidence`
