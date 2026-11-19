# Requirements Document: Attestation Confidence Score

## Introduction

Trust and assurance levels in decentralized identity ecosystems depend on multiple dynamic factors: the issuer's trust tier, community endorsements on the attestation, issuer activity recency, and the issuer's revocation history. Rather than storing a static score on the attestation struct, TrustLink computes a dynamic confidence score (0–100) at query time via `get_confidence_score(env, attestation_id)`.

This feature provides verifiers and consumers with an algorithmic confidence metric evaluating the overall trustworthiness of an attestation based on on-chain signals.

## Glossary

- **Attestation**: An on-chain record created by a registered issuer asserting a claim about a subject.
- **Confidence_Score**: A computed dynamic score in the range [0, 100] returned by `get_confidence_score`.
- **Issuer_Tier**: A trust tier assigned to an issuer (`Basic` = 30, `Verified` = 60, `Premium` = 90). Unset tier defaults to `Basic`.
- **Endorsement**: An on-chain endorsement of an attestation by other authorized issuers, adding up to 10 points (+2 points per endorsement).
- **Decay_Config**: Contract-wide configurable parameters (`half_life_days`, `revocation_weight`) governing score decay.
- **Inactivity_Decay**: Reduction in confidence score based on the elapsed time since the issuer's last attestation issuance.
- **Revocation_Decay**: Reduction in confidence score proportional to the issuer's historical revocation ratio (`revocations / total_issued`), scaled by `revocation_weight`.

---

## Requirements

### Requirement 1: Confidence Score Query

**User Story:** As an integrator or verifier, I want to query the dynamic confidence score of an attestation, so that I can evaluate its assurance level based on up-to-date issuer and endorsement metrics.

#### Acceptance Criteria

1. WHEN `get_confidence_score(env, attestation_id)` is called for an existing attestation, THE TrustLink SHALL return `Some(score)` where `score` is a `u32` in the range [0, 100].
2. WHEN `get_confidence_score(env, attestation_id)` is called for a non-existent attestation ID, THE TrustLink SHALL return `None`.
3. THE `get_confidence_score` function SHALL compute base score from the issuer's tier and the attestation's endorsement count.
4. THE `get_confidence_score` function SHALL apply inactivity decay and revocation ratio decay based on the contract's `DecayConfig`.

---

### Requirement 2: Issuer Tier Scoring

**User Story:** As an admin, I want to assign trust tiers to issuers, so that attestations from higher-reputation issuers have higher baseline confidence scores.

#### Acceptance Criteria

1. THE `IssuerTier` SHALL support three levels: `Basic` (tier score = 30), `Verified` (tier score = 60), and `Premium` (tier score = 90).
2. WHEN an issuer has no tier explicitly set, THE TrustLink SHALL default to `Basic` (tier score = 30).
3. WHEN `set_issuer_tier(env, admin, issuer, tier)` is called by an authorized admin, THE TrustLink SHALL update the issuer's tier and emit an `issuer_tier_updated` event.
4. WHEN `get_issuer_tier(env, issuer)` is called, THE TrustLink SHALL return `Some(tier)` if set, or `None` if not set.

---

### Requirement 3: Endorsement Score Bonus

**User Story:** As a consumer, I want attestations with third-party endorsements to receive a confidence score bonus.

#### Acceptance Criteria

1. EACH endorsement on the attestation SHALL add 2 points to the base score.
2. THE endorsement bonus SHALL be capped at a maximum of 10 points (5 endorsements).
3. Base score (`tier_score + endorsement_score`) SHALL NOT exceed 100 before decay.

---

### Requirement 4: Inactivity & Revocation Decay

**User Story:** As a protocol designer, I want confidence scores to decay if an issuer is inactive or has a high revocation rate, so that stale or unreliable issuers are accurately reflected.

#### Acceptance Criteria

1. WHEN `DecayConfig.half_life_days > 0`, THE confidence score SHALL decrease linearly based on `days_inactive` since the issuer's last issuance: `penalty_bps = min(10000, days_inactive * 5000 / half_life_days)`.
2. WHEN `DecayConfig.revocation_weight > 0` and the issuer has issued attestations, THE confidence score SHALL decrease based on the ratio of revoked attestations: `penalty = min(10000, (revocations * 10000 / total_issued) * revocation_weight / 100)`.
3. WHEN `set_decay_config(env, admin, config)` is called by an authorized admin, THE TrustLink SHALL persist the new decay parameters.
4. WHEN `get_decay_config(env)` is called, THE TrustLink SHALL return the configured `DecayConfig`, or the default (`half_life_days: 90, revocation_weight: 50`) if unset.
