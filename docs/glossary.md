# TrustLink Glossary

This glossary defines terms that have specific meanings within TrustLink. Many of these terms overlap with general identity or blockchain vocabulary but carry a more precise meaning in this context.

See also: [Integration Guide](./integration-guide.md) | [Storage Layout](./storage-layout.md) | [Security Model](./security.md)

---

## Admin

The single privileged address that controls the TrustLink contract. The admin can register and remove issuers and bridges, configure fees, manage claim types, and upgrade the contract. There is exactly one admin at any point in time. The admin is set during `initialize` and can be transferred with `transfer_admin`.

Contrast with **issuer** (which creates attestations) and **subject** (which is attested about).

---

## Attestation

A signed, on-chain record that a trusted **issuer** has made a **claim** about a **subject** address. An attestation is identified by a deterministic 32-character hex ID derived from the issuer address, subject address, claim type, and creation timestamp.

An attestation can be in one of four states: **Valid**, **Pending**, **Expired**, or **Revoked**. The state is computed at query time from the stored fields and the current ledger timestamp — it is not stored directly.

```
{
  id:           String,          // 32-char hex
  issuer:       Address,
  subject:      Address,
  claim_type:   String,          // e.g. "KYC_PASSED"
  timestamp:    u64,             // creation time (ledger seconds)
  expiration:   Option<u64>,     // optional expiry (ledger seconds)
  revoked:      bool,
  metadata:     Option<String>,
  valid_from:   Option<u64>,     // optional future activation time
  imported:     bool,
  bridged:      bool,
  source_chain: Option<String>,
  source_tx:    Option<String>,
}
```

---

## Attestation ID

A deterministic, collision-resistant 32-character hexadecimal string that uniquely identifies an attestation. Derived from a SHA-256 hash of the issuer address, subject address, claim type, and creation timestamp. Because the same issuer can attest the same claim about the same subject more than once (e.g. after the first attestation expires), the timestamp component ensures a fresh ID on each issuance.

---

## Bridge / Bridge Contract

A Soroban smart contract that has been granted the `bridge` role by the admin. Bridge contracts can call `bridge_attestation` to create attestations that originated on another blockchain. The bridge acts as a trusted relay: it vouches that a given claim was verified on the source chain.

Bridged attestations carry `bridged: true`, a `source_chain` identifier (e.g. `"ethereum"`), and a `source_tx` reference (e.g. the originating transaction hash).

---

## Claim

A statement an **issuer** makes about a **subject**. A claim is typed via a `claim_type` string (e.g. `"KYC_PASSED"`, `"ACCREDITED_INVESTOR"`, `"MERCHANT_VERIFIED"`). A single subject may hold many claims from different issuers, of different types, and with different expiry dates.

---

## Claim Type

A string identifier that categorizes a **claim**. Claim types are registered by the admin using `register_claim_type` with a human-readable description. Consuming contracts and frontends look up attestations by claim type.

Well-known claim types used in TrustLink examples:

| Identifier | Meaning |
| --- | --- |
| `KYC_PASSED` | Subject has completed Know Your Customer verification |
| `ACCREDITED_INVESTOR` | Subject qualifies as an accredited investor |
| `MERCHANT_VERIFIED` | Subject is a verified merchant |
| `AML_CLEARED` | Subject has passed Anti-Money Laundering screening |

Custom claim types can be registered by the admin for application-specific purposes.

---

## Delegation

The ability for an **issuer** to authorize another address to act on their behalf. TrustLink does not currently implement general delegation, but the **multi-signature attestation** flow (`propose_attestation` / `cosign_attestation`) allows a group of signers to collectively authorize an attestation, which achieves a similar effect for shared-custody issuers.

---

## Endorsement

An informal term for an **attestation** in contexts that emphasize the trust relationship: an issuer *endorses* a subject by attesting a claim about them. The two terms are interchangeable in TrustLink documentation.

---

## Expiration

An optional future timestamp (in ledger seconds) after which an attestation is considered **Expired** and no longer satisfies `has_valid_claim`. An attestation with no expiration remains valid indefinitely (until revoked).

`has_valid_claim` and `get_attestation_status` detect expiration lazily at query time and emit an `attestation_expired` event when an expired attestation is first encountered.

---

## Fee

An optional token-denominated charge that the admin can configure using `set_fee`. When a fee is active, callers of `create_attestation` must hold sufficient balance of the configured fee token; the fee is transferred from the issuer to the `fee_collector` address before the attestation is persisted.

---

## Historical Import

An attestation created via `import_attestation` that records a trust decision made off-chain or before TrustLink was deployed. Imported attestations carry `imported: true` and use a caller-supplied `timestamp` (the original verification date) rather than the current ledger time. Only the admin can call `import_attestation`.

---

## Issuer

An address that has been granted the `issuer` role by the admin and is therefore permitted to call `create_attestation`, `revoke_attestation`, and related functions. Issuers are the entities that perform off-chain verification (KYC, AML checks, merchant onboarding, etc.) and then record the result on-chain as an attestation.

An issuer cannot attest on behalf of another issuer without explicit admin approval. See also **bridge** for cross-chain issuance.

---

## Ledger Timestamp

The `Env::ledger().timestamp()` value in Soroban, expressed as Unix seconds. TrustLink uses ledger timestamps (not block numbers) for attestation creation time, expiration, and `valid_from` fields. Ledger timestamps are deterministic and available on-chain, making them safe for smart contract time comparisons.

---

## Metadata

An optional free-form string field on an attestation that the issuer can use to attach additional context — for example, a jurisdiction code, a reference to an external document, or structured JSON. TrustLink does not interpret this field; consuming contracts and indexers can parse it as needed.

---

## Multi-Signature Attestation

An attestation that requires approval from multiple signers before it is finalized. The flow is:

1. One signer calls `propose_attestation` — creates a `MultisigProposal` with a threshold.
2. Other signers call `cosign_attestation` to add their approval.
3. When the number of approvals reaches the threshold, the attestation is automatically created and the proposal is consumed.

This feature supports shared-custody issuers and governance-controlled trust decisions.

---

## Pending

An attestation status indicating that the current ledger time is before the attestation's `valid_from` field. A pending attestation has been issued but is not yet active. `has_valid_claim` returns `false` for pending attestations.

---

## Revocation

The act of marking an attestation as no longer valid by calling `revoke_attestation`. Revoked attestations are not deleted from storage — they remain with `revoked: true` to preserve audit history. `has_valid_claim` returns `false` for revoked attestations, and `get_attestation_status` returns `Revoked`.

Only the issuer who created an attestation can revoke it (or the admin, in emergency cases via batch operations).

---

## Subject

The address that an **attestation** is *about*. The subject is typically a user's wallet address. A subject can hold many attestations from different issuers and of different claim types.

Subjects do not need to take any action to receive an attestation — the issuer creates it unilaterally. Subjects can query their own attestations using `get_subject_attestations`.

---

## TTL (Time-to-Live)

The number of ledger entries remaining before a storage key is evicted from the Soroban ledger. TrustLink refreshes every storage key's TTL to 30 days (518,400 ledgers) whenever it is written. Keys that are never written again will eventually be evicted; consuming contracts should not assume that a key persists forever without activity.

See [docs/storage-layout.md](./storage-layout.md) for full TTL extension behavior.

---

## valid_from

An optional field on an attestation that sets a future activation time. Until the ledger timestamp reaches `valid_from`, the attestation has status **Pending** and does not satisfy `has_valid_claim`. This allows issuers to pre-issue attestations that become active at a scheduled future time.

---

## Version

A semver string stored in instance storage that identifies the deployed contract version. Currently `"1.0.0"`. Readable via `get_version`. Used by indexers and monitoring tools to detect upgrades.
