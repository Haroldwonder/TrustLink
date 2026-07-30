# Freelance Marketplace Reputation Example

Demonstrates composing TrustLink's **endorsement system** and **IssuerTier system** to derive a weighted reputation score for a freelancer — a real-world pattern for marketplace trust.

## Scenario

Three marketplace clients are registered as TrustLink issuers at different trust tiers:

| Client | IssuerTier | Weight |
|--------|-----------|--------|
| Client A | Basic | 1 pt / endorsement |
| Client B | Verified | 2 pts / endorsement |
| Client C | Premium | 3 pts / endorsement |

Each client issues a `JOB_COMPLETED` attestation to the freelancer after project delivery. They then cross-endorse one another's attestations. The platform aggregates endorsements and weights them by tier to produce a reputation score.

### Score formula

```
for each JOB_COMPLETED attestation:
  for each endorsement of that attestation:
    score += TIER_WEIGHTS[endorser.tier]

total_reputation = sum of scores across all attestations
```

### Example outcome

| Attestation | Endorsers | Score |
|-------------|-----------|-------|
| Client A's attestation | Client B (Verified=2) + Client C (Premium=3) | 5 |
| Client B's attestation | Client C (Premium=3) | 3 |
| Client C's attestation | (none) | 0 |
| **Total** | | **8** |

A freelancer whose work is endorsed by Premium-tier clients ranks higher than one with the same endorsement count from Basic-tier clients.

## Prerequisites

1. A deployed TrustLink contract on Stellar testnet or mainnet.
2. Three keypairs registered as issuers (`register_issuer`).
3. Admin has set each issuer's tier via `set_issuer_tier`.
4. Each client keypair must have enough XLM to pay transaction fees.

## Setup

```bash
npm install
cp .env.example .env
# Edit .env with your contract ID, freelancer address, and client secrets
```

## Run

```bash
node index.mjs
```

## Key concepts

### Why weight by tier?

Not all issuers carry equal trust. A `Verified` issuer has been vetted more rigorously by the platform than a `Basic` issuer. Weighting endorsements by tier ensures that the reputation signal reflects issuer trustworthiness, not just endorsement volume.

### Why use endorsements instead of a separate rating system?

TrustLink endorsements are on-chain, cryptographically signed, and tied to specific attestations. Unlike off-chain star ratings, they cannot be faked or selectively omitted. Endorsers stake their own issuer reputation when they endorse work attested by a peer.

### Extending the pattern

- Add minimum score thresholds for project eligibility (`score >= 5` to bid on premium contracts).
- Weight by claim recency: decay older attestations' contribution to the score.
- Combine with `has_valid_claim_from_tier` to gate access to high-value work categories for freelancers with at least one Premium-endorsed attestation.
