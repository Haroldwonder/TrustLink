# Endorsements and Confidence Scoring Example

A complete reference implementation demonstrating TrustLink's on-demand **endorsement chaining**, **dynamic confidence scoring** (`get_confidence_score`), and **decay configuration** on the Stellar Soroban smart contract platform.

---

## Overview

In decentralized trust architectures, not all attestations carry equal assurance. Static reputation scores stored directly on attestations suffer from three critical flaws:
1. They cannot reflect subsequent verification or endorsements by independent peer auditors.
2. They do not account for issuer inactivity over extended time periods (credential staleness).
3. They fail to dynamically penalize issuers with a high history of revoked credentials.

TrustLink addresses these challenges via **dynamic on-demand confidence scoring** (`get_confidence_score`), returning a real-time trust score in the range `[0, 100]`.

---

## Scenario Walkthrough

This example simulates a decentralized compliance & DeFi security credential lifecycle:

```
[Primary Issuer]                [Peer Endorser 1]             [Peer Endorser 2]                 [Verifier]
  (Auditor A)                     (CertiK Node)                 (Gauntlet DAO)                (DeFi Lending Pool)
       │                                │                             │                                │
       ├─ create_attestation() ────────┼─────────────────────────────┼────────────────────────────────┤
       │  (Verified Tier: 60 base)     │                             │                                │
       │                                │                             │        get_confidence_score()  │
       │                                │                             │ ◄──────────────────────────────┤
       │                                │                             │  Returns: 60/100 (0 endors.)   │
       │                                │                             │ ──────────────────────────────►│
       │                                │                             │                                │
       │   endorse_attestation() ◄──────┤                             │                                │
       │   (Records endorsement on-chain)                             │                                │
       │                                │                             │        get_confidence_score()  │
       │                                │                             │ ◄──────────────────────────────┤
       │                                │                             │  Returns: 62/100 (+2 points)   │
       │                                │                             │ ──────────────────────────────►│
       │                                │                             │                                │
       │   endorse_attestation() ◄────────────────────────────────────┤                                │
       │   (Records 2nd endorsement on-chain)                         │                                │
       │                                │                             │        get_confidence_score()  │
       │                                │                             │ ◄──────────────────────────────┤
       │                                │                             │  Returns: 64/100 (+4 points)   │
       │                                │                             │ ──────────────────────────────►│
```

1. **Attestation Issuance**: Primary Issuer A (Tier: `Verified`) issues an attestation `att_soc2_vault_98712` for Subject (a DeFi Vault protocol).
2. **Initial Verification**: Verifier queries `get_confidence_score("att_soc2_vault_98712")`. Since Issuer A has `Verified` status and 0 endorsements, the baseline score is **60 / 100**.
3. **Peer Endorsement 1**: Peer Issuer B (an independent auditing firm) reviews the audit evidence and invokes `endorse_attestation`.
   - The contract records the endorsement on-chain.
   - Verifier queries `get_confidence_score` again: Score increases from **60 to 62** (+2 points).
4. **Peer Endorsement 2**: Peer Issuer C (a DeFi risk rating agency) invokes `endorse_attestation`.
   - Verifier queries `get_confidence_score` again: Score increases from **62 to 64** (+4 points cumulative).

---

## Smart Contract Mathematical Formulation

The dynamic score is computed on-demand inside `TrustLinkContract::get_confidence_score` in `src/admin.rs`:

### 1. Base Score (Tier + Endorsements)
```
base_score = tier_score + min(endorsements.len() * 2, 10)
```
- **Tier Base**:
  - `Premium`: **90**
  - `Verified`: **60**
  - `Basic` (default / unregistered): **30**
- **Endorsement Bonus**: Each endorsement adds `+2` points, capped at a maximum of `+10` points (up to 5 endorsements).

### 2. Inactivity Decay (`half_life_days`)
Decay reduces score when an issuer has been inactive (not issued credentials recently):
```
penalty_bps = min(days_inactive * 5000 / half_life_days, 10000)
activity_factor_bps = 10000 - penalty_bps
```
- When `days_inactive == half_life_days`, the penalty is `5000 bps` (50% reduction).
- If `half_life_days == 0`, inactivity decay is disabled (`activity_factor_bps = 10000`).

### 3. Revocation-Ratio Decay (`revocation_weight`)
Decay penalizes issuers who have a track record of revoking issued attestations:
```
ratio_bps = min(revocations * 10000 / total_issued, 10000)
penalty_bps = min(ratio_bps * revocation_weight / 100, 10000)
reputation_factor_bps = 10000 - penalty_bps
```
- `revocation_weight` (0–100) dictates how aggressively the contract penalizes revocations.
- If `revocation_weight == 0`, revocation decay is disabled (`reputation_factor_bps = 10000`).

### 4. Final Compound Score
```
decayed = ((base_score * activity_factor_bps / 10000) * reputation_factor_bps) / 10000
```
- If the attestation is marked `deleted = true` or is not in `AttestationStatus::Valid` state, `get_confidence_score` returns `None` (`null`).

---

## Running the Example

### 1. Run the Flow Simulation
Execute the interactive console simulation demonstrating the full endorsement progression and decay calculations:
```bash
node flow.mjs
```

### 2. Run the Test Suite
Run the 14-point test suite validating all boundary conditions, tier mappings, decay dynamics, and invariant checks using Node's native test runner:
```bash
npm test
```

Expected output:
```
▶ Endorsements & Confidence Scoring Suite
  ▶ 1. Issuer Tier Base Scores
  ▶ 2. Endorsements Progression (Before & After Two Peer Endorsements)
  ▶ 3. Inactivity Decay Dynamics
  ▶ 4. Revocation-Ratio Decay Dynamics
  ▶ 5. Invalidation & Deletion Rules
✔ Endorsements & Confidence Scoring Suite (14 passed)
```

---

## Contract Integration (Soroban Rust Client)

```rust
use trustlink::Client as TrustLinkClient;

let client = TrustLinkClient::new(&env, &contract_id);

// Query confidence score for an attestation
match client.get_confidence_score(&attestation_id) {
    Some(score) if score >= 60 => {
        // High confidence: proceed with operation (e.g. collateral release, uncollateralized loan)
    }
    Some(score) => {
        // Moderate confidence: require additional collateral or multi-sig approval
    }
    None => {
        // Attestation does not exist, expired, revoked, or deleted
    }
}
```
