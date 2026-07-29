# ADR-004: Separate issuer and subject indexes

- **Status**: Accepted
- **Date**: 2024-01-01

## Context

Attestation lookups are needed from two directions:

- **Subject perspective** — "what attestations exist about this address?" Used
  by `has_valid_claim`, `get_subject_attestations`, `get_valid_claims`, and
  cross-contract verification.
- **Issuer perspective** — "what attestations has this issuer created?" Used
  by `get_issuer_attestations`, compliance audits, and issuer activity stats.

Without an index, answering either question requires scanning every
attestation in storage — which is not feasible on-chain because Soroban
contracts cannot iterate over all storage keys.

The alternative is to maintain one or more ordered lists of attestation IDs
keyed by the relevant address. The question is whether to maintain one shared
index or two separate ones.

## Decision

Two independent persistent indexes are maintained for every attestation write:

- `SubjectAttestations(Address)` → `Vec<String>` of attestation IDs, ordered
  by insertion time (oldest first).
- `IssuerAttestations(Address)` → `Vec<String>` of attestation IDs, ordered
  by insertion time (oldest first).

Both are updated atomically inside the `store_attestation` helper in
[`src/lib.rs`](../../src/lib.rs), which is the single call site for all
attestation creation paths (native, batch, import, bridge, multi-sig
activation). This guarantees the indexes are always consistent with the
attestation store.

```rust
// src/lib.rs — store_attestation
fn store_attestation(env: &Env, attestation: &Attestation) {
    Storage::set_attestation(env, attestation);
    Storage::add_subject_attestation(env, &attestation.subject, &attestation.id);
    Storage::add_issuer_attestation(env, &attestation.issuer, &attestation.id);
    // ... stats update
}
```

Implementation: [`src/storage.rs`](../../src/storage.rs) —
`StorageKey::SubjectAttestations`, `StorageKey::IssuerAttestations`,
`add_subject_attestation`, `add_issuer_attestation`,
`get_subject_attestations`, `get_issuer_attestations`.

## Consequences

**Positive**
- Both lookup directions are O(1) storage reads (load the index vector) plus
  O(n) iteration over the returned IDs — no full-table scan required.
- Pagination (`start` / `limit`) is straightforward because the index is an
  ordered `Vec`.
- `has_valid_claim` only needs to iterate the subject's index, not all
  attestations globally.
- Issuer audits (`get_issuer_attestations`) are equally efficient without
  touching subject data.

**Negative**
- Every attestation write touches three storage entries instead of one
  (the attestation record plus both index vectors), increasing ledger resource
  usage per transaction.
- Index vectors grow monotonically (see ADR-003). Very active subjects or
  issuers will have large vectors, making index reads progressively more
  expensive as the vector must be deserialised in full before pagination is
  applied.
- There is no cross-index (e.g. "all attestations for subject X from issuer
  Y") — callers must load the subject index and filter client-side, or use
  `has_valid_claim_from_issuer` which does the same on-chain.

**Neutral**
- Revoked and expired attestation IDs remain in the index vectors (consistent
  with ADR-003). Query functions filter by status after loading the index.
- The `IssuerStats` counters (`total_issued`, `total_revoked`) are maintained
  separately from the index vectors and provide O(1) aggregate counts without
  loading the full index.
