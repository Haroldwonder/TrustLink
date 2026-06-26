# Sanctions and PEP Screening Integration

## Overview

TrustLink's KYC examples attest identity verification, but regulated fintech
issuers also need to screen subjects against sanctions lists and politically-
exposed-person (PEP) databases before issuing attestations. This document
describes the recommended pattern: **screen off-chain, attest on-chain**, and
how to model a "screening passed on date X" claim without storing any
screening-provider data on the ledger.

## Why screen off-chain

Sanctions and PEP screening produces sensitive data that must not be stored
on a public blockchain:

- **Provider data is confidential.** Screening results, match scores, and
  list references are licensed data from providers such as OFAC, Refinitiv,
  LexisNexis, or Dow Jones. Publishing them on-chain would violate licensing
  agreements.
- **Hit details are personal data.** Reasons for a match (or near-match) are
  sensitive information about the subject that would be permanently and publicly
  visible on the ledger, creating GDPR/CCPA liability.
- **False positives require confidential handling.** A false-positive screening
  result that is published on-chain could damage a subject's reputation and
  expose the issuer to legal risk.

The on-chain attestation records only that screening *passed* — not the
provider, list version, or any detail of what was checked.

## Recommended workflow

```
Off-chain                              On-chain
──────────────────────────────────     ───────────────────────────────────
1. Receive subject's identity data
2. Call sanctions/PEP provider API
3. Review any matches
      │
      ├─ PASS ──────────────────────►  4. create_attestation(
      │                                      issuer, subject,
      │                                      "SANCTIONS_CHECKED",
      │                                      expiration,
      │                                      metadata
      │                                   )
      │
      └─ HIT / INCONCLUSIVE ────────►  Do NOT create attestation.
                                        Log internally for compliance team.
                                        Reject onboarding or escalate.
```

### Step-by-step

1. **Collect identity data off-chain.** Gather the subject's name, date of
   birth, nationality, and address from your KYC documents. Do not put these
   in TrustLink.

2. **Call your screening provider.** Submit the identity data to your provider
   (e.g. Refinitiv World-Check, OFAC SDN list, UN Consolidated List). Retain
   the full API response in your internal compliance system.

3. **Evaluate results.** Apply your risk policy to any matches:
   - Clear pass (no matches or confirmed false positives) → proceed.
   - Confirmed hit → reject, log to your compliance system, do not attest.
   - Escalation (possible match) → route to compliance officer before
     proceeding.

4. **Issue the attestation (pass only).**
   ```typescript
   const screenedAt = new Date().toISOString();
   const metadata = JSON.stringify({
     provider: "acme_aml",          // provider name — NOT the result details
     screened_at: screenedAt,
     list_version: "OFAC-20240115", // list snapshot identifier, not the list
   });

   await client.createAttestation(
     issuerKeypair,
     subjectAddress,
     "SANCTIONS_CHECKED",
     expirationTimestamp,
     metadata
   );
   ```

5. **Retain the full screening record off-chain.** Store the provider response,
   the list version checked, the operator who reviewed any matches, and the
   decision rationale in your internal compliance database. This record must be
   retained per your jurisdiction's requirements (typically 5 years for
   AML/CFT compliance).

## Claim type design

Use separate claim types for sanctions and PEP screening so that verifiers
can distinguish the two:

| Claim type | Meaning |
|---|---|
| `SANCTIONS_CHECKED` | Subject was screened against applicable sanctions lists and no confirmed match was found at the time of issuance. |
| `PEP_CHECKED` | Subject was screened against PEP databases; either no match found or PEP status accepted under enhanced due diligence. |
| `AML_CLEARED` | Broader AML check passed, which may encompass both of the above. Use if your provider combines sanctions and PEP in one check. |

Verifiers can gate access with:

```typescript
const sanctionsOk = await client.hasValidClaim(subject, "SANCTIONS_CHECKED");
const pepOk       = await client.hasValidClaim(subject, "PEP_CHECKED");

if (!sanctionsOk || !pepOk) {
  throw new Error("Required screening attestations missing or expired");
}
```

## Metadata conventions

The `metadata` field (max 256 characters) should record **what was checked**,
not **what was found**:

```json
{
  "provider": "acme_aml",
  "screened_at": "2024-01-15T10:30:00Z",
  "list_version": "OFAC-20240115"
}
```

Do **not** include:

- Match scores or similarity percentages.
- Names of lists that produced hits (implies a near-miss was found).
- Any detail that indicates why a result was a pass.
- Provider-specific identifiers that could be reverse-engineered to retrieve
  the result from the provider.

The metadata is publicly visible on-chain. Treat it as a receipt, not a report.

## Expiration policy

Sanctions lists are updated frequently — sometimes daily. Attestations must
expire and be re-screened regularly to remain meaningful:

| Claim type | Recommended expiration | Rationale |
|---|---|---|
| `SANCTIONS_CHECKED` | 90 days | OFAC/UN lists update frequently; risk of post-issuance listing |
| `PEP_CHECKED` | 180 days | PEP status changes less frequently but should be refreshed |
| `AML_CLEARED` | 90–180 days | Follow your jurisdiction's risk-based approach |

Set expiration at issuance:

```typescript
const ninetyDays = 90 * 24 * 60 * 60;
const expiration = BigInt(Math.floor(Date.now() / 1000) + ninetyDays);

await client.createAttestation(issuer, subject, "SANCTIONS_CHECKED", expiration, metadata);
```

After expiration, `has_valid_claim` returns `false` and downstream verifiers
will require the subject to be re-screened before re-attestation.

## Post-issuance hits

If your screening provider or a regulatory body notifies you of a match for a
subject after you have already issued a `SANCTIONS_CHECKED` attestation:

1. **Immediately revoke the attestation** with a descriptive internal reason:
   ```typescript
   await client.revokeAttestation(
     issuerKeypair,
     attestationId,
     "Post-issuance sanctions match — subject under review"
   );
   ```

2. **Suspend the subject's access** in your application until the investigation
   concludes.

3. **File a Suspicious Activity Report (SAR)** if required by your
   jurisdiction.

4. Do **not** re-issue `SANCTIONS_CHECKED` until the match is fully resolved
   and documented.

## Jurisdictional notes

| Jurisdiction | Relevant framework | Key requirement |
|---|---|---|
| United States | OFAC / Bank Secrecy Act | Screen against SDN list; report matches to FinCEN |
| European Union | 6AMLD / EU Sanctions Regulation | Screen against EU Consolidated List; apply enhanced due diligence for PEPs |
| United Kingdom | UK Sanctions List / MLR 2017 | Screen against OFSI Consolidated List; annual PEP re-screening |
| FATF members | FATF Recommendations 6, 12 | Risk-based approach; document screening methodology and frequency |

TrustLink does not determine which lists must be checked — that obligation
rests with the issuing entity based on the subject's risk profile and
applicable regulations.

## Summary

| Principle | Implementation |
|---|---|
| Screen before attesting | Call provider API; only call `create_attestation` on a confirmed pass |
| Nothing sensitive on-chain | Metadata records provider name and list version — never hit details |
| Short expiry | 90 days for sanctions; re-screen and re-issue before expiry |
| Revoke on post-issuance hit | Call `revoke_attestation` immediately; do not wait for natural expiry |
| Full record off-chain | Retain provider response, reviewer notes, and decision rationale in your compliance system |
