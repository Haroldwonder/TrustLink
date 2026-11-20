# Endorsements and Confidence Scores

This example demonstrates two distinctive features of the attestation protocol:

1. **Endorsements** — issuers can endorse another issuer's attestation, adding their own credibility to it.
2. **Confidence scores** — the protocol aggregates endorsements into a single `get_confidence_score` value that a verifier can read before and after endorsements are added.

The example has two endorsers endorse a third issuer's attestation, and the verifier reads `get_confidence_score` before and after the endorsements are submitted.

## What is an endorsement?

An endorsement is a statement by one issuer that it vouches for an attestation created by another issuer. Endorsements are recorded on-chain and are weighted by the endorsing issuer's own reputation/confidence. A verifier can therefore ask the protocol: *"How much should I trust this attestation, given who has endorsed it?"* — and get back a single confidence score.

## How `get_confidence_score` is computed

The confidence score for an attestation is derived from:

- The **base confidence** of the attestation itself (set by the original issuer).
- The **endorsements** attached to it, each weighted by the endorsing issuer's confidence.
- **Time decay** — endorsements lose influence as they age, so stale endorsements count for less than fresh ones.

Conceptually:

```
score = base_confidence + Σ (endorsement_weight_i × decay(age_i))
```

where `decay(age)` is a function of how long ago the endorsement was made, controlled by the decay configuration described below.

In the example, the verifier reads the score **before** any endorsements (only the base confidence contributes) and **after** the two endorsers have endorsed the attestation (the score increases).

## Decay configuration

Endorsements are not permanent boosts — they decay over time. The decay is controlled by configuration parameters on the scoring module:

| Parameter | Meaning |
|-----------|---------|
| **Decay rate** | How quickly an endorsement's weight falls off per unit of time. A higher rate means endorsements become stale faster. |
| **Half-life** | The time it takes for an endorsement's weight to drop to half of its initial value. This is an alternative, more intuitive way to express the decay rate (`half_life = ln(2) / decay_rate`). |

### How decay affects confidence over time

- Immediately after an endorsement, its full weight is applied.
- As time passes, the endorsement's contribution shrinks according to the decay curve.
- After several half-lives, the endorsement contributes negligibly to the score.

This means a verifier reading `get_confidence_score` at different times will see the score gradually decrease unless fresh endorsements are added.

### Tuning the decay

- **Short half-life (fast decay):** confidence reflects only recent activity. Good when trust must be continuously re-established (e.g. fast-moving operational contexts). Endorsers must re-endorse frequently to keep scores high.
- **Long half-life (slow decay):** endorsements remain meaningful for a long time. Good for stable, long-lived credentials where re-endorsement is impractical.
- **No decay (disabled):** every endorsement counts at full weight forever. Use only when you explicitly want permanent trust accumulation.

To tune it, set the decay rate / half-life in the scoring configuration when deploying or initializing the scoring module. Choose a half-life that matches how often you expect endorsers to refresh their endorsements — a common rule of thumb is to set the half-life to roughly the expected re-endorsement interval.

## Setup

Install dependencies from the repository root:

```bash
npm install
```

Build the contracts if you have not already:

```bash
npm run build
```

## Running the example

From this directory:

```bash
npx ts-node index.ts
```

Or, if you are running from the repository root:

```bash
npx ts-node examples/endorsements/index.ts
```

### What you will see

The script will:

1. Deploy the required contracts and register three issuers.
2. Have the first issuer create an attestation.
3. Read `get_confidence_score` for that attestation (baseline — no endorsements yet).
4. Have the second and third issuers endorse the attestation.
5. Read `get_confidence_score` again and print the increase.

Example output shape:

```
Confidence score before endorsements: <value>
Endorsement 1 submitted by issuer 2
Endorsement 2 submitted by issuer 3
Confidence score after endorsements:  <value>
```

The score after endorsements should be higher than the baseline, illustrating how endorsements raise confidence and how the decay configuration governs how long that boost lasts.