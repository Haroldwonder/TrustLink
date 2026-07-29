# ADR-007: Pull-Based Attestation Request Workflow

- **Status**: Accepted
- **Date**: 2026-05-26

## Context

The original attestation model is **push-based**: a registered issuer
unilaterally calls `create_attestation` to certify a subject. This works well
when the issuer already holds the verification data (e.g. a KYC provider
processing a batch), but it breaks down when the subject must initiate the
process — for example, a user who wants to prove accredited-investor status to
an issuer they have never interacted with before.

Three design questions arose when adding the pull-based request workflow:

1. **Why pull-based (subject initiates) instead of extending the push model?**
2. **Why a 7-day request TTL?**
3. **Why is the rejection reason optional?**

## Decision

### 1. Pull-based workflow: subject initiates the request

In the pull model the subject calls `request_attestation(subject, issuer,
claim_type)`, creating a pending `AttestationRequest` record. The issuer then
calls `fulfill_request` or `reject_request` at their discretion.

**Why not extend the push model?**

The push model requires the issuer to know the subject's address and to decide
unilaterally to issue a credential. This is appropriate for batch KYC
pipelines, but it cannot express the common pattern where:

- The subject has completed an off-chain verification process and needs to
  signal readiness to the issuer.
- The issuer wants an on-chain audit trail showing that the subject explicitly
  requested the credential (consent record).
- The subject wants to target a specific issuer rather than waiting for any
  issuer to notice them.

A pull request creates an on-chain signal that the subject has consented to
and requested the credential, which is valuable for compliance purposes (GDPR
Article 6 lawful basis, financial regulation audit trails). It also decouples
the subject's off-chain verification workflow from the issuer's issuance
schedule.

**Why not use off-chain messaging?**

Off-chain channels (email, API calls) are not auditable on-chain and cannot be
verified by third-party contracts. An on-chain request record is immutable,
timestamped, and queryable by any contract or indexer.

**Relationship to the push model**

The pull workflow is additive — it does not replace `create_attestation`. Both
paths produce the same `Attestation` record. `fulfill_request` internally calls
`store_attestation` with the same logic as the push path, including limit
checks and duplicate-ID guards.

### 2. 7-day request TTL

Requests expire `7 * 24 * 60 * 60` seconds (604,800 seconds) after creation,
stored as `expires_at` on the `AttestationRequest` record. `fulfill_request`
and `reject_request` both check `current_time >= request.expires_at` and
return `RequestExpired` if the window has passed.

**Why 7 days?**

- **Issuer review time.** Issuers are human-operated organisations. A week
  gives them time to review the off-chain verification evidence, consult
  compliance teams, and act — without the request lingering indefinitely.
- **Consistency with multi-sig TTL.** Multi-sig proposals use the same 7-day
  window (see ADR-006). A uniform TTL across time-bounded workflows simplifies
  operator tooling and monitoring: a single alert threshold covers both
  proposal and request expiry.
- **Bounded pending queue.** `get_pending_requests` filters out expired entries
  at read time. A finite TTL ensures the pending queue does not grow without
  bound even if issuers stop processing requests.
- **Security.** An unbounded TTL would allow a request to sit open
  indefinitely, creating a persistent phishing surface: a malicious subject
  could create a request and wait for a moment when the issuer's key is
  compromised to have it fulfilled. A 7-day cap limits this window.

The TTL is defined as `ATTESTATION_REQUEST_TTL_SECS` in `src/types.rs`,
co-located with `MULTISIG_PROPOSAL_TTL_SECS`, so both can be updated together
if governance requirements change.

### 3. Rejection reason is optional

`reject_request(issuer, request_id, reason: Option<String>)` accepts a
`None` reason. When provided, the reason is validated to a maximum of 128
characters by `validate_reason` (same helper used for revocation reasons) and
stored on the `AttestationRequest` record as `rejection_reason`.

**Why not require a reason?**

- **Privacy and legal risk.** Issuers operating under financial regulation
  (AML, sanctions screening) may be legally prohibited from disclosing why a
  subject was rejected. Requiring a reason would force issuers to either
  violate their legal obligations or provide a meaningless placeholder.
- **Operational flexibility.** Some rejections are administrative (duplicate
  request, wrong issuer) rather than substantive. Mandating a reason for every
  rejection adds friction without adding value in those cases.
- **Consistency with revocation.** `revoke_attestation` also accepts an
  optional reason for the same reasons. A consistent API reduces cognitive
  overhead for integrators.

**Why allow a reason at all?**

When a reason can be safely disclosed (e.g. "claim type not supported by this
issuer"), providing it improves the subject's experience and reduces support
overhead. The optional field makes this possible without mandating it.

The 128-character cap prevents storage exhaustion while allowing a meaningful
one-sentence explanation.

## Consequences

**Positive**
- Subjects can initiate credential requests on-chain, creating an auditable
  consent record.
- Issuers get a structured pending queue (`get_pending_requests`) rather than
  relying on off-chain coordination.
- The pull workflow is fully additive — existing push-based integrations are
  unaffected.
- Optional rejection reasons accommodate legal constraints while still
  supporting informative feedback when safe to provide.

**Negative**
- Expired requests remain in storage as inert records. A garbage-collection
  mechanism may be needed at scale (same concern as multi-sig proposals).
- The 7-day TTL is a constant, not a per-request parameter. Deployments
  requiring shorter windows (e.g. time-sensitive compliance checks) must wait
  for a contract upgrade.
- A subject can create multiple requests to the same issuer for the same claim
  type (different timestamps → different IDs). Issuers must handle duplicate
  pending requests in their off-chain tooling.

**Neutral**
- Request IDs use the same deterministic hash scheme as attestation IDs, with
  a `"req:"` byte prefix to prevent collisions — see ADR-001.
- `fulfill_request` reuses `store_attestation` and all its guards (limit
  checks, duplicate-ID detection), so the pull path has identical safety
  properties to the push path.
