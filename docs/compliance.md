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

Attestations support optional expiration (`expiration` field). Issuers should
set appropriate expiration times aligned with their data retention policy rather
than creating indefinite attestations.

Recommended retention periods by claim type:

| Claim Type           | Suggested Expiration |
|----------------------|----------------------|
| `KYC_PASSED`         | 1–2 years            |
| `ACCREDITED_INVESTOR`| 1 year               |
| `AML_CLEARED`        | 6–12 months          |
| `SANCTIONS_CHECKED`  | 3–6 months           |
| `MERCHANT_VERIFIED`  | 1–2 years            |

## Summary of GDPR-Relevant Contract Functions

| Function | GDPR Relevance |
|---|---|
| `request_deletion(subject, id)` | Right to erasure — soft-deletes attestation and removes from index |
| `revoke_attestation(issuer, id, reason)` | Invalidates attestation without deletion |
| `get_attestation(id)` | Returns raw record; deleted flag visible to caller |
| `has_valid_claim(subject, claim_type)` | Skips deleted attestations |
| `DeletionRequested` event | Audit trail for off-chain compliance systems |
