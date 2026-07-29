# Requirements Document

## Introduction

The `create_attestation` function in the TrustLink Soroban smart contract currently constructs `Attestation` structs inline using struct literal syntax. As the feature set grows (expiration, metadata, tags, valid_from, bridging, etc.), this approach becomes harder to read and maintain. This feature introduces an `AttestationBuilder` struct in `types.rs` that encapsulates optional field construction behind a fluent builder API. The external `create_attestation` contract function signature remains unchanged; the builder is an internal implementation detail.

## Glossary

- **AttestationBuilder**: A struct in `types.rs` that accumulates optional `Attestation` fields and produces a final `Attestation` via a `build()` method.
- **Attestation**: The existing `#[contracttype]` struct representing a trust claim stored on-chain.
- **Builder**: The `AttestationBuilder` instance under construction before `build()` is called.
- **Fluent interface**: A method-chaining style where each setter returns `Self`, allowing calls to be chained.
- **External API**: The public `create_attestation` function exposed by `TrustLinkContract` via `#[contractimpl]`.
- **Internal construction**: Any place inside `lib.rs` or `types.rs` where an `Attestation` struct literal is assembled.

## Requirements

### Requirement 1: AttestationBuilder Struct

**User Story:** As a contract developer, I want an `AttestationBuilder` struct, so that I can construct `Attestation` values without specifying every optional field as a positional argument.

#### Acceptance Criteria

1. THE `AttestationBuilder` SHALL be defined in `src/types.rs` with fields covering all optional `Attestation` properties: `expiration`, `metadata`, `valid_from`, `tags`, `imported`, `bridged`, `source_chain`, `source_tx`, and `revocation_reason`.
2. THE `AttestationBuilder` SHALL provide a `new()` associated function that initialises all optional fields to `None` and all boolean fields to `false`.
3. WHEN `AttestationBuilder::new()` is called, THE `AttestationBuilder` SHALL return a builder with `imported` set to `false`, `bridged` set to `false`, and `revoked` set to `false`.

### Requirement 2: Builder Setter Methods

**User Story:** As a contract developer, I want fluent setter methods on `AttestationBuilder`, so that I can set only the fields relevant to a given attestation type without boilerplate.

#### Acceptance Criteria

1. THE `AttestationBuilder` SHALL provide a `with_expiration(expiration: Option<u64>) -> Self` method that sets the `expiration` field and returns the builder.
2. THE `AttestationBuilder` SHALL provide a `with_metadata(metadata: Option<String>) -> Self` method that sets the `metadata` field and returns the builder.
3. THE `AttestationBuilder` SHALL provide a `with_valid_from(valid_from: Option<u64>) -> Self` method that sets the `valid_from` field and returns the builder.
4. THE `AttestationBuilder` SHALL provide a `with_tags(tags: Option<Vec<String>>) -> Self` method that sets the `tags` field and returns the builder.
5. THE `AttestationBuilder` SHALL provide a `with_imported(imported: bool) -> Self` method that sets the `imported` field and returns the builder.
6. THE `AttestationBuilder` SHALL provide a `with_bridged(source_chain: String, source_tx: String) -> Self` method that sets `bridged` to `true`, `source_chain`, and `source_tx`, and returns the builder.
7. WHEN setter methods are chained in any order, THE `AttestationBuilder` SHALL produce the same `Attestation` field values as when the same setters are called individually.

### Requirement 3: build() Method

**User Story:** As a contract developer, I want a `build()` method on `AttestationBuilder`, so that I can produce a complete `Attestation` from the accumulated fields and the required runtime parameters.

#### Acceptance Criteria

1. THE `AttestationBuilder` SHALL provide a `build(env: &Env, issuer: Address, subject: Address, claim_type: String, timestamp: u64) -> Attestation` method.
2. WHEN `build()` is called, THE `AttestationBuilder` SHALL generate the attestation `id` using `Attestation::generate_id` with the provided `issuer`, `subject`, `claim_type`, and `timestamp`.
3. WHEN `build()` is called, THE `AttestationBuilder` SHALL set `revoked` to `false` on the resulting `Attestation`.
4. WHEN `build()` is called, THE `AttestationBuilder` SHALL transfer all accumulated optional fields (`expiration`, `metadata`, `valid_from`, `tags`, `imported`, `bridged`, `source_chain`, `source_tx`, `revocation_reason`) to the resulting `Attestation`.

### Requirement 4: Internal Adoption in create_attestation

**User Story:** As a contract developer, I want `create_attestation` to use `AttestationBuilder` internally, so that the construction logic is centralised and the function body is more readable.

#### Acceptance Criteria

1. WHEN `create_attestation` constructs an `Attestation`, THE `TrustLinkContract` SHALL use `AttestationBuilder::new()` followed by the relevant setter methods and `build()` instead of an inline struct literal.
2. THE external signature of `create_attestation(env, issuer, subject, claim_type, expiration, metadata, tags)` SHALL remain unchanged after the refactor.
3. WHEN `create_attestation` is called with the same arguments before and after the refactor, THE `TrustLinkContract` SHALL produce an `Attestation` with identical field values.

### Requirement 5: Unit Tests for AttestationBuilder

**User Story:** As a contract developer, I want unit tests for `AttestationBuilder`, so that I can verify builder correctness independently of the full contract flow.

#### Acceptance Criteria

1. THE test suite SHALL include a test verifying that `AttestationBuilder::new().build(...)` produces an `Attestation` with `revoked = false`, `imported = false`, `bridged = false`, and all optional fields set to `None`.
2. THE test suite SHALL include a test verifying that `with_expiration` and `with_metadata` set the corresponding fields on the built `Attestation`.
3. THE test suite SHALL include a test verifying that calling `with_expiration(x).with_metadata(y)` produces the same `Attestation` fields as calling `with_metadata(y).with_expiration(x)` (order-independence / confluence property).
4. THE test suite SHALL include a test verifying that all existing `create_attestation` tests continue to pass after the refactor.
