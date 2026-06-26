# Implementation Plan: Admin Transfer

## Overview

Add `transfer_admin` to the TrustLink contract by wiring together the existing `Validation`, `Storage`, and `Events` modules. No new files or storage keys are needed — only `src/lib.rs`, `src/events.rs`, and `src/test.rs` require changes.

## Tasks

- [x] 1. Add `Events::admin_transferred` to src/events.rs
  - Add a new `admin_transferred(env: &Env, old_admin: &Address, new_admin: &Address)` method to the `Events` impl block
  - Publish with topic `(symbol_short!("adm_xfer"),)` and data `(old_admin.clone(), new_admin.clone())`
  - Follow the same style as the existing `admin_initialized` event
  - _Requirements: 4.1, 4.2_

- [x] 2. Implement `transfer_admin` in src/lib.rs
  - [x] 2.1 Add the `transfer_admin` function to the `TrustLinkContract` impl block
    - Signature: `pub fn transfer_admin(env: Env, current_admin: Address, new_admin: Address) -> Result<(), Error>`
    - Call `current_admin.require_auth()`
    - Call `Validation::require_admin(&env, &current_admin)?`
    - Call `Storage::set_admin(&env, &new_admin)`
    - Call `Events::admin_transferred(&env, &current_admin, &new_admin)`
    - Return `Ok(())`
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

  - [ ]* 2.2 Write property test: non-admin cannot transfer (Property 1)
    - `test_transfer_admin_unauthorized`: generate a random non-admin address, call `transfer_admin` with it, assert result is `Error::Unauthorized`
    - Comment: `// Property 1: Non-admin cannot transfer — Validates: Requirements 2.1`
    - _Requirements: 2.1_

  - [ ]* 2.3 Write property test: admin address updated after transfer (Property 2)
    - `test_transfer_admin_success`: call `transfer_admin` with valid admin and a new address, assert `get_admin()` returns the new address
    - Comment: `// Property 2: Admin address updated after transfer — Validates: Requirements 1.3`
    - _Requirements: 1.3_

  - [ ]* 2.4 Write property test: privilege handoff is complete and immediate (Property 3)
    - `test_transfer_admin_old_admin_loses_privileges`: after transfer, call `register_issuer` with old admin, assert `Error::Unauthorized`
    - `test_transfer_admin_new_admin_can_register_issuer`: after transfer, call `register_issuer` with new admin, assert success
    - Comment: `// Property 3: Privilege handoff — Validates: Requirements 3.1, 3.2`
    - _Requirements: 3.1, 3.2_

  - [ ]* 2.5 Write property test: exactly one event with correct data (Property 4)
    - `test_transfer_admin_emits_event`: call `transfer_admin`, inspect `env.events().all()`, assert exactly one `adm_xfer` event exists and its data contains old and new admin addresses
    - Comment: `// Property 4: Event emission — Validates: Requirements 1.4, 4.1, 4.2`
    - _Requirements: 1.4, 4.1, 4.2_

  - [ ]* 2.6 Write edge-case test: uninitialized contract (Edge Case)
    - `test_transfer_admin_not_initialized`: create a fresh uninitialized contract, call `transfer_admin`, assert `Error::NotInitialized`
    - Comment: `// Edge Case: Uninitialized contract — Validates: Requirements 2.2`
    - _Requirements: 2.2_

- [x] 3. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.
