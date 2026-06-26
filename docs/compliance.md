# GDPR Compliance in TrustLink

## Overview

TrustLink operates on a public blockchain where all on-chain data is permanently
visible. This document explains how TrustLink addresses GDPR's right to erasure
("right to be forgotten") within those constraints, and what obligations
integrators must be aware of.

## Right to Erasure — `request_deletion`

Subjects may call `request_deletion(subject, attestation_id)` to request removal
of their attestation data. The function:

1. Requires authentication from the subject (only the subject can delete their own attestation).
2. Sets a `deleted: true` flag on the attestation record (soft-delete).
3. Removes the attestation ID from the subject's on-chain index so it no longer
   appears in any query (`has_valid_claim`, `get_subject_attestations`, etc.).
4. Emits a `DeletionRequested` event for off-chain compliance audit trails.

```rust
contract.request_deletion(&subject, &attestation_id);
```

### What "Deleted" Means On-Chain

Blockchain storage is immutable — the raw attestation record cannot be physically
erased from ledger history. The `deleted` flag achieves functional erasure:

- The attestation is **invisible to all public query functions**.
- `has_valid_claim`, `has_any_claim`, `has_all_claims`, `get_attestation_by_type`,
  `get_valid_claims`, and `get_attestations_by_tag` all skip deleted attestations.
- The subject index no longer contains the attestation ID.
- The raw record remains accessible only via `get_attestation(id)` if the caller
  already knows the ID — this is equivalent to the immutable ledger history that
  cannot be removed from any blockchain.

### Limitations

- **Historical ledger data**: Soroban ledger history is public and immutable.
  Anyone who observed the original `AttestationCreated` event or stored the
  attestation ID off-chain can still retrieve the raw record via `get_attestation`.
  This is an inherent property of public blockchains.
- **Off-chain indexes**: Any indexer or dApp that cached attestation data before
  deletion must honour the `DeletionRequested` event and purge its own copy.

## Off-Chain Compliance Obligations for Integrators

Integrators who index TrustLink events or cache attestation data **must**:

1. **Subscribe to `DeletionRequested` events** (`topics[0] == "del_req"`) and
   delete the corresponding records from any off-chain database or cache.
2. **Not re-surface deleted attestations** in user-facing interfaces after
   receiving a deletion event.
3. **Retain the deletion event itself** as part of the compliance audit trail —
   the event proves the deletion request was processed.

### Event Format

```
topics: ["del_req", subject_address]
data:   (attestation_id, timestamp)
```

## Right to Rectification — GDPR Article 16

GDPR Article 16 grants data subjects the right to have inaccurate personal data
corrected. TrustLink attestations are immutable by design (see
[ADR-003](adr/ADR-003-immutable-history.md)): once an attestation is written to
the ledger it cannot be edited in place. Rectification is therefore achieved
through a **revoke-and-reissue** workflow.

### Why immutability applies here

An attestation record is a verifiable claim made by an issuer at a point in
time. Allowing silent in-place edits would undermine the non-repudiation
property that makes attestations trustworthy. The historical record of what was
claimed, and when, must remain intact. Rectification replaces the incorrect
claim with a new, correct one while preserving the full audit trail.

### Recommended revoke-and-reissue workflow

When a subject requests correction of inaccurate attestation metadata or claim
data, the operator should follow these steps:

1. **Validate the rectification request** — confirm the subject's identity and
   document the nature of the inaccuracy and the corrected data.

2. **Revoke the incorrect attestation**:
   ```rust
   contract.revoke_attestation(&issuer, &attestation_id, Some("Rectification: data corrected on subject request"));
   ```
   This sets `revoked: true` on the record and emits a `RevocationEvent`, which
   off-chain systems must honour. The revocation reason should reference the
   rectification request for audit purposes.

3. **Issue a corrected attestation**:
   ```rust
   contract.create_attestation(&issuer, &subject, &claim_type, expiration, Some(corrected_metadata));
   ```
   The new attestation receives a new deterministic ID and timestamp.

4. **Notify the subject** — provide the new attestation ID so relying parties
   the subject interacts with can update their records.

5. **Update off-chain indexes** — any indexer or dApp that cached the original
   attestation must subscribe to `RevocationEvent` (`topics[0] == "revoke"`) and
   replace cached records with the new attestation.

### Limitations

- **Historical ledger data**: The original (incorrect) attestation record
  remains visible in Soroban ledger history to anyone who already holds the
  attestation ID. This is an inherent property of public blockchains and does
  not affect functional correctness — the revoked attestation is rejected by all
  validity-checking functions.
- **Metadata scope**: Rectification via reissue only applies to data stored in
  the `metadata` field or claim type. The subject and issuer addresses are
  structural identifiers and cannot be changed; if either needs to change, a new
  attestation to a new address is the only option.
- **Off-chain data**: Operators who stored personal data in off-chain systems
  and used TrustLink only as a reference anchor must independently apply
  rectification to those systems. TrustLink does not propagate corrections to
  external data stores.

### Operator workflow checklist

| Step | Action | Contract call |
|------|--------|---------------|
| 1 | Verify subject identity and document the inaccuracy | — |
| 2 | Revoke the incorrect attestation with a descriptive reason | `revoke_attestation(issuer, id, reason)` |
| 3 | Issue the corrected attestation | `create_attestation(issuer, subject, claim_type, ...)` |
| 4 | Record the old and new attestation IDs in your compliance log | — |
| 5 | Notify the subject of the new attestation ID | — |

## Data Minimisation

TrustLink stores only the data necessary for attestation verification:

- Issuer and subject addresses (pseudonymous on-chain identifiers).
- Claim type (e.g. `KYC_PASSED`) — a category label, not personal data itself.
- Optional metadata string (max 256 characters) — integrators should avoid
  storing personal data in this field.
- Timestamps and status flags.

Integrators should avoid placing personal data (names, email addresses, document
numbers) in the `metadata` field. Use off-chain storage for sensitive personal
data and store only a reference or hash in `metadata`.

## Lawful Basis for Processing

TrustLink itself does not determine the lawful basis for processing — that
responsibility lies with the issuer and the integrating application. Common
lawful bases include:

- **Consent**: Subject explicitly consents to KYC verification as part of
  onboarding.
- **Legitimate interest**: AML/sanctions screening required by financial
  regulations.
- **Legal obligation**: Regulatory requirements (e.g. MiCA, FATF Travel Rule).

Issuers must document their lawful basis before creating attestations about
EU/EEA data subjects.

## Data Retention

TrustLink provides two distinct mechanisms that govern how long attestation data
persists. Operators must understand both to comply with their jurisdiction's data
retention laws.

### Mechanism 1 — Attestation expiration (`expiration` field)

Every attestation may carry an optional `expiration` Unix timestamp. Once that
timestamp passes, the attestation is treated as invalid by all claim-checking
functions (`has_valid_claim`, `get_valid_claims`, etc.) even though the record
remains in storage. This is a **logical** expiry: the data is still present on
the ledger but no longer surfaced to relying parties.

Set expiration at issuance time:

```rust
let expiration_unix = env.ledger().timestamp() + (365 * 24 * 60 * 60); // 1 year
contract.create_attestation(&issuer, &subject, &claim_type, Some(expiration_unix), metadata);
```

After expiry, if a subject also wants the record removed from queries,
`request_deletion` should be called to set the `deleted` flag (see
Right to Erasure section above).

### Mechanism 2 — Soroban ledger TTL (`TtlConfig`)

Soroban persistent storage entries are archived to off-chain cold storage when
they have not been accessed for longer than their configured TTL (time-to-live).
TrustLink uses a contract-level `TtlConfig { ttl_days: u32 }` to set the ledger
entry TTL for all attestation records. The contract automatically extends the
TTL of any record it reads or writes, resetting the archival countdown.

The admin configures this at initialisation:

```rust
contract.initialize(&admin, Some(ttl_days));
```

The default is **30 days**. Ledger archival is **not** the same as deletion —
archived entries can be restored via Stellar's state-archival mechanism. For
compliance purposes, ledger TTL should be treated as a cache lifetime, not a
deletion policy. True functional expiry is controlled by the `expiration` field
(Mechanism 1) and the `deleted` flag (right-to-erasure).

### Recommended retention periods by claim type

The following periods reflect common regulatory and business requirements.
Operators should review their specific jurisdictional obligations before
deploying.

| Claim Type           | Suggested Expiration | Rationale                                      |
|----------------------|----------------------|------------------------------------------------|
| `KYC_PASSED`         | 1–2 years            | FATF guidance; most KYC regimes require refresh |
| `ACCREDITED_INVESTOR`| 1 year               | SEC Rule 506(c); annual re-verification common  |
| `AML_CLEARED`        | 6–12 months          | Risk-based AML policies; sanctions lists update |
| `SANCTIONS_CHECKED`  | 3–6 months           | Sanctions lists change frequently               |
| `MERCHANT_VERIFIED`  | 1–2 years            | PCI/merchant agreements typically annual        |

### Configuring TTL for jurisdictional compliance

Different jurisdictions impose different maximum and minimum retention periods.
Operators should configure the attestation `expiration` field — not the ledger
TTL — as their primary retention control, since ledger TTL only affects cold
storage archival, not data validity.

**GDPR (EU/EEA)** — GDPR's storage limitation principle (Article 5(1)(e))
requires that personal data not be retained longer than necessary. Set
`expiration` to the shortest period consistent with the lawful basis for
processing. For KYC attestations backed by consent, re-obtain consent and
reissue before expiry rather than issuing indefinitely-valid attestations.

**CCPA (California)** — No mandated retention period, but data must be
retained no longer than disclosed in the privacy notice. Align `expiration`
with the retention period stated in your privacy disclosure.

**FATF / Travel Rule jurisdictions** — Most financial-intelligence frameworks
require retaining transaction records for 5 years. Because TrustLink stores
claim status (not the underlying transaction), the attestation expiration should
outlive the underlying transaction if the attestation itself is part of the
compliance record.

**Operator workflow for TTL compliance**

1. Determine the maximum retention period required by the applicable regulation.
2. Set the `expiration` field on new attestations to that maximum period.
3. Subscribe to `DeletionRequested` events and delete off-chain copies promptly.
4. After an attestation expires, call `request_deletion` to suppress it from
   all on-chain queries (completing the functional erasure).
5. Review and refresh attestations for subjects with ongoing relationships before
   their `expiration` passes to avoid service interruption.

### Ledger TTL vs. attestation expiration — summary

| Property           | Attestation `expiration` field         | Soroban ledger TTL (`ttl_days`)         |
|--------------------|----------------------------------------|-----------------------------------------|
| Controls           | Logical validity of the attestation    | When the entry is archived to cold store |
| Set by             | Issuer at creation time                | Admin at contract initialisation         |
| Effect on queries  | Expired = invalid in all query fns     | No effect on validity; entry is archived |
| Compliance role    | Primary retention policy control       | Infrastructure cache-lifetime only       |
| Default            | None (no expiry unless set)            | 30 days                                  |

## Summary of GDPR-Relevant Contract Functions

| Function | GDPR Relevance |
|---|---|
| `request_deletion(subject, id)` | Right to erasure — soft-deletes attestation and removes from index |
| `revoke_attestation(issuer, id, reason)` | Invalidates attestation without deletion |
| `get_attestation(id)` | Returns raw record; deleted flag visible to caller |
| `has_valid_claim(subject, claim_type)` | Skips deleted attestations |
| `DeletionRequested` event | Audit trail for off-chain compliance systems |
