# Requirements Document

## Introduction

When an issuer revokes an attestation on TrustLink, they should be able to supply an optional
human-readable reason code for audit and compliance purposes. The reason is stored on the
`Attestation` struct, emitted in the `AttestationRevoked` event, and is bounded to a maximum
of 128 characters to keep on-chain storage predictable. Existing revocations that omit a reason
must continue to work without modification.

## Glossary

- **TrustLink_Contract**: The Soroban smart contract that manages attestation lifecycle.
- **Attestation**: An on-chain record linking an issuer, a subject, and a claim type.
- **Issuer**: A registered address authorised to create and revoke attestations.
- **Revocation_Reason**: An optional, bounded string supplied by an issuer when revoking an attestation.
- **AttestationRevoked_Event**: The Soroban event emitted by TrustLink_Contract when an attestation is revoked.

## Requirements

### Requirement 1: Store Revocation Reason on Attestation

**User Story:** As an issuer, I want to attach a reason code when revoking an attestation, so that auditors can understand why the attestation was invalidated.

#### Acceptance Criteria

1. THE `Attestation` struct SHALL contain a `revocation_reason` field of type `Option<String>`.
2. WHEN an attestation is created or imported, THE TrustLink_Contract SHALL set `revocation_reason` to `None`.
3. WHEN `revoke_attestation` is called with a `reason` of `None`, THE TrustLink_Contract SHALL store `None` in `revocation_reason`, leaving existing behaviour unchanged.
4. WHEN `revoke_attestation` is called with a `reason` of `Some(value)`, THE TrustLink_Contract SHALL store `value` in the `revocation_reason` field of the revoked `Attestation`.

### Requirement 2: Include Reason in Revocation Event

**User Story:** As an auditor, I want the revocation event to carry the reason code, so that off-chain indexers can record the full revocation context without additional queries.

#### Acceptance Criteria

1. WHEN an attestation is revoked, THE AttestationRevoked_Event SHALL include the `revocation_reason` value (which may be `None`).
2. THE `attestation_revoked` event function SHALL accept a `reason: Option<String>` parameter and publish it alongside the attestation ID and issuer.

### Requirement 3: Enforce Maximum Reason Length

**User Story:** As a contract operator, I want reason strings to be bounded, so that on-chain storage costs remain predictable.

#### Acceptance Criteria

1. WHEN `revoke_attestation` is called with a `reason` whose length exceeds 128 characters, THE TrustLink_Contract SHALL return `Error::ReasonTooLong`.
2. WHEN `revoke_attestations_batch` is called and any supplied `reason` exceeds 128 characters, THE TrustLink_Contract SHALL return `Error::ReasonTooLong` and revoke no attestations in that call.
3. THE TrustLink_Contract SHALL accept a `reason` of exactly 128 characters without error.

### Requirement 4: Backward Compatibility

**User Story:** As an integrator, I want existing revocation calls without a reason to continue working, so that I do not need to update callers immediately.

#### Acceptance Criteria

1. WHEN `revoke_attestation` is called without a `reason` argument (i.e., `reason: None`), THE TrustLink_Contract SHALL revoke the attestation and emit the event with `revocation_reason` set to `None`.
2. THE TrustLink_Contract SHALL NOT require a non-`None` reason to complete a revocation.

### Requirement 5: Unit Test Coverage

**User Story:** As a developer, I want unit tests covering revocation with and without a reason, so that regressions are caught automatically.

#### Acceptance Criteria

1. THE test suite SHALL include a test that revokes an attestation with a non-`None` reason and asserts the stored `revocation_reason` matches the supplied value.
2. THE test suite SHALL include a test that revokes an attestation with `reason: None` and asserts the stored `revocation_reason` is `None`.
3. THE test suite SHALL include a test that supplies a reason exceeding 128 characters and asserts `Error::ReasonTooLong` is returned.
4. THE test suite SHALL include a test that supplies a reason of exactly 128 characters and asserts the revocation succeeds.
