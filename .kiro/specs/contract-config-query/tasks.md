# Implementation Plan: contract-config-query

## Overview

Purely additive changes across three files: add `ContractConfig` to `types.rs`, add `Storage::get_ttl_config()` to `storage.rs`, and wire up the `get_config()` entry point in `lib.rs`. Unit and property tests live in `test.rs`.

## Tasks

- [x] 1. Add `ContractConfig` struct to `src/types.rs`
  - Add `#[contracttype] #[derive(Clone, Debug, Eq, PartialEq)]` struct with fields: `ttl_config: TtlConfig`, `fee_config: FeeConfig`, `contract_name: String`, `contract_version: String`, `contract_description: String`
  - Reuse existing `TtlConfig` and `FeeConfig` types — no redefinition
  - _Requirements: 1.1, 1.2_

- [x] 2. Add `Storage::get_ttl_config()` helper to `src/storage.rs`
  - [x] 2.1 Implement `pub fn get_ttl_config(env: &Env) -> Option<TtlConfig>`
    - Read `StorageKey::TtlConfig` from instance storage, mirroring the existing `get_fee_config` pattern
    - _Requirements: 2.1, 3.1_

  - [ ]* 2.2 Write property test for `Storage::get_ttl_config` consistency
    - **Property 1: get_config is consistent with individual query functions**
    - **Validates: Requirements 3.1**

- [x] 3. Implement `get_config(env: Env) -> ContractConfig` in `src/lib.rs`
  - [x] 3.1 Add the entry point to `TrustLinkContract`
    - Call `Storage::get_ttl_config`, `Storage::get_fee_config`, `Storage::get_version` with `unwrap_or`/`unwrap_or_else` defaults as specified in the design
    - Hardcode `contract_name = "TrustLink"` and `contract_description = "On-chain attestation and verification system for the Stellar blockchain."`
    - Function must be infallible (`-> ContractConfig`, not `Result`)
    - _Requirements: 2.1, 2.2, 2.3, 2.4_

  - [ ]* 3.2 Write property test for idempotence (Property 2)
    - **Property 2: get_config is idempotent and does not mutate state**
    - Generate random initialized contract states; call `get_config` twice; assert structural equality of both results
    - Tag: `// Feature: contract-config-query, Property 2: get_config is idempotent and does not mutate state`
    - **Validates: Requirements 2.2, 4.1**

  - [ ]* 3.3 Write property test for round-trip through initialization (Property 3)
    - **Property 3: Initialized values round-trip through get_config**
    - Generate random `(ttl_days: u32, fee: i128, collector: Address)` tuples; initialize; assert `get_config` fields match inputs
    - Tag: `// Feature: contract-config-query, Property 3: Initialized values round-trip through get_config`
    - **Validates: Requirements 2.1, 2.3**

- [x] 4. Write unit tests in `src/test.rs`
  - [x] 4.1 Test uninitialized defaults
    - Call `get_config` on a fresh contract; assert `ttl_days == 30`, `attestation_fee == 0`, `fee_token == None`, `contract_version == ""`, and hardcoded name/description strings
    - _Requirements: 2.4_

  - [x] 4.2 Test post-initialization values
    - Initialize with specific `ttl_days`, fee, and collector; call `get_config`; assert all fields match the initialization inputs
    - _Requirements: 2.1, 2.3_

  - [x] 4.3 Test consistency with individual query functions
    - After initialization, assert `get_config().fee_config == get_fee_config()`, `get_config().ttl_config == Storage::get_ttl_config()`, and metadata fields match `get_contract_metadata()`
    - _Requirements: 3.1, 3.2, 3.3_

  - [ ]* 4.4 Write property test for consistency with individual queries (Property 1)
    - Generate random `(ttl_days: u32, fee: i128 >= 0, collector: Address)` tuples; initialize; call `get_config` and individual query functions; assert field-by-field equality
    - Tag: `// Feature: contract-config-query, Property 1: get_config is consistent with individual query functions`
    - **Validates: Requirements 3.1, 3.2, 3.3**

- [x] 5. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Property tests use the `proptest` crate with a minimum of 100 iterations per property
- Each property test must include the tag comment in the format specified in the design
- All defaults are defined in the design's data model table — refer to it during implementation
