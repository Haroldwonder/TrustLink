# ADR-001: Deterministic IDs instead of sequential counters

- **Status**: Accepted
- **Date**: 2024-01-01

## Context

Every record needs a stable identifier so callers can retrieve or reference it.
TrustLink currently has two compatible ID formats: legacy attestation and
bridge IDs use 32 lowercase hexadecimal characters, while request, multi-sig
proposal, and bundle IDs use 64 lowercase hexadecimal characters.

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

Attestation IDs are derived from the SHA-256 hash of the XDR-serialised
concatenation of the four fields that uniquely identify an attestation. The
first 16 hash bytes are hex-encoded, yielding 32 lowercase hexadecimal
characters:

```
id = hex(SHA-256(issuer_xdr || subject_xdr || claim_type_xdr || timestamp_be)[0..16])
```

Bridge IDs use the same truncation after hashing the bridge, subject, claim
type, source chain, source transaction, and big-endian timestamp. These legacy
formats are retained because IDs are storage keys and are referenced by
clients and indexers.

Request, multi-sig proposal, and bundle IDs use `Attestation::hash_payload`:
SHA-256 over a domain prefix (`req:`, `multisig:`, or `bundle:`), XDR-encoded
fields, and an XDR-encoded timestamp; the full hash is hex-encoded to 64
lowercase characters. SDK validators accept both lengths while requiring
lowercase hexadecimal characters.

Any future change to the attestation ID format must use a versioned migration
or new ID namespace; existing IDs must remain valid and resolvable.

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
  collide. The contract rejects the second with `DuplicateAttestation`.
- Attestation and bridge IDs are truncated to 128 bits, unlike the full 256-bit
  hash used by other ID families. Existing IDs cannot be changed without a
  storage and client migration.

**Neutral**
- ID length is fixed per record family: 32 characters for attestation and
  bridge IDs, and 64 characters for request, multi-sig proposal, and bundle
  IDs.
- Attestation timestamps use raw big-endian bytes; other ID families use
  XDR-encoded timestamps.
