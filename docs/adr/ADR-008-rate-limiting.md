# ADR-008: Rate Limiting Design — Per-Issuer Last-Issuance Timestamp

- **Status**: Accepted
- **Date**: 2026-05-29

## Context

TrustLink needs to protect against high-frequency attestation spam from a single
issuer. Without rate limiting, a registered issuer could flood the contract with
thousands of attestations per ledger close, exhausting storage quotas and
degrading availability for other issuers.

The rate limiting mechanism must satisfy three constraints:

1. **On-chain enforceability** — The limit must be checked and enforced inside
   the contract without relying on off-chain intermediaries.
2. **Low storage overhead** — Each registered issuer should require at most one
   additional storage entry for rate limit state.
3. **Configurable by the admin** — The admin must be able to set a global default
   and per-claim-type overrides without a contract upgrade.

## Decision

### Chosen approach: per-issuer last-issuance timestamp

The contract stores a single `LastIssuanceTime(Address)` entry per issuer in
persistent storage. On every attestation creation call, `check_rate_limit`
compares the current ledger timestamp against this stored value:

```rust
if current_time.saturating_sub(last) < interval {
    return Err(Error::RateLimited);
}
```

After a successful issuance, `set_last_issuance_time` overwrites the stored
value with the current timestamp.

The effective interval is resolved in this priority order:

1. Per-claim-type override (`set_rate_limit_for_claim_type(admin, claim_type, interval_secs)`)
2. Global default (`set_rate_limit(admin, min_issuance_interval)`)
3. No limit (if neither is configured)

An interval of `0` disables the rate limit entirely for that claim type.

### Why not token bucket?

A token bucket allows bursting up to a configured capacity and then refills at
a fixed rate. While more expressive, it requires storing both a token count and
a last-refill timestamp per issuer, doubling the storage reads and writes per
attestation. Token buckets also introduce parameter complexity (capacity,
refill rate) that is difficult to reason about in a governance context. Given
that the primary concern is sustained spam rather than short bursts from
legitimate issuers, a simple interval check is sufficient.

### Why not sliding window?

A sliding window counter tracks the number of issuances within a rolling time
window, enabling accurate rate enforcement regardless of where in the window
issuances cluster. However, it requires storing a sorted list of recent
timestamps per issuer, which is expensive on Soroban where every storage entry
has a per-byte cost. For TrustLink's use case — preventing sustained abuse
rather than enforcing precise throughput — the simpler interval approach is
preferred.

### Why not per (issuer, claim_type) timestamps?

Tracking a separate last-issuance timestamp for each `(issuer, claim_type)`
pair would allow independent rate limiting for each claim type. The cost is
O(claim types) storage entries per issuer. Given that per-claim-type rate
limits are already the exception rather than the rule, a single shared timer
per issuer is a reasonable simplification for the common case.

## Consequences

### Positive

- One storage entry per issuer — minimal overhead.
- Straightforward to reason about: one ledger call per interval, no complex
  state machine.
- Per-claim-type overrides allow fine-grained control for high-value or
  high-risk claim types without altering the global policy.
- `RateLimited` errors are deterministic and auditable on-chain.

### Negative

- **Shared timer across claim types**: The `LastIssuanceTime(Address)` key is
  keyed on issuer address alone. Issuing any claim type resets the timer for
  all claim types. An issuer with two claim types that legitimately require
  independent high-frequency issuance may be inadvertently throttled.
- **Delegates bypass rate limiting**: `create_attestation_as_delegate` does not
  call `check_rate_limit` and does not update `LastIssuanceTime`. A delegate
  key can therefore issue attestations on behalf of the delegating issuer at an
  unconstrained rate. Operators who rely on rate limiting for spam prevention
  should apply off-chain controls on delegate key distribution.
- **Batch issuance counts as one**: `create_attestations_batch` applies a
  single `check_rate_limit` call and a single `set_last_issuance_time` update
  regardless of the number of subjects in the batch. An issuer can issue N
  attestations in one call after waiting one interval, effectively achieving
  burst throughput of N per interval.
- **Ledger timestamp granularity**: The Soroban ledger timestamp advances in
  approximately 5-second increments. Rate limit intervals shorter than ~10
  seconds may not behave as configured due to timestamp resolution.
- **No cross-issuer coordination**: Each issuer's timer is independent. If an
  issuer operates multiple registered addresses, the combined issuance rate is
  a multiple of the configured per-issuer limit.

### Neutral

- Rate limit configuration is stored separately from issuer registration; an
  issuer can be registered without any rate limit in effect.
- The rate limit applies to `create_attestation` and `create_attestations_batch`
  but not to multi-sig proposal creation (`propose_attestation`) — the proposal
  workflow has its own TTL-based throttle via the 7-day expiry window.
