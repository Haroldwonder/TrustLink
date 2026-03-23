# Design Document: has_any_claim

## Overview

`has_any_claim` is a new read-only query function added to the TrustLink Soroban smart contract. It extends the existing single-claim verification pattern of `has_valid_claim` to support OR-logic across a list of claim types, returning `true` as soon as any one of them resolves to a valid attestation for the given subject.

The function is a pure query — it reads from persistent storage but writes nothing and emits no events. It follows the same short-circuit pattern used internally by `has_valid_claim`, stopping iteration the moment a match is found to minimize compute budget consumption on Stellar.

## Architecture

The function fits entirely within the existing contract architecture. No new storage keys, no new types, and no new modules are required.

```
Caller
  │
  ▼
TrustLinkContract::has_any_claim(env, subject, claim_types)
  │
  ├─► Storage::get_subject_attestations(&env, &subject)
  │       returns Vec<String> of attestation IDs
  │
  └─► for each claim_type in claim_types:
        for each id in attestation_ids:
          Storage::get_attestation(&env, &id)
            └─► attestation.get_status(current_time) == Valid → return true
      return false
```

The outer loop iterates over `claim_types`; the inner loop iterates over the subject's attestation IDs. On the first `Valid` match the function returns immediately.

## Components and Interfaces

### Function Signature

```rust
pub fn has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool
```

| Parameter     | Type          | Description                                      |
|---------------|---------------|--------------------------------------------------|
| `env`         | `Env`         | Soroban environment (ledger time, storage)       |
| `subject`     | `Address`     | The address whose attestations are queried       |
| `claim_types` | `Vec<String>` | One or more claim type identifiers to check      |

Returns `bool` — `true` if at least one valid attestation exists for any listed claim type, `false` otherwise.

### Placement

The function is added to the `#[contractimpl]` block in `src/lib.rs`, immediately after `has_valid_claim`, keeping related query functions co-located.

### Dependencies (unchanged)

- `Storage::get_subject_attestations` — retrieves the list of attestation IDs for a subject
- `Storage::get_attestation` — retrieves a single attestation by ID
- `Attestation::get_status` — computes `AttestationStatus` from the attestation fields and current ledger time
- `AttestationStatus::Valid` — the only status that counts as a match

## Data Models

No new data models are introduced. The function operates entirely on existing types:

- `Attestation` (from `src/types.rs`) — holds `claim_type`, `revoked`, `expiration`, `valid_from`
- `AttestationStatus` (from `src/types.rs`) — `Valid | Expired | Revoked | Pending`
- `StorageKey::SubjectAttestations(Address)` — existing index mapping subject → attestation IDs
- `StorageKey::Attestation(String)` — existing key mapping ID → `Attestation`

### Implementation

```rust
/// Check if an address has a valid attestation for any of the given claim types
pub fn has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool {
    if claim_types.is_empty() {
        return false;
    }
    let attestation_ids = Storage::get_subject_attestations(&env, &subject);
    let current_time = env.ledger().timestamp();
    for claim_type in claim_types.iter() {
        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if attestation.claim_type == claim_type {
                    if attestation.get_status(current_time) == AttestationStatus::Valid {
                        return true;
                    }
                }
            }
        }
    }
    false
}
```

The outer loop over `claim_types` and inner loop over `attestation_ids` ensures short-circuit on the first valid match. The empty-list guard at the top satisfies Requirement 1.3 without entering the loop.


## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: OR-logic match returns true

*For any* subject and any non-empty list of claim types where at least one claim type has a valid attestation for that subject, `has_any_claim` shall return `true`.

**Validates: Requirements 1.1**

### Property 2: No match returns false

*For any* subject and any non-empty list of claim types where none of the claim types have a valid attestation for that subject, `has_any_claim` shall return `false`.

**Validates: Requirements 1.2**

### Property 3: Non-valid attestations are excluded

*For any* subject whose attestations for a given claim type are all non-valid (i.e., each has `AttestationStatus` of `Revoked`, `Expired`, or `Pending`), calling `has_any_claim` with a list containing only that claim type shall return `false`.

**Validates: Requirements 3.1, 3.2, 3.3**

### Property 4: Equivalence with has_valid_claim for single-element list

*For any* subject address and any claim type value, calling `has_any_claim` with a single-element list `[claim_type]` shall return the same result as calling `has_valid_claim` with the same subject and claim type.

**Validates: Requirements 4.1**

## Error Handling

`has_any_claim` is infallible — it returns `bool`, not `Result`. The following cases are handled gracefully without panicking:

| Scenario | Behavior |
|---|---|
| Empty `claim_types` list | Returns `false` immediately |
| Subject has no attestations | `get_subject_attestations` returns an empty `Vec`; loop body never executes; returns `false` |
| Attestation ID in index but record missing | `get_attestation` returns `Err(NotFound)`; the `if let Ok(...)` guard skips it silently |
| All attestations non-valid | Loop completes without a match; returns `false` |

No new error variants are needed.

## Testing Strategy

### Unit Tests (in `src/test.rs`)

Unit tests cover concrete scenarios and edge cases:

- Empty `claim_types` list returns `false`
- Single valid attestation in list returns `true`
- Multiple claim types, only one has a valid attestation — returns `true`
- Multiple claim types, none have valid attestations — returns `false`
- Revoked attestation for matching claim type — returns `false`
- Expired attestation for matching claim type — returns `false`
- Pending attestation (future `valid_from`) for matching claim type — returns `false`
- Subject with no attestations at all — returns `false`
- Equivalence with `has_valid_claim` for a concrete single-element list

### Property-Based Tests

Use the [`proptest`](https://github.com/proptest-rs/proptest) crate for property-based testing in Rust.

Each property test runs a minimum of 100 iterations. Each test is tagged with a comment referencing the design property it validates.

**Tag format:** `// Feature: has-any-claim, Property {N}: {property_text}`

**Property 1 test** — generate a random subject, a random set of claim types, ensure at least one has a valid attestation, assert `has_any_claim` returns `true`.

**Property 2 test** — generate a random subject with attestations only for claim types not in the query list, assert `has_any_claim` returns `false`.

**Property 3 test** — generate a random subject with attestations that are all non-valid (randomly mix revoked/expired/pending), assert `has_any_claim` returns `false` for a list containing those claim types.

**Property 4 test** — generate a random subject and random claim type, assert `has_any_claim([claim_type]) == has_valid_claim(claim_type)` for all generated inputs.

Unit tests and property tests are complementary: unit tests pin down concrete behavior and edge cases; property tests verify the general rules hold across the full input space.
