# DID-style social-recovery / guardian attestation flow

This example demonstrates **account/key recovery** on top of TrustLink: when a
subject loses their signing key, a quorum of trusted **guardians** can attest to
their identity and authorise migrating the subject's attestation history to a
**new address**.

This is a different problem from delegation / sub-issuer authority (ADR-008),
which governs *who may issue*. Here we recover *control of an account*.

```
           register_guardians (M-of-N threshold)
  Subject ───────────────────────────────────────▶ GuardianRecovery

  (subject loses key)

  Guardian_1 ─ initiate_recovery(new_address) ─▶ GuardianRecovery
  Guardian_2 ─ approve_recovery ───────────────▶ GuardianRecovery
                       │ quorum reached
                       ▼
           re-link subject's valid claims ──▶ TrustLink (new_address)
```

## Design note

The core `TrustLinkContract` currently exposes **no** `transfer_attestation` or
multisig primitive. Rather than modify the core contract, this example
implements the guardian set, the M-of-N quorum, and the re-linkage inside a
self-contained `GuardianRecovery` contract that uses only TrustLink's existing
public API:

- `get_valid_claims(subject)` — reads the claims currently held by the subject;
- `create_attestation(...)` — re-issues each claim to the new address. The
  recovery contract is registered as a TrustLink issuer, so it acts as the
  "recovery authority" that vouches for continuity of identity.

## Flow / API

| Function | Caller | Purpose |
|----------|--------|---------|
| `register_guardians(subject, guardians, threshold)` | subject | Set the M-of-N guardian set (while the subject still holds their key) |
| `initiate_recovery(guardian, subject, trustlink, new_address)` | a guardian | Open a recovery request; initiator counts as the first approval |
| `approve_recovery(guardian, subject)` | a guardian | Add an approval; re-linkage runs automatically once the threshold is met |
| `guardians(subject)` / `get_recovery(subject)` | anyone | Read guardian set / in-flight request |

Re-linkage only ever copies **valid** claims (`get_valid_claims` skips revoked
and expired attestations), and duplicate or non-guardian approvals are rejected.

## Running

```bash
# from the repo root
cargo run --example guardian-recovery

# or use the wrapper script (runnable from anywhere)
./examples/guardian-recovery/run.sh
```

The example `assert!`s each step, so it doubles as an end-to-end validation
script. Expected output:

```
== DID-style guardian social-recovery demo ==
[1] subject holds KYC_PASSED + ACCREDITED_INVESTOR
[2] subject registers 2-of-3 guardians
[3] guardian g1 initiates recovery       -> 1/2 approvals, not yet re-linked
[4] non-guardian approval rejected
[5] guardian g2 approves                 -> 2/2 quorum, claims re-linked
[6] new address now holds all 2 claims

All checks passed — guardian recovery re-linked attestations end-to-end.
```

## What the scenario proves

- Guardian registration with an M-of-N threshold (2-of-3 here).
- A recovery request below quorum does **not** migrate anything.
- Non-guardians cannot approve.
- Reaching quorum re-links all of the subject's valid claims to the new address.
