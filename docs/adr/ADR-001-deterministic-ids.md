# ADR-001: Deterministic IDs instead of sequential counters

- **Status**: Accepted
- **Date**: 2024-01-01

## Context

Every attestation needs a unique, stable identifier so that callers can
retrieve, revoke, or reference it later. Two broad approaches exist:

1. **Sequential counter** — maintain a global `u64` counter in storage,
   increment it on each write, and use the resulting integer as the ID.
2. **Deterministic hash** — derive the ID from the attestation's content
   fields using a cryptographic hash function.

Soroban contracts run in a parallel, multi-contract environment. A global
counter requires a read-modify-write on a single shared storage entry for
every attestation created, which serialises all writes and creates a
contention hotspot. It also means the ID carries no information about the
attestation itself, making off-chain reconstruction impossible without
querying the chain.

## Decision

Attestation IDs are the SHA-256 hash of the XDR-serialised concatenation of
the four fields that uniquely identify an attestation:

```
id = SHA-256( issuer_xdr || subject_xdr || claim_type_xdr || timestamp_xdr )
```

The hash is hex-encoded to a 64-character `String` for storage and display.
Bridge attestations extend the input with `source_chain_xdr || source_tx_xdr`
to remain unique across chains. Multi-sig proposal IDs use the same scheme
with a `"multisig:"` byte prefix to prevent collisions with regular
attestation IDs.

Implementation: [`src/types.rs`](../../src/types.rs) —
`Attestation::generate_id`, `Attestation::generate_bridge_id`,
`MultiSigProposal::generate_id`.

## Consequences

**Positive**
- No shared mutable counter — ID generation is stateless and parallelisable.
- IDs are reproducible off-chain from known inputs, enabling indexers and
  clients to compute expected IDs without a chain query.
- Duplicate detection is a single storage existence check
  (`Storage::has_attestation`) rather than a full scan.
- IDs are content-addressed: the same logical attestation always maps to the
  same ID, making idempotent imports safe.

**Negative**
- Two attestations with identical issuer, subject, claim type, and timestamp
  collide. The contract rejects the second with `DuplicateAttestation`. In
  practice this is desirable (prevents replay), but it means an issuer cannot
  issue the same claim type to the same subject twice within the same ledger
  second.
- IDs are opaque 64-character hex strings rather than human-readable numbers,
  which makes manual debugging slightly harder.

**Neutral**
- ID length is fixed at 64 characters regardless of input size.
- The XDR serialisation format is stable across Soroban SDK versions for the
  types used (`Address`, `String`, `u64`).
