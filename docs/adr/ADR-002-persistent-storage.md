# ADR-002: Persistent storage instead of temporary storage

- **Status**: Accepted
- **Date**: 2024-01-01

## Context

Soroban offers three storage tiers with different lifetime and cost
characteristics:

| Tier | Lifetime | Cost | Typical use |
|------|----------|------|-------------|
| `instance` | Lives as long as the contract instance | Cheapest per-entry | Small, always-needed config |
| `persistent` | Survives indefinitely when TTL is extended | Medium | Long-lived application data |
| `temporary` | Automatically deleted after TTL expires | Cheapest overall | Short-lived session data |

Attestations are legal/compliance records. A KYC attestation issued today may
need to be verified months or years later. Temporary storage would silently
delete attestations after their TTL, making them unverifiable without any
on-chain signal. That is unacceptable for a trust layer.

## Decision

All attestation data, indexes, issuer/bridge registries, and claim type
records are stored in **persistent** storage. Only the three small
contract-wide config entries (`Admin`, `Version`, `FeeConfig`, `TtlConfig`)
use **instance** storage because they are always read together and are tiny.

Every persistent write also calls `extend_ttl` to refresh the entry's
lifetime to the configured TTL (default 30 days, overridable via
`initialize`). This means active data stays alive as long as the contract is
used, and dormant data eventually expires — which is the correct behaviour for
a trust registry (stale issuers and old attestations should not live forever
at network cost).

Implementation: [`src/storage.rs`](../../src/storage.rs) — every
`env.storage().persistent().set(...)` call is immediately followed by
`extend_ttl`.

## Consequences

**Positive**
- Attestations survive indefinitely as long as their TTL is periodically
  refreshed, matching the long-lived nature of compliance records.
- TTL is configurable at deploy time, giving operators control over the
  cost/longevity trade-off.
- Persistent storage entries are individually addressable, so a single
  attestation lookup does not load unrelated data.

**Negative**
- Persistent storage costs more in ledger rent than temporary storage.
  High-volume deployments will pay ongoing rent to keep attestations alive.
- Operators must monitor TTL and either extend it or accept that old entries
  will eventually expire. The current design refreshes TTL on every write but
  not on reads, so read-only attestations will eventually expire if never
  updated.

**Neutral**
- The TTL is expressed in ledger-seconds (days × 17,280 ledgers/day) rather
  than wall-clock time, so the effective duration depends on Stellar's ledger
  close rate (~5 s/ledger).
- Instance storage is used for config because those entries are always loaded
  when the contract is invoked, making the per-entry overhead negligible.
