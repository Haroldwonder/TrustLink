# Implementation Plan: has_any_claim

## Overview

Add the `has_any_claim` function to the TrustLink Soroban smart contract in `src/lib.rs`, immediately after `has_valid_claim`. The function iterates over a list of claim types and returns `true` on the first valid attestation found (short-circuit). Unit tests and property-based tests go in `src/test.rs`. The README is updated to document the new function.

## Tasks

- [x] 1. Implement `has_any_claim` in `src/lib.rs`
  - Add `pub fn has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool` to the `#[contractimpl]` block immediately after `has_valid_claim`
  - Guard against empty `claim_types` by returning `false` immediately
  - Retrieve subject attestation IDs via `Storage::get_subject_attestations`
  - Outer loop over `claim_types`, inner loop over attestation IDs; use `if let Ok(attestation)` guard for missing records
  - Return `true` on first attestation whose `get_status(current_time) == AttestationStatus::Valid` and `claim_type` matches
  - Return `false` after both loops complete with no match
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 3.1, 3.2, 3.3_

- [x] 2. Write unit tests for `has_any_claim` in `src/test.rs`
  - [x] 2.1 Add concrete unit tests covering all specified scenarios
    - Empty `claim_types` list → `false`
    - Single valid attestation in list → `true`
    - Multiple claim types, only one valid → `true`
    - Multiple claim types, none valid → `false`
    - Revoked attestation for matching claim type → `false`
    - Expired attestation for matching claim type → `false`
    - Pending attestation (`valid_from` in future) for matching claim type → `false`
    - Subject with no attestations at all → `false`
    - Single-element list equivalence with `has_valid_claim` (concrete case)
    - _Requirements: 1.1, 1.2, 1.3, 3.1, 3.2, 3.3, 4.1_

  - [ ]* 2.2 Write property test for OR-logic match (Property 1)
    - `// Feature: has-any-claim, Property 1: OR-logic match returns true`
    - Generate random subject and claim type list; ensure at least one has a valid attestation; assert `has_any_claim` returns `true`
    - **Property 1: OR-logic match returns true**
    - **Validates: Requirements 1.1**

  - [ ]* 2.3 Write property test for no-match returns false (Property 2)
    - `// Feature: has-any-claim, Property 2: No match returns false`
    - Generate random subject with attestations only for claim types not in the query list; assert `has_any_claim` returns `false`
    - **Property 2: No match returns false**
    - **Validates: Requirements 1.2**

  - [ ]* 2.4 Write property test for non-valid attestation exclusion (Property 3)
    - `// Feature: has-any-claim, Property 3: Non-valid attestations are excluded`
    - Generate random subject with attestations that are all non-valid (mix of revoked/expired/pending); assert `has_any_claim` returns `false`
    - **Property 3: Non-valid attestations are excluded**
    - **Validates: Requirements 3.1, 3.2, 3.3**

  - [ ]* 2.5 Write property test for equivalence with `has_valid_claim` (Property 4)
    - `// Feature: has-any-claim, Property 4: Equivalence with has_valid_claim for single-element list`
    - Generate random subject and claim type; assert `has_any_claim([claim_type]) == has_valid_claim(claim_type)` for all inputs
    - **Property 4: Equivalence with has_valid_claim for single-element list**
    - **Validates: Requirements 4.1**

- [x] 3. Checkpoint — ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. Update README.md documentation
  - [x] 4.1 Document `has_any_claim` alongside `has_valid_claim`
    - Add function signature, parameters, return value, and behavior description
    - Include a multi-element `claim_types` example
    - Describe the relationship between `has_any_claim` and `has_valid_claim` (OR-logic vs single-claim)
    - _Requirements: 5.1, 5.2, 5.3_

- [x] 5. Final checkpoint — ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Property tests use the `proptest` crate; add it to `[dev-dependencies]` in `Cargo.toml` if not already present
- Each property test is tagged with a comment referencing the design property it validates
- Unit tests and property tests are complementary: unit tests pin concrete behavior, property tests verify general rules
