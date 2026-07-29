# ADR-006: Multi-Sig Attestation Design

- **Status**: Accepted
- **Date**: 2026-05-26

## Context

High-value claim types such as `ACCREDITED_INVESTOR` carry significant
financial and legal weight. Allowing a single registered issuer to unilaterally
issue such a credential creates an unacceptable single point of failure: one
compromised or colluding issuer can certify any subject without oversight.

Three design questions arose when adding multi-sig support:

1. **Why M-of-N instead of requiring all N signers?**
2. **Why a 7-day proposal TTL?**
3. **Why are proposals immutable once finalized?**

## Decision

### 1. M-of-N threshold instead of unanimous signing

The contract requires `threshold` signatures out of a fixed `required_signers`
list, where `1 ≤ threshold ≤ len(required_signers)`.

**Why not require all N signers?**

Unanimous signing (N-of-N) is operationally fragile. A single unavailable or
unresponsive issuer permanently blocks credential issuance for a subject who
has legitimately satisfied all requirements. In regulated contexts (e.g.
accredited investor verification) this creates a denial-of-service risk that
is worse than the security risk it was meant to prevent.

M-of-N provides a configurable fault-tolerance margin. A 2-of-3 scheme, for
example, tolerates one absent signer while still requiring independent
agreement from a majority. The proposer counts as the first signer
automatically, so the minimum overhead for a 2-of-3 scheme is one additional
co-sign call.

**Why not a fixed 2-of-3?**

Different deployments have different trust models. A consortium of five
regulated entities might require 3-of-5; a simpler setup might use 2-of-2.
Hardcoding a ratio would force all deployments into the same model. The
threshold is validated at proposal time (`threshold == 0` or
`threshold > signer_count` → `InvalidThreshold`) so invalid configurations
are rejected immediately.

**Premium issuer bypass**

Premium-tier issuers are granted unilateral authority for `ACCREDITED_INVESTOR`
via `propose_attestation`. This is an explicit admin policy decision: the admin
elevates an issuer to `IssuerTier::Premium` only after additional vetting,
effectively making that issuer a trusted single signer for that claim type.
The bypass is transparent in the code and auditable via the issuer tier
registry.

### 2. 7-day proposal TTL

Proposals expire `7 * 24 * 60 * 60` seconds (604,800 seconds) after creation,
enforced by comparing `env.ledger().timestamp()` against `proposal.expires_at`
in `cosign_attestation`.

**Why 7 days?**

- **Operational realism.** Co-signers are human operators at separate
  organisations. A window shorter than a business week (e.g. 24 hours) would
  routinely expire before all required parties have had a chance to review and
  sign, especially across time zones or during weekends.
- **Bounded storage cost.** Proposals that never reach threshold must not
  accumulate indefinitely. A 7-day window ensures stale proposals become
  inert within a predictable timeframe. (Proposal records remain in storage
  after expiry but are rejected by `cosign_attestation`, so they do not
  consume active processing.)
- **Security.** An unbounded TTL would allow an attacker who compromises one
  issuer key to wait indefinitely for an opportunity to co-sign a stale
  proposal. A 7-day cap limits the window of exposure.

The TTL is exposed via `get_multisig_ttl` for off-chain tooling and is defined
as `MULTISIG_PROPOSAL_TTL_SECS` in `src/types.rs` so it can be updated in a
single place if governance requirements change.

### 3. Proposals are immutable once finalized

When `sig_count >= threshold`, `cosign_attestation` sets `proposal.finalized =
true`, writes the proposal back to storage, creates the attestation, and
returns. Any subsequent call to `cosign_attestation` with the same
`proposal_id` is rejected with `ProposalFinalized`.

**Why not allow additional signatures after finalization?**

- **Deterministic attestation ID.** The attestation ID is derived from the
  proposer address, subject, claim type, and `proposal.created_at` timestamp
  (see ADR-001). Allowing post-finalization signatures would not change the
  attestation record but would silently succeed, creating a misleading audit
  trail where the signature count in the proposal diverges from the threshold
  that actually triggered issuance.
- **Idempotency.** The `finalized` flag makes the proposal's terminal state
  explicit and permanent. Callers can inspect `proposal.finalized` to
  determine whether the attestation has been issued without querying the
  attestation store separately.
- **Preventing double-issuance.** If finalization were not terminal, a race
  condition could allow two concurrent `cosign_attestation` calls to both
  observe `sig_count == threshold - 1`, both increment, and both attempt to
  store the attestation. The `finalized` flag, written atomically with the
  attestation in the same ledger entry update, closes this window.

## Consequences

**Positive**
- Single-issuer compromise cannot unilaterally issue high-value credentials.
- Configurable threshold accommodates diverse governance models without
  contract changes.
- 7-day TTL balances operational realism against storage and security concerns.
- Immutable finalization prevents double-issuance and keeps the audit log
  consistent.

**Negative**
- Proposals that expire before reaching threshold leave orphaned storage
  entries. A future garbage-collection mechanism may be needed at scale.
- The premium-issuer bypass for `ACCREDITED_INVESTOR` is a policy exception
  that must be carefully governed; granting `IssuerTier::Premium` to an
  unvetted issuer defeats the multi-sig protection for that claim type.
- The 7-day TTL is a constant, not a per-proposal parameter. Deployments with
  stricter time requirements must wait for a contract upgrade to change it.

**Neutral**
- Proposal IDs use the same deterministic hash scheme as attestation IDs
  (with a `"multisig:"` prefix) — see ADR-001.
- The `required_signers` list is fixed at proposal creation time; signers
  cannot be added or substituted after the proposal is stored.
