# Implementation Plan: Attestation Templates

## Overview

Add reusable per-issuer attestation blueprints to TrustLink. The feature is purely additive: new storage keys, a new type, a new event, and four new contract entry points. All changes slot into the existing layered architecture (`types.rs` → `storage.rs` → `events.rs` → `lib.rs`).

## Tasks

- [x] 1. Extend types: `AttestationTemplate` struct and `InvalidClaimType` error
  - Add `AttestationTemplate` struct with `claim_type: String`, `default_expiration_days: Option<u32>`, `metadata_template: Option<String>` to `src/types.rs`
  - Add `InvalidClaimType = 21` variant to the `Error` enum in `src/types.rs`
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 2. Extend storage: template keys and helper methods
  - [x] 2.1 Add `Template(Address, String)` and `TemplateRegistry(Address)` variants to `StorageKey` in `src/storage.rs`
    - Both use persistent storage tier with TTL refreshed via `get_ttl_lifetime`
    - _Requirements: 6.1_

  - [x] 2.2 Implement storage helper methods on `Storage` in `src/storage.rs`
    - `set_template`, `get_template`, `has_template`, `get_template_registry`, `add_to_template_registry`
    - `add_to_template_registry` must only append if the ID is not already present (preserves insertion order, no duplicates)
    - _Requirements: 2.1, 2.6, 4.1, 4.3, 6.1_

  - [ ]* 2.3 Write property test for template round-trip (Property 1)
    - **Property 1: Template round-trip**
    - **Validates: Requirements 2.1, 5.1**

  - [ ]* 2.4 Write property test for template registry insertion order (Property 6)
    - **Property 6: Template registry insertion order**
    - **Validates: Requirements 2.6, 4.1, 4.2, 4.3**

- [x] 3. Add `template_created` event to `src/events.rs`
  - Implement `Events::template_created(env, issuer, template_id)`
  - Event shape: `topics: (symbol_short!("tmpl_crt"), issuer)`, `data: template_id`
  - _Requirements: 7.1_

- [x] 4. Implement `create_template` entry point in `src/lib.rs`
  - Add `pub fn create_template(env, issuer, template_id, template) -> Result<(), Error>`
  - Validation order: `require_auth` → `require_issuer` → non-empty `claim_type` → `metadata_template` ≤ 256 bytes
  - On success: `Storage::set_template`, `Storage::add_to_template_registry` (if new), `Events::template_created`
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

  - [ ]* 4.1 Write property test for metadata length boundary (Property 2)
    - **Property 2: Metadata length validation**
    - **Validates: Requirements 1.2, 2.4, 3.9**

  - [ ]* 4.2 Write property test for empty claim_type rejection (Property 3)
    - **Property 3: Empty claim_type rejected**
    - **Validates: Requirements 1.3, 2.5**

  - [ ]* 4.3 Write property test for template overwrite (Property 4)
    - **Property 4: Template overwrite**
    - **Validates: Requirements 2.2**

  - [ ]* 4.4 Write property test for non-issuer Unauthorized (Property 5)
    - **Property 5: Non-issuer gets Unauthorized**
    - **Validates: Requirements 2.3, 3.7**

  - [ ]* 4.5 Write property test for template_created event (Property 13)
    - **Property 13: template_created event emitted on success**
    - **Validates: Requirements 7.1**

- [x] 5. Implement `get_template` and `list_templates` entry points in `src/lib.rs`
  - `pub fn get_template(env, issuer, template_id) -> Result<AttestationTemplate, Error>` — returns `NotFound` if absent
  - `pub fn list_templates(env, issuer) -> Vec<String>` — delegates to `Storage::get_template_registry`; returns empty vec if no templates
  - _Requirements: 4.1, 4.2, 4.3, 5.1, 5.2_

  - [ ]* 5.1 Write property test for missing template NotFound (Property 9)
    - **Property 9: Missing template returns NotFound**
    - **Validates: Requirements 3.6, 5.2**

  - [ ]* 5.2 Write property test for template storage isolation (Property 12)
    - **Property 12: Template storage isolation across issuers**
    - **Validates: Requirements 6.1, 6.2**

- [x] 6. Checkpoint — ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 7. Implement `create_attestation_from_template` entry point in `src/lib.rs`
  - Add `pub fn create_attestation_from_template(env, issuer, template_id, subject, expiration_override: Option<u64>, metadata_override: Option<String>) -> Result<String, Error>`
  - Validation order: `require_auth` → `require_issuer` → load template (`NotFound`) → `metadata_override` ≤ 256 bytes → `expiration_override` > current timestamp (`InvalidExpiration`)
  - Expiration resolution: override wins; else `current_timestamp + (default_expiration_days * 86_400)`; else `None`
  - Metadata resolution: override wins; else `template.metadata_template`
  - Delegate to existing `store_attestation` helper and `Events::attestation_created`
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10_

  - [ ]* 7.1 Write property test for attestation fields matching template defaults (Property 7)
    - **Property 7: Attestation fields match template defaults**
    - **Validates: Requirements 3.1, 3.2, 3.3**

  - [ ]* 7.2 Write property test for overrides taking precedence (Property 8)
    - **Property 8: Overrides take precedence over template defaults**
    - **Validates: Requirements 3.4, 3.5**

  - [ ]* 7.3 Write property test for invalid expiration override (Property 10)
    - **Property 10: Invalid expiration override rejected**
    - **Validates: Requirements 3.8**

  - [ ]* 7.4 Write property test for attestation indexed like regular attestation (Property 11)
    - **Property 11: Attestation from template is indexed like a regular attestation**
    - **Validates: Requirements 3.10**

  - [ ]* 7.5 Write property test for attestation_created event on template instantiation (Property 14)
    - **Property 14: attestation_created event emitted on template instantiation**
    - **Validates: Requirements 7.2**

- [x] 8. Write unit tests in `src/test.rs`
  - `create_template` happy path: create then `get_template` returns same struct
  - `create_template` overwrite: second values win
  - `create_template` with empty `claim_type` → `InvalidClaimType`
  - `create_template` with 257-byte `metadata_template` → `MetadataTooLong`
  - `create_template` from non-issuer → `Unauthorized`
  - `list_templates` returns IDs in insertion order; overwrite does not duplicate
  - `list_templates` for issuer with no templates returns empty vec
  - `get_template` for unknown ID → `NotFound`
  - `create_attestation_from_template` happy path: verify `claim_type`, `metadata`, computed expiration
  - `create_attestation_from_template` with `expiration_override` and `metadata_override`
  - `create_attestation_from_template` with unknown `template_id` → `NotFound`
  - `create_attestation_from_template` with stale `expiration_override` → `InvalidExpiration`
  - `create_attestation_from_template` with 257-byte `metadata_override` → `MetadataTooLong`
  - Two issuers with same `template_id` store and retrieve independently
  - `template_created` event present after `create_template`
  - `attestation_created` event present after `create_attestation_from_template`
  - _Requirements: 1.1, 1.2, 1.3, 2.1–2.6, 3.1–3.10, 4.1–4.3, 5.1–5.2, 6.1–6.2, 7.1–7.2_

- [x] 9. Final checkpoint — ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Property tests go in `tests/integration_test.rs` or a dedicated `tests/template_property_tests.rs` using `proptest`; each must run ≥ 100 iterations and include a comment `// Feature: attestation-templates, Property N: <text>`
- All new storage keys use the persistent tier with TTL refreshed via the existing `get_ttl_lifetime` helper
- `create_attestation_from_template` reuses the existing `store_attestation` helper — no new indexing logic needed
