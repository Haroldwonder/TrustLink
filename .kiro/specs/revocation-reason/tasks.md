# Implementation Plan: Revocation Reason

## Overview

Add an optional `revocation_reason: Option<String>` field to the `Attestation` struct, wire it through the revocation entry points, emit it in the `AttestationRevoked` event, and enforce a 128-character length limit. All changes are backward-compatible — callers that pass `None` behave exactly as before.

## Tasks

- [x] 1. Extend `Attestation` struct and `Error` enum in `src/types.rs`
  - Add `pub revocation_reason: Option<String>` as the last field of the `Attestation` struct
  - Add `ReasonTooLong = 21` variant to the `Error` enum
  - _Requirements: 1.1, 3.1_

- [x] 2. Initialise `revocation_reason` to `None` at all construction sites in `src/lib.rs`
  - Set `revocation_reason: None` in `create_attestation`
  - Set `revocation_reason: None` in `import_attestation`
  - Set `revocation_reason: None` in `bridge_attestation`
  - Set `revocation_reason: None` in `create_attestations_batch` (per-subject struct literal)
  - Set `revocation_reason: None` in `cosign_attestation` (multisig activation struct literal)
  - _Requirements: 1.2_

- [x] 3. Update `Events::attestation_revoked` in `src/events.rs`
  - Change signature to `pub fn attestation_revoked(env: &Env, attestation_id: &String, issuer: &Address, reason: &Option<String>)`
  - Change published data from `attestation_id.clone()` to `(attestation_id.clone(), reason.clone())`
  - _Requirements: 2.1, 2.2_

- [x] 4. Add `validate_reason` helper and update `revoke_attestation` in `src/lib.rs`
  - [x] 4.1 Add `fn validate_reason(reason: &Option<String>) -> Result<(), Error>` that returns `Err(Error::ReasonTooLong)` when `reason.len() > 128`
    - _Requirements: 3.1, 3.3_
  - [x] 4.2 Add `reason: Option<String>` parameter to `revoke_attestation`
    - Call `validate_reason(&reason)?` before mutating the attestation
    - Set `attestation.revocation_reason = reason.clone()` before storing
    - Pass `&reason` to `Events::attestation_revoked`
    - _Requirements: 1.3, 1.4, 3.1, 4.1, 4.2_
  - [ ]* 4.3 Write property test for `validate_reason` length boundary
    - **Property 1: Reason length boundary — strings of length ≤ 128 always accepted; strings of length > 128 always rejected**
    - **Validates: Requirements 3.1, 3.3**

- [x] 5. Update `revoke_attestations_batch` in `src/lib.rs`
  - Add `reason: Option<String>` parameter
  - Call `validate_reason(&reason)?` once before the loop (atomic all-or-nothing)
  - Set `attestation.revocation_reason = reason.clone()` inside the loop before storing
  - Pass `&reason` to `Events::attestation_revoked` inside the loop
  - _Requirements: 1.4, 2.1, 3.2_

- [x] 6. Checkpoint — ensure the project compiles with no errors
  - Run `cargo build` and confirm zero compilation errors before writing tests
  - Ask the user if any questions arise

- [-] 7. Write unit tests in `src/test.rs`
  - [x] 7.1 `test_revoke_with_reason_stores_reason` — revoke with a non-`None` reason and assert `get_attestation(...).revocation_reason` equals the supplied value
    - _Requirements: 5.1, 1.4_
  - [ ] 7.2 `test_revoke_without_reason_stores_none` — revoke with `reason: None` and assert `revocation_reason` is `None`
    - _Requirements: 5.2, 1.3_
  - [~] 7.3 `test_revoke_reason_too_long_rejected` — supply a 129-character reason and assert `Err(Ok(Error::ReasonTooLong))` is returned
    - _Requirements: 5.3, 3.1_
  - [~] 7.4 `test_revoke_reason_exactly_128_chars_accepted` — supply a 128-character reason and assert the revocation succeeds and the reason is stored
    - _Requirements: 5.4, 3.3_
  - [ ]* 7.5 Write property test for `revoke_attestation` reason round-trip
    - **Property 2: Round-trip consistency — any reason string of length ≤ 128 supplied to `revoke_attestation` is retrievable unchanged from `get_attestation`**
    - **Validates: Requirements 1.4, 5.1**

- [~] 8. Final checkpoint — ensure all tests pass
  - Run `cargo test` and confirm all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- The `revocation_reason` field is appended last on `Attestation` to minimise XDR disruption; existing stored attestations deserialise with `revocation_reason = None` automatically — no migration needed
- `validate_reason` must be called once before the batch loop so that an invalid reason causes zero attestations to be revoked (atomic behaviour per Requirement 3.2)
- Property tests validate universal correctness properties; unit tests validate specific examples and edge cases
