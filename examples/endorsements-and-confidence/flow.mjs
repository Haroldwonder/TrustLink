#!/usr/bin/env node
/**
 * TrustLink Endorsements & Confidence Scoring Example Flow
 *
 * Demonstrates:
 * 1. Primary Issuer creates a verifiable attestation (e.g. SOC2_COMPLIANCE).
 * 2. Verifier queries dynamic `get_confidence_score` before any endorsements.
 * 3. Two independent peer issuers endorse the attestation.
 * 4. Verifier queries `get_confidence_score` after each endorsement to observe score increases.
 * 5. Explains and simulates DecayConfig behavior (inactivity decay & revocation-ratio decay).
 *
 * Run with:
 *   node flow.mjs
 */

export const IssuerTier = {
  Basic: "Basic",
  Verified: "Verified",
  Premium: "Premium",
};

export const TIER_BASE_SCORES = {
  [IssuerTier.Basic]: 30,
  [IssuerTier.Verified]: 60,
  [IssuerTier.Premium]: 90,
};

export const SECS_PER_DAY = 86400;
export const MAX_ENDORSEMENT_POINTS = 10;
export const POINTS_PER_ENDORSEMENT = 2;

/**
 * Computes the dynamic confidence score mirroring TrustLink Soroban contract `get_confidence_score`.
 *
 * Formula:
 *   tier_score = Basic (30) | Verified (60) | Premium (90)
 *   endorsement_score = min(endorsements.len() * 2, 10)
 *   base_score = tier_score + endorsement_score
 *
 * Inactivity decay (if half_life_days > 0):
 *   penalty_bps = min(days_inactive * 5000 / half_life_days, 10000)
 *   activity_factor_bps = 10000 - penalty_bps
 *
 * Revocation decay (if revocation_weight > 0):
 *   ratio_bps = min(revocations * 10000 / total_issued, 10000)
 *   penalty_bps = min(ratio_bps * revocation_weight / 100, 10000)
 *   reputation_factor_bps = 10000 - penalty_bps
 *
 * Final Score:
 *   (base_score * activity_factor_bps / 10000) * reputation_factor_bps / 10000
 */
export function computeConfidenceScore({
  issuerTier = IssuerTier.Basic,
  endorsementCount = 0,
  daysInactive = 0,
  halfLifeDays = 0,
  totalIssued = 1,
  revocations = 0,
  revocationWeight = 0,
  isDeleted = false,
  isValid = true,
}) {
  if (isDeleted || !isValid) {
    return null;
  }

  const tierScore = TIER_BASE_SCORES[issuerTier] ?? 30;
  const endorsementScore = Math.min(endorsementCount * POINTS_PER_ENDORSEMENT, MAX_ENDORSEMENT_POINTS);
  const baseScore = tierScore + endorsementScore;

  let activityFactorBps = 10000;
  if (halfLifeDays > 0) {
    const penalty = Math.min(Math.floor((daysInactive * 5000) / halfLifeDays), 10000);
    activityFactorBps = 10000 - penalty;
  }

  let reputationFactorBps = 10000;
  if (revocationWeight > 0 && totalIssued > 0) {
    const ratioBps = Math.min(Math.floor((revocations * 10000) / totalIssued), 10000);
    const penalty = Math.min(Math.floor((ratioBps * revocationWeight) / 100), 10000);
    reputationFactorBps = 10000 - penalty;
  }

  const afterActivity = Math.floor((baseScore * activityFactorBps) / 10000);
  const finalScore = Math.floor((afterActivity * reputationFactorBps) / 10000);

  return finalScore;
}

export function runSimulation() {
  console.log("===============================================================================");
  console.log("   TrustLink: Endorsements & Confidence Scoring Lifecycle Simulation           ");
  console.log("===============================================================================\n");

  const primaryIssuer = {
    address: "GA_PRIMARY_AUDITOR_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    tier: IssuerTier.Verified,
    totalIssued: 25,
    revocations: 0,
  };

  const peerEndorser1 = {
    address: "GB_PEER_SECURITY_FIRM_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
    name: "CertiK / OpenZeppelin Node",
  };

  const peerEndorser2 = {
    address: "GC_DEFI_RATING_AGENCY_CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
    name: "Gauntlet Risk DAO",
  };

  const subject = {
    address: "GS_VAULT_PROTOCOL_SSSSSSSSSSSSSSSSSSSSSSSSSSSSSSSSSSSSSS",
    name: "YieldVault Core Contract",
  };

  const attestation = {
    id: "att_soc2_vault_98712",
    claimType: "SMART_CONTRACT_AUDIT_PASSED",
    issuer: primaryIssuer.address,
    subject: subject.address,
    endorsements: [],
  };

  // Step 1: Baseline Attestation Created
  console.log("▶ STEP 1: Primary Issuer Creates Attestation");
  console.log(`  Attestation ID : ${attestation.id}`);
  console.log(`  Claim Type     : ${attestation.claimType}`);
  console.log(`  Issuer Tier    : ${primaryIssuer.tier} (Base Tier Score: ${TIER_BASE_SCORES[primaryIssuer.tier]})`);
  console.log(`  Subject        : ${subject.name} (${subject.address})`);

  let currentScore = computeConfidenceScore({
    issuerTier: primaryIssuer.tier,
    endorsementCount: attestation.endorsements.length,
  });
  console.log(`\n  [Verifier Query] get_confidence_score("${attestation.id}") -> ${currentScore}/100`);
  console.log("  (Verified tier base score = 60, Endorsements = 0)\n");

  // Step 2: First Endorsement
  console.log("▶ STEP 2: Peer Issuer 1 Endorses Attestation");
  console.log(`  Endorser: ${peerEndorser1.name} (${peerEndorser1.address})`);
  attestation.endorsements.push({
    endorser: peerEndorser1.address,
    timestamp: Date.now(),
  });

  currentScore = computeConfidenceScore({
    issuerTier: primaryIssuer.tier,
    endorsementCount: attestation.endorsements.length,
  });
  console.log(`  Endorsement recorded on-chain.`);
  console.log(`\n  [Verifier Query] get_confidence_score("${attestation.id}") -> ${currentScore}/100`);
  console.log(`  (Verified tier 60 + 1 endorsement * 2 pts = 62)\n`);

  // Step 3: Second Endorsement
  console.log("▶ STEP 3: Peer Issuer 2 Endorses Attestation");
  console.log(`  Endorser: ${peerEndorser2.name} (${peerEndorser2.address})`);
  attestation.endorsements.push({
    endorser: peerEndorser2.address,
    timestamp: Date.now(),
  });

  currentScore = computeConfidenceScore({
    issuerTier: primaryIssuer.tier,
    endorsementCount: attestation.endorsements.length,
  });
  console.log(`  Endorsement recorded on-chain.`);
  console.log(`\n  [Verifier Query] get_confidence_score("${attestation.id}") -> ${currentScore}/100`);
  console.log(`  (Verified tier 60 + 2 endorsements * 2 pts = 64)\n`);

  // Step 4: DecayConfig Breakdown & Impact
  console.log("▶ STEP 4: Demonstrating DecayConfig Dynamics");
  console.log("  TrustLink supports configurable decay parameters stored in on-chain storage:");
  console.log("    - half_life_days    : Inactivity decay (penalty halving rate)");
  console.log("    - revocation_weight : Reputation penalty for issuers with high revocation ratios\n");

  const decayConfig = {
    halfLifeDays: 180, // Score halves if inactive for 180 days
    revocationWeight: 50, // 50% penalty scale on revocation ratio
  };
  console.log(`  Applied DecayConfig: half_life_days=${decayConfig.halfLifeDays}, revocation_weight=${decayConfig.revocationWeight}`);

  // Scenario 4A: 90 Days of Issuer Inactivity
  const daysInactive = 90;
  const scoreAfterInactivity = computeConfidenceScore({
    issuerTier: primaryIssuer.tier,
    endorsementCount: attestation.endorsements.length,
    daysInactive: daysInactive,
    halfLifeDays: decayConfig.halfLifeDays,
  });
  console.log(`  - After ${daysInactive} days of inactivity:`);
  console.log(`    penalty_bps = (90 * 5000 / 180) = 2500 bps (25% penalty)`);
  console.log(`    [Verifier Query] get_confidence_score() -> ${scoreAfterInactivity}/100 (decreased from 64)`);

  // Scenario 4B: Inactivity + Revocation Ratio Penalty
  const revocations = 5;
  const totalIssued = 25; // 20% revocation ratio
  const scoreAfterRevocations = computeConfidenceScore({
    issuerTier: primaryIssuer.tier,
    endorsementCount: attestation.endorsements.length,
    daysInactive: daysInactive,
    halfLifeDays: decayConfig.halfLifeDays,
    totalIssued: totalIssued,
    revocations: revocations,
    revocationWeight: decayConfig.revocationWeight,
  });
  console.log(`  - After ${revocations} revocations out of ${totalIssued} issued (20% revocation ratio, weight 50%):`);
  console.log(`    penalty_bps = (2000 bps * 50 / 100) = 1000 bps (10% penalty)`);
  console.log(`    [Verifier Query] get_confidence_score() -> ${scoreAfterRevocations}/100\n`);

  console.log("===============================================================================");
  console.log("   Simulation Completed Successfully                                           ");
  console.log("===============================================================================");
}

// Execute when run directly via CLI
if (import.meta.url === `file://${process.argv[1]}`) {
  runSimulation();
}
