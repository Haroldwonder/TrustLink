# Requirements Document

## Introduction

Different verification methods carry different levels of assurance. A biometric check is more reliable than a self-attested claim; a hardware-backed signature is stronger than an email confirmation. This feature lets issuers attach a numeric confidence score (0–100) to each attestation so that consumers can filter by minimum confidence when querying claims.

The feature adds a `confidence` field to the `Attestation` struct, threads it through `create_attestation`, validates the range, and exposes a new query function `has_claim_with_min_confidence` that returns `true` only when a valid attestation of the requested type meets or exceeds the caller's minimum threshold.

## Glossary

- **Attestation**: An on-chain record created by a registered issuer asserting a claim about a subject.
- **Confidence_Score**: An integer in the range [0, 100] representing the issuer's assurance level for an attestation. 100 means maximum confidence; 0 means minimal confidence.
- **Issuer**: A registered address authorized to create attestations.
- **Subject**: The address an attestation is about.
- **Claim_Type**: A string identifier categorizing the nature of an attestation (e.g., `"KYC_PASSED"`).
- **TrustLink**: The smart contract that stores and queries attestations.
- **Validator**: The input-validation logic within TrustLink that enforces field constraints.

## Requirements

### Requirement 1: Confidence Score Field on Attestation

**User Story:** As an issuer, I want to attach a confidence score to each attestation I create, so that consumers can distinguish high-assurance claims from low-assurance ones.

#### Acceptance Criteria

1. THE `Attestation` struct SHALL include a `confidence` field of type `u8`.
2. WHEN `create_attestation` is called with a `confidence` value, THE TrustLink SHALL store that value in the resulting `Attestation`.
3. WHEN `create_attestation` is called without specifying a `confidence` value (i.e., `None`), THE TrustLink SHALL default the stored `confidence` to `100`.
4. WHEN `get_attestation` is called for an existing attestation, THE TrustLink SHALL return the `confidence` value that was stored at creation time.

### Requirement 2: Confidence Score Validation

**User Story:** As a contract operator, I want invalid confidence values to be rejected at creation time, so that the stored data is always well-formed.

#### Acceptance Criteria

1. WHEN `create_attestation` is called with a `confidence` value greater than `100`, THE Validator SHALL return `Error::InvalidConfidence`.
2. WHEN `create_attestation` is called with a `confidence` value in the range [0, 100], THE TrustLink SHALL accept the attestation and proceed normally.
3. THE Validator SHALL enforce the confidence range check before any fee is charged or storage is written.

### Requirement 3: Minimum-Confidence Claim Query

**User Story:** As a verifier, I want to query whether a subject holds a valid claim of a given type with at least a specified confidence level, so that I can enforce assurance thresholds in my application.

#### Acceptance Criteria

1. WHEN `has_claim_with_min_confidence(env, subject, claim_type, min_confidence)` is called and the subject has a valid, non-revoked, non-expired attestation of `claim_type` whose `confidence` is greater than or equal to `min_confidence`, THE TrustLink SHALL return `true`.
2. WHEN `has_claim_with_min_confidence` is called and no valid attestation of `claim_type` meets the minimum confidence threshold, THE TrustLink SHALL return `false`.
3. WHEN `has_claim_with_min_confidence` is called and the subject has a valid attestation of `claim_type` but its `confidence` is less than `min_confidence`, THE TrustLink SHALL return `false`.
4. WHEN `has_claim_with_min_confidence` is called and the matching attestation is revoked or expired, THE TrustLink SHALL return `false` regardless of the stored confidence value.
5. THE `has_claim_with_min_confidence` function SHALL accept `min_confidence` as a `u8` parameter.

### Requirement 4: Backward Compatibility

**User Story:** As a developer integrating TrustLink, I want existing attestation creation paths to continue working without modification, so that I do not need to update callers that do not care about confidence scores.

#### Acceptance Criteria

1. WHEN `create_attestation` is called without a `confidence` argument (using the default), THE TrustLink SHALL behave identically to the pre-feature behavior for all other fields.
2. THE `has_valid_claim` function SHALL continue to return results independent of confidence score, preserving existing semantics.
3. FOR ALL attestations created before this feature is deployed, THE TrustLink SHALL treat the absence of a stored confidence value as equivalent to a confidence of `100` when queried via `has_claim_with_min_confidence`.
