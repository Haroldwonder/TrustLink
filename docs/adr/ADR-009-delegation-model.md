# ADR-009: Delegation Model and Trust Chain Implications

- **Status**: Accepted
- **Date**: 2026-05-29

## Context

TrustLink's initial design assumed that only directly registered issuers would
create attestations. In practice, enterprise deployments require a hub-and-spoke
model: a single trusted issuer (the root) wants to authorise subsidiary
agents — KYC vendors, branch offices, automated services — to issue a specific
subset of claim types on its behalf.

Without a delegation mechanism, every authorised agent must be independently
registered as a root issuer, granting it unbounded authority to issue any claim
type. This creates two problems:

1. **Overpermissioning** — a subsidiary that only handles `KYC_PASSED` checks
   gains the same authority as the root issuer over all claim types.
2. **Attribution loss** — attestations appear to come from a vendor address
   rather than the root issuer address, breaking relying-party trust policies
   that are keyed on the root issuer.

## Decision

### Delegation mechanism

The admin grants an issuer the ability to call `delegate_claim_type`:

```rust
delegate_claim_type(issuer, delegate, claim_type, expiration: Option<u64>)
```

This stores a `Delegation` record keyed on `(delegator, delegate, claim_type)`:

```rust
pub struct Delegation {
    pub delegator: Address,
    pub delegate: Address,
    pub claim_type: String,
    pub expiration: Option<u64>,
}
```

Delegations are **per-claim-type**: a delegate authorised for `KYC_PASSED`
cannot issue `AML_CLEARED` on the delegator's behalf without a separate
delegation record.

The delegate invokes:

```rust
create_attestation_as_delegate(delegate, delegator, subject, claim_type, ...)
```

The contract verifies the delegation record exists and has not expired before
proceeding. The resulting attestation is stored with `issuer = delegator`,
making the attestation indistinguishable from one the delegator issued
directly. The `actor` field in the audit log is set to the delegate address,
preserving a complete audit trail.

### Trust chain model

```
Admin
  └─ registers Issuer A  (root issuer)
         └─ delegates claim_type=KYC_PASSED to Agent X
                └─ Agent X creates attestations for subjects
                       └─ attestation.issuer = Issuer A
                          attestation.audit_actor = Agent X
```

Relying parties that trust Issuer A therefore implicitly trust every
attestation created by any current or past delegate of Issuer A for the
delegated claim type.

### Revocation

An issuer may revoke a delegation at any time:

```rust
revoke_delegation(issuer, delegate, claim_type)
```

Revocation removes the delegation record immediately. Delegate calls after
revocation return `Error::Unauthorized`. Existing attestations previously
created by the delegate are not affected — they remain valid under the
delegator's identity unless separately revoked.

### Why not a capability token model?

An alternative design would issue the delegate a capability token (an NFT or
signed capability struct) that the contract verifies on each call. This avoids
a storage entry per delegation but requires the delegate to present the token
on every call, which complicates key rotation and token revocation — a revoked
token must be tracked on-chain anyway, negating the storage advantage. The
storage-keyed model chosen here is simpler and allows instant revocation without
a token blocklist.

### Why not inherit the delegator's rate limit?

`create_attestation_as_delegate` does not call `check_rate_limit` and does not
update `LastIssuanceTime`. See ADR-008 for the rate limiting design. The
interaction between delegation and rate limiting is a known gap: operators who
need rate limiting on delegate paths must enforce this off-chain or at the
delegate key management layer.

## Consequences

### Positive

- Claim-type-scoped authority limits blast radius: a compromised delegate key
  can only issue the specific claim types it was delegated.
- Attestations retain the root issuer identity, preserving relying-party trust
  policies without modification.
- The audit log records the actual actor (delegate address), enabling forensic
  attribution of every attestation to the specific key that created it.
- Optional expiration on delegations supports time-bounded vendor contracts
  without requiring explicit revocation.
- Delegations are revocable immediately, with no grace period.

### Negative

- **Compromised delegate = compromised issuer for delegated claim types**: A
  delegate key that is stolen or misused can issue arbitrary attestations of
  the delegated claim type under the trusted root issuer's identity. Relying
  parties cannot distinguish a legitimate attestation from a fraudulent one
  issued by a compromised delegate.
- **No rate limiting on delegate path**: Delegates bypass the per-issuer rate
  limit (see ADR-008). A compromised or malicious delegate can issue at
  unrestricted volume.
- **Revocation does not invalidate past attestations**: Revoking a delegation
  prevents future issuance but does not revoke attestations already created by
  the delegate. Operators must separately audit and revoke individual
  attestations if delegate compromise is suspected.
- **No multi-level delegation**: A delegate cannot further sub-delegate. The
  `create_attestation_as_delegate` function verifies that `delegator` is a
  registered issuer but does not support chained delegation. Attempts to build
  deeper delegation chains require root-issuer involvement for each level.
- **Attribution is in the audit log, not the attestation**: The attestation
  struct records `issuer = delegator`. The delegate's identity is only
  recoverable from the audit log. Off-chain systems that do not index audit
  entries will see all delegated attestations as if directly issued by the root.

### Neutral

- Delegation records are stored in persistent Soroban storage with the same
  TTL extension logic as attestations; they will not be archived while active.
- `list_delegations_by_delegator` returns only non-expired delegations; expired
  records remain in storage until TTL-based archival.
- The `DelegationCreated` and `DelegationRevoked` events enable off-chain
  indexers to maintain an up-to-date delegation graph.

## Recommended Mitigations for Operators

1. **Minimise delegate key lifetime** — prefer short-lived delegations with an
   explicit `expiration` rather than indefinite authority.
2. **Rotate delegate keys frequently** — treat delegate keys as operational
   credentials, not long-lived secrets.
3. **Monitor `DelegationCreated` events** — flag unexpected new delegations as
   a potential indicator of account compromise at the root issuer level.
4. **Audit after suspected compromise** — if a delegate key is compromised,
   query the audit log for all attestations where `actor = delegate_address`
   and revoke any that cannot be verified as legitimate.
5. **Apply off-chain rate limiting** — because the contract does not rate-limit
   delegate issuance, deploy an off-chain proxy or key-management service that
   enforces per-delegate issuance quotas.
