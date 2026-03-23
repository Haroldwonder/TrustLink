# Requirements Document

## Introduction

This feature adds a `has_any_claim` verification function to the TrustLink smart contract. While the existing `has_valid_claim` function checks whether a subject holds a valid attestation for a single claim type, some use cases require OR-logic verification — confirming that a subject holds at least one valid attestation from a set of claim types. `has_any_claim` addresses this by accepting a list of claim types and returning `true` as soon as any one of them resolves to a valid attestation for the given subject.

## Glossary

- **TrustLink_Contract**: The Soroban smart contract that manages attestations and claim verification.
- **Subject**: An `Address` whose attestations are being queried.
- **Claim_Type**: A `String` identifier categorizing the kind of attestation (e.g., `"KYC"`, `"AML"`).
- **Claim_Types_List**: A `Vec<String>` of one or more `Claim_Type` values passed to `has_any_claim`.
- **Attestation**: A record stored on-chain linking an issuer, subject, and claim type, with optional expiration and revocation state.
- **AttestationStatus**: An enum with variants `Valid`, `Expired`, `Revoked`, and `Pending`, computed from the attestation's fields at query time.
- **Valid_Attestation**: An attestation whose `AttestationStatus` is `Valid` at the current ledger timestamp.
- **Short-Circuit**: Stopping iteration and returning immediately upon finding the first matching result.

## Requirements

### Requirement 1: OR-Logic Claim Verification

**User Story:** As a relying party, I want to check whether a subject holds a valid attestation for any one of several claim types, so that I can implement flexible access control without requiring multiple separate contract calls.

#### Acceptance Criteria

1. WHEN `has_any_claim` is called with a `Subject` and a non-empty `Claim_Types_List`, THE `TrustLink_Contract` SHALL return `true` if the `Subject` has at least one `Valid_Attestation` matching any `Claim_Type` in the list.
2. WHEN `has_any_claim` is called with a `Subject` and a non-empty `Claim_Types_List`, THE `TrustLink_Contract` SHALL return `false` if the `Subject` has no `Valid_Attestation` matching any `Claim_Type` in the list.
3. WHEN `has_any_claim` is called with an empty `Claim_Types_List`, THE `TrustLink_Contract` SHALL return `false`.

### Requirement 2: Short-Circuit Evaluation

**User Story:** As a smart contract operator, I want claim verification to stop as soon as a match is found, so that unnecessary storage reads are avoided and compute costs are minimized.

#### Acceptance Criteria

1. WHEN `has_any_claim` iterates over the `Claim_Types_List` and finds a `Valid_Attestation` for a `Claim_Type`, THE `TrustLink_Contract` SHALL return `true` immediately without evaluating remaining entries in the list.

### Requirement 3: Exclusion of Non-Valid Attestations

**User Story:** As a relying party, I want revoked, expired, and pending attestations to be excluded from the check, so that only currently valid credentials are accepted.

#### Acceptance Criteria

1. WHEN an attestation for a matching `Claim_Type` has `AttestationStatus` of `Revoked`, THE `TrustLink_Contract` SHALL NOT count it as a match.
2. WHEN an attestation for a matching `Claim_Type` has `AttestationStatus` of `Expired`, THE `TrustLink_Contract` SHALL NOT count it as a match.
3. WHEN an attestation for a matching `Claim_Type` has `AttestationStatus` of `Pending`, THE `TrustLink_Contract` SHALL NOT count it as a match.

### Requirement 4: Consistency with has_valid_claim

**User Story:** As a developer integrating TrustLink, I want `has_any_claim` with a single-element list to behave identically to `has_valid_claim`, so that the two functions are interchangeable for single-claim checks.

#### Acceptance Criteria

1. FOR ALL `Subject` addresses and `Claim_Type` values, calling `has_any_claim` with a `Claim_Types_List` containing exactly that `Claim_Type` SHALL return the same result as calling `has_valid_claim` with the same `Subject` and `Claim_Type`.

### Requirement 5: Documentation

**User Story:** As a developer, I want `has_any_claim` documented alongside `has_valid_claim` in the README, so that I can understand when to use each function.

#### Acceptance Criteria

1. THE README SHALL document the `has_any_claim` function signature, parameters, return value, and behavior.
2. THE README SHALL include an example demonstrating `has_any_claim` with a multi-element `Claim_Types_List`.
3. THE README SHALL describe the relationship between `has_any_claim` and `has_valid_claim`.
