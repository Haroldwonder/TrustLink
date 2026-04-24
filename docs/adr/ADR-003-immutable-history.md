# ADR-003: Immutable attestation history (no delete)

- **Status**: Accepted
- **Date**: 2024-01-01

## Context

When an attestation is no longer valid — because the issuer made a mistake,
the subject's status changed, or the claim expired — there are two ways to
handle it:

1. **Delete** — remove the storage entry entirely. The attestation ceases to
   exist on-chain.
2. **Revoke** — set a `revoked: bool` flag on the existing record and leave
   the entry in storage.

TrustLink is designed to serve legal and compliance use cases where auditability
matters. Regulators, auditors, and counterparties may need to answer questions
like "was this address KYC-verified on date X?" even after the attestation has
been superseded. Deletion makes that question unanswerable on-chain.

Additionally, Soroban's event log is append-only. Even if a storage entry is
deleted, the original `["created", subject]` event remains in ledger history.
Allowing deletion would create an inconsistency between the event log (which
shows a creation) and storage (which shows nothing), confusing off-chain
indexers.

## Decision

Attestations are **never deleted**. The only state transitions are:

- `revoked: false` → `revoked: true` (via `revoke_attestation` or
  `revoke_attestations_batch`)
- `expiration: None` → `expiration: Some(t)` (via `renew_attestation` or
  `update_expiration`)

The `revoked` flag is checked by `get_attestation_status`, `has_valid_claim`,
and all related query functions, so revoked attestations are treated as
invalid for all practical purposes while remaining queryable for audit.

Revocation also prunes the attestation ID from the subject and issuer
indexes used for pagination and listing.

Implementation: [`src/lib.rs`](../../src/lib.rs) — `revoke_attestation`,
`revoke_attestations_batch`. [`src/types.rs`](../../src/types.rs) —
`Attestation::get_status`.

## Consequences

**Positive**
- Full audit trail: every attestation ever issued is permanently queryable,
  supporting compliance, legal discovery, and dispute resolution.
- Consistent with the event log: a `["created"]` event always has a
  corresponding storage entry.
- Revocation is itself auditable — the `["revoked", issuer]` event records
  who revoked and when.
- Off-chain indexers can reconstruct complete issuer history using events /
  audit log entries, even though pagination indexes prune revoked IDs.

**Negative**
- Storage is never reclaimed for revoked attestations. High-volume issuers
  accumulate entries indefinitely (subject to TTL expiry).
- The attestation record remains in storage, but revoked attestation IDs are
  removed from the subject/issuer index vectors, so pagination counts reflect
  only non-revoked IDs.
- There is no on-chain mechanism for a subject to request erasure (e.g. GDPR
  right-to-erasure). Any such requirement must be handled off-chain or via a
  wrapper contract.

**Neutral**
- `get_attestation_status` returns `Revoked` for revoked entries, so callers
  do not need to inspect the `revoked` field directly.
- The `imported` and `bridged` flags follow the same immutability rule —
  they are set once at creation and never changed.
