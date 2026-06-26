# Implementation Plan: Expiration Warning Query

## Overview

Add two read-only query functions to `TrustLinkContract` in `src/lib.rs`. Both functions reuse existing storage helpers — no new storage keys, types, or events are needed. The implementation follows the same inline-filter pattern used by `get_valid_claims` and `has_valid_claim`.

## Tasks

- [x] 1. Implement `get_expiring_attestations` in `src/lib.rs`
  - Add the method to the `#[contractimpl]` block for `TrustLinkContract`
  - Retrieve the subject's attestation index via `Storage::get_subject_attestations`
  - Compute `current_time` from `env.ledger().timestamp()`
  - Compute `upper_bound` using `current_time.saturating_add(within_seconds)`
  - For each ID, call `Storage::get_attestation`; skip `Err` results silently
  - Include the ID only when `!revoked`, `expiration == Some(exp)`, `exp > current_time`, and `exp <= upper_bound`
  - Return `Vec<String>` of matching IDs
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 3.1, 3.2, 3.3, 3.4_

- [x] 2. Implement `get_issuer_expiring_attestations` in `src/lib.rs`
  - Add the method directly after `get_expiring_attestations` in the same `#[contractimpl]` block
  - Identical logic to task 1 but retrieves the index via `Storage::get_issuer_attestations`
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 3.4_

- [x] 3. Write unit tests in `src/test.rs`
  - [x] 3.1 Unit tests for `get_expiring_attestations`
    - Subject with no attestations → empty result
    - All attestations outside the window → empty result
    - Mix of in-window and out-of-window attestations → only in-window IDs returned
    - Attestation with `expiration = None` → excluded
    - Attestation with `revoked = true` → excluded
    - Boundary: `expiration = current_time + within_seconds` → included
    - Boundary: `expiration = current_time + within_seconds + 1` → excluded
    - Boundary: `expiration = current_time` → excluded (already expired)
    - `within_seconds = 0` → empty result
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 3.1, 3.2, 3.3, 3.4_

  - [x] 3.2 Unit tests for `get_issuer_expiring_attestations`
    - Mirror the subject-scoped cases above using the issuer index
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4_

- [x] 4. Checkpoint — ensure all unit tests pass
  - Run `cargo test` and confirm all tests in `src/test.rs` pass; ask the user if questions arise.

- [ ] 5. Write property-based tests in `tests/integration_test.rs`
  - Add `proptest` to `[dev-dependencies]` in `Cargo.toml` if not already present
  - Each test is tagged with `// Feature: expiration-warning-query, Property N: <text>`

  - [ ]* 5.1 Write property test for subject-scoped filter correctness (Property 1)
    - **Property 1: Subject-scoped filter correctness**
    - Generate a random subject, random `within_seconds`, and a random set of attestations with varied `expiration` and `revoked` values; store them; call `get_expiring_attestations`; for each returned ID assert `revoked = false` and `exp` in `(current_time, current_time + within_seconds]`; also assert no qualifying attestation is absent from the result
    - **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 4.3**

  - [ ]* 5.2 Write property test for issuer-scoped filter correctness (Property 2)
    - **Property 2: Issuer-scoped filter correctness**
    - Same structure as 5.1 but using `get_issuer_expiring_attestations` and the issuer index
    - **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 4.4**

  - [ ]* 5.3 Write property test for zero-window always returns empty (Property 3)
    - **Property 3: Zero window always returns empty**
    - Generate a random subject or issuer with any set of attestations; call the query with `within_seconds = 0`; assert the result is empty
    - **Validates: Requirements 3.1**

  - [ ]* 5.4 Write property test for query idempotence (Property 4)
    - **Property 4: Query idempotence**
    - Generate a random subject or issuer and `within_seconds`; call the same query twice without any state change between calls; assert both results are identical
    - **Validates: Requirements 4.1, 4.2**

- [x] 6. Final checkpoint — ensure all tests pass
  - Run `cargo test` and confirm all unit and property tests pass; ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- The saturating add (`current_time.saturating_add(within_seconds)`) prevents theoretical `u64` overflow without panicking
- Stale index entries (where `get_attestation` returns `Err`) are silently skipped, consistent with `has_valid_claim`
- Property tests require generators that include edge cases: `expiration = None`, `revoked = true`, `expiration = current_time`, `expiration = current_time + within_seconds`, and `within_seconds = 0`
