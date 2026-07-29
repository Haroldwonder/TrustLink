# Requirements Document

## Introduction

TrustLink issuers and subjects need to proactively identify attestations that are approaching expiration so they can renew or replace them before they lapse. This feature adds two query functions to the TrustLink Soroban smart contract: one scoped to a subject's attestations and one scoped to an issuer's attestations. Both return the IDs of attestations whose expiration timestamp falls within a caller-specified time window from the current ledger timestamp, excluding already-expired and revoked attestations.

## Glossary

- **TrustLink**: The Soroban smart contract on the Stellar blockchain that manages attestations.
- **Attestation**: A signed on-chain claim record with an optional expiration timestamp, a revocation flag, and an issuer/subject pair.
- **Subject**: The Stellar address that an attestation is about.
- **Issuer**: The registered Stellar address that created an attestation.
- **Expiration**: The Unix timestamp (seconds) stored in `Attestation.expiration` after which the attestation is considered expired.
- **Warning Window**: The caller-supplied `within_seconds: u64` duration added to the current ledger timestamp to form the upper bound of the expiration search range.
- **Query**: A read-only contract function that does not mutate state.
- **ExpirationQuery**: The logical component responsible for filtering attestation lists by expiration proximity.

---

## Requirements

### Requirement 1: Subject-scoped expiration warning query

**User Story:** As a subject, I want to query which of my attestations are expiring within a given time window, so that I can take action before they lapse.

#### Acceptance Criteria

1. WHEN `get_expiring_attestations(env, subject, within_seconds)` is called, THE ExpirationQuery SHALL return a `Vec<String>` containing the IDs of all attestations for `subject` whose `expiration` value satisfies `current_time < expiration <= current_time + within_seconds`.
2. WHEN an attestation for `subject` has `revoked = true`, THE ExpirationQuery SHALL exclude that attestation from the result.
3. WHEN an attestation for `subject` has `expiration` equal to `None`, THE ExpirationQuery SHALL exclude that attestation from the result.
4. WHEN an attestation for `subject` has `expiration <= current_time` (already expired), THE ExpirationQuery SHALL exclude that attestation from the result.
5. WHEN no attestations for `subject` fall within the warning window, THE ExpirationQuery SHALL return an empty `Vec<String>`.
6. THE ExpirationQuery SHALL NOT mutate any contract state when executing `get_expiring_attestations`.

### Requirement 2: Issuer-scoped expiration warning query

**User Story:** As an issuer, I want to query which attestations I have issued are expiring within a given time window, so that I can notify subjects or issue renewals proactively.

#### Acceptance Criteria

1. WHEN `get_issuer_expiring_attestations(env, issuer, within_seconds)` is called, THE ExpirationQuery SHALL return a `Vec<String>` containing the IDs of all attestations created by `issuer` whose `expiration` value satisfies `current_time < expiration <= current_time + within_seconds`.
2. WHEN an attestation created by `issuer` has `revoked = true`, THE ExpirationQuery SHALL exclude that attestation from the result.
3. WHEN an attestation created by `issuer` has `expiration` equal to `None`, THE ExpirationQuery SHALL exclude that attestation from the result.
4. WHEN an attestation created by `issuer` has `expiration <= current_time` (already expired), THE ExpirationQuery SHALL exclude that attestation from the result.
5. WHEN no attestations created by `issuer` fall within the warning window, THE ExpirationQuery SHALL return an empty `Vec<String>`.
6. THE ExpirationQuery SHALL NOT mutate any contract state when executing `get_issuer_expiring_attestations`.

### Requirement 3: Window boundary correctness

**User Story:** As a developer integrating TrustLink, I want the expiration window boundaries to be precise and consistent, so that I can rely on deterministic query results.

#### Acceptance Criteria

1. WHEN `within_seconds` is `0`, THE ExpirationQuery SHALL return an empty `Vec<String>` because no attestation can satisfy `current_time < expiration <= current_time + 0`.
2. WHEN an attestation's `expiration` equals exactly `current_time + within_seconds`, THE ExpirationQuery SHALL include that attestation in the result (inclusive upper bound).
3. WHEN an attestation's `expiration` equals exactly `current_time + within_seconds + 1`, THE ExpirationQuery SHALL exclude that attestation from the result (strictly outside window).
4. WHEN an attestation's `expiration` equals exactly `current_time`, THE ExpirationQuery SHALL exclude that attestation from the result (already expired, not expiring soon).

### Requirement 4: Round-trip and idempotence properties

**User Story:** As a developer, I want the query results to be stable and consistent with the underlying attestation state, so that repeated calls with the same inputs produce the same outputs.

#### Acceptance Criteria

1. WHEN `get_expiring_attestations` is called twice in succession with the same `subject` and `within_seconds` and no state changes occur between calls, THE ExpirationQuery SHALL return identical results both times.
2. WHEN `get_issuer_expiring_attestations` is called twice in succession with the same `issuer` and `within_seconds` and no state changes occur between calls, THE ExpirationQuery SHALL return identical results both times.
3. FOR ALL attestation IDs returned by `get_expiring_attestations(subject, within_seconds)`, THE ExpirationQuery SHALL guarantee that fetching each ID via `get_attestation` returns an attestation with `revoked = false` and `expiration` within `(current_time, current_time + within_seconds]`.
4. FOR ALL attestation IDs returned by `get_issuer_expiring_attestations(issuer, within_seconds)`, THE ExpirationQuery SHALL guarantee that fetching each ID via `get_attestation` returns an attestation with `revoked = false` and `expiration` within `(current_time, current_time + within_seconds]`.
