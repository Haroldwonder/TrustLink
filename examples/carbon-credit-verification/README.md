# Carbon-credit / ESG offset attestation — verifier-of-verifiers example

This example demonstrates a **"chain of trust about trust"** built on TrustLink:
a registry attests to an auditor's credibility, and that auditor in turn attests
to a specific carbon-offset project's claim. A verifier then validates the *full
chain* before anyone trusts the offset.

```
  Registry ──AUDITOR_ACCREDITED──▶ Auditor ──OFFSET_VERIFIED──▶ Project
  (root of trust)                  (accredited)                 (offset claim)
```

Unlike a single-party provenance model (one issuer vouching directly for a
subject), a carbon-offset claim is only trustworthy when **both** links hold —
which is exactly what the growing market for verifiable ESG / carbon-credit
claims requires.

## Claim types

| Claim type          | Issued by | Subject | Meaning                                            |
|---------------------|-----------|---------|----------------------------------------------------|
| `AUDITOR_ACCREDITED`| Registry  | Auditor | The registry has vetted this auditor's credibility |
| `OFFSET_VERIFIED`   | Auditor   | Project | The auditor has verified this project's offset      |

## The verifier

`CarbonVerifier::verify_offset(trustlink, project, expected_registry)` walks the
chain in reverse using cross-contract calls into TrustLink:

1. **Tier 1** — read the project's most-recent *valid* `OFFSET_VERIFIED`
   attestation. Its issuer is the auditor being trusted.
2. **Tier 2** — that auditor must itself hold a *valid* `AUDITOR_ACCREDITED`
   attestation **issued by `expected_registry`** (the root of trust).

Because TrustLink's `get_attestation_by_type` only ever returns a `Valid`
attestation, a revoked or expired link *anywhere* in the chain makes the whole
verification fail automatically.

## Running

```bash
# from the repo root
cargo run --example carbon-credit-verification

# or use the wrapper script (runnable from anywhere)
./examples/carbon-credit-verification/run.sh
```

The example `assert!`s each step, so it doubles as an end-to-end validation
script — a non-zero exit code means the trust chain failed to validate. Expected
output:

```
== Carbon-credit verifier-of-verifiers demo ==
[1] no attestations yet            -> untrusted  (ok)
[2] registry accredits auditor
[3] auditor verifies offset
[4] full accredited chain          -> TRUSTED    (ok)
[5] newest offset from rogue auditor -> untrusted (ok)
[6] auditor accreditation revoked  -> untrusted  (ok)

All checks passed — verifier-of-verifiers chain validated end-to-end.
```

## What the scenario proves

- A complete `project → auditor → registry` chain verifies as **trusted**.
- An offset verified by an **unaccredited** auditor is rejected — trust cannot be
  laundered through a fresh signer.
- **Revoking** the auditor's accreditation at the registry breaks the chain, so
  previously-valid offsets from that auditor stop verifying.
