# ADR-006: OR-logic across issuers in `has_valid_claim`

- **Status**: Accepted
- **Date**: 2026-06-01

## Context

A subject may hold the same claim type issued by more than one registered
issuer. For example, two KYC providers might both issue `KYC_PASSED` to the
same wallet address. When a consuming contract calls `has_valid_claim(subject,
"KYC_PASSED")`, the contract must decide how to aggregate across those
multiple attestations.

Two aggregation strategies were considered:

1. **OR-logic** — return `true` if *any* attestation for the given claim type
   is currently valid (not revoked, not expired).
2. **AND-logic / strict validation** — return `true` only if *all* attestations
   for the given claim type are currently valid, or require a specific issuer
   to be named.

The choice has direct consequences for security, usability, and the trust
model exposed to consuming contracts.

## Decision

`has_valid_claim` uses **OR-logic**: it iterates the subject's attestation
index, and returns `true` as soon as it finds one attestation matching the
requested claim type that is neither revoked nor expired. The search
short-circuits on the first valid match.

```rust
// src/lib.rs — has_valid_claim (simplified)
for id in subject_attestations {
    let att = Storage::get_attestation(env, &id);
    if att.claim_type == claim_type && !att.revoked && !is_expired(env, &att) {
        return true;   // short-circuit: one valid attestation is sufficient
    }
}
false
```

When a caller needs to verify a claim from a *specific* issuer, they use
`has_valid_claim_from_issuer(subject, claim_type, issuer)`, which adds an
issuer equality check to the same loop.

Implementation: [`src/lib.rs`](../../src/lib.rs) — `has_valid_claim`,
`has_valid_claim_from_issuer`.

## Rationale

### Why OR-logic is the correct default

**Issuer redundancy is a feature, not a risk.** In a decentralised identity
system, subjects may obtain the same credential from multiple providers for
legitimate reasons: a primary KYC provider and a backup, two jurisdictional
authorities, or a migration from a deprecated issuer to a new one. Requiring
*all* attestations to be valid would mean that a single revocation or
expiration — even from a secondary issuer the subject no longer relies on —
would invalidate the subject's credential entirely. This is a denial-of-service
vector, not a security improvement.

**Revocation semantics are per-attestation, not per-claim-type.** An issuer
revokes a specific attestation record (identified by its deterministic ID).
Revocation does not express "this subject should never hold this claim type";
it expresses "this particular attestation I issued is no longer valid." A
second, independent attestation of the same claim type from a different issuer
is unaffected by the first issuer's revocation decision.

**Consuming contracts can always opt in to stricter validation.** A contract
that requires a credential from a specific trusted issuer calls
`has_valid_claim_from_issuer`. A contract that requires credentials from
multiple specific issuers simultaneously calls `has_valid_claim_from_issuer`
once per required issuer. OR-logic at the default level does not prevent
stricter policies at the application level.

**Short-circuit evaluation keeps gas costs bounded.** Because the loop returns
on the first valid match, subjects with many attestations do not pay a
proportionally higher cost for the common case where at least one valid
attestation exists.

## Alternatives Considered

### AND-logic: all attestations must be valid

Rejected. If a subject holds two `KYC_PASSED` attestations and one is revoked
(e.g. the issuing organisation shut down), `has_valid_claim` would return
`false` even though the subject holds a perfectly valid credential from a
second issuer. This would silently break integrations whenever an issuer is
deregistered or their attestations expire, with no action required from the
subject.

### Require a specific issuer to be named (no default aggregation)

Rejected as the default. Forcing every caller to name an issuer couples
consuming contracts to the issuer registry. If an issuer is replaced, all
consuming contracts must be updated. OR-logic decouples consuming contracts
from issuer churn: as long as *some* registered issuer has validated the
subject, the claim is satisfied.

`has_valid_claim_from_issuer` is provided for callers that explicitly need
issuer-specific validation, so this option is available as an opt-in.

### Weighted voting / quorum across issuers

Rejected as over-engineering for the current use cases. A quorum model would
require the contract to maintain per-claim-type issuer weights, adding storage
and governance complexity. The multi-sig attestation mechanism (ADR-006 /
`propose_attestation`) already addresses the use case where M-of-N issuers
must agree *before* an attestation is created. Once an attestation exists, it
is a first-class record and OR-logic applies.

## Security Implications

**A single valid attestation is sufficient.** This means a consuming contract
that calls `has_valid_claim` trusts the entire set of registered issuers
equally for that claim type. If an issuer is registered that should not be
trusted for a particular claim type, the admin must either not register that
issuer or the consuming contract must use `has_valid_claim_from_issuer` with
an explicit allowlist.

**Issuer deregistration does not retroactively invalidate attestations.**
Removing an issuer via `remove_issuer` prevents future issuance but leaves
existing attestations in place (see README — Issuer Removal Behavior). A
deregistered issuer's attestations remain valid under OR-logic until they are
explicitly revoked or expire. Consuming contracts that require only
currently-registered issuers must use `has_valid_claim_from_issuer` and
maintain their own issuer allowlist.

**OR-logic does not weaken multi-sig attestations.** Attestations created via
`propose_attestation` / `cosign_attestation` are stored as ordinary
attestation records once finalized. OR-logic applies to them identically to
natively-issued attestations. The multi-sig requirement is enforced at
*creation* time; once the attestation exists, it is treated like any other.

## Consequences

**Positive**
- Subjects retain a valid credential as long as any one of their issuers
  remains active, making the system resilient to issuer churn.
- Consuming contracts are decoupled from the issuer registry; they do not need
  to be updated when issuers are added or replaced.
- Short-circuit evaluation keeps `has_valid_claim` gas costs proportional to
  the position of the first valid attestation, not the total attestation count.
- The stricter `has_valid_claim_from_issuer` is available for applications
  that need issuer-specific trust.

**Negative**
- A consuming contract that calls `has_valid_claim` implicitly trusts all
  registered issuers for that claim type. Misconfigured or compromised issuers
  that have not yet been deregistered can satisfy the check.
- Subjects cannot selectively invalidate one issuer's attestation without
  asking that issuer to revoke it; there is no subject-initiated per-issuer
  opt-out.

**Neutral**
- The behaviour is documented in the README multi-issuer table and is
  consistent with the `has_any_claim` OR-logic across claim types.
- `has_all_claims` uses AND-logic across *claim types* (not issuers), which is
  a complementary but orthogonal design choice.
