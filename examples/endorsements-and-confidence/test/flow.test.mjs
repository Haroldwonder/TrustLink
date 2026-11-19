/**
 * Unit & Integration Tests for Endorsements & Confidence Scoring Flow
 *
 * Validates:
 * 1. Base tier scores (Basic = 30, Verified = 60, Premium = 90).
 * 2. Endorsement score accumulation (+2 per endorsement, capped at +10).
 * 3. Before-and-after confidence scoring when 2 issuers endorse an attestation.
 * 4. Decay configuration math:
 *    - Inactivity decay (linear approximation with half_life_days)
 *    - Inactivity decay disabled when half_life_days == 0
 *    - Revocation ratio decay (revocations / total_issued * revocation_weight)
 *    - Revocation decay disabled when revocation_weight == 0
 * 5. Invalidation rules: returns null for deleted or non-valid attestations.
 *
 * Run with: node --test test/flow.test.mjs
 */

import { strict as assert } from "node:assert";
import { describe, it } from "node:test";
import {
  computeConfidenceScore,
  IssuerTier,
  TIER_BASE_SCORES,
  MAX_ENDORSEMENT_POINTS,
  POINTS_PER_ENDORSEMENT,
} from "../flow.mjs";

describe("Endorsements & Confidence Scoring Suite", () => {
  describe("1. Issuer Tier Base Scores", () => {
    it("assigns 30 to Basic tier", () => {
      const score = computeConfidenceScore({ issuerTier: IssuerTier.Basic });
      assert.equal(score, 30);
    });

    it("assigns 60 to Verified tier", () => {
      const score = computeConfidenceScore({ issuerTier: IssuerTier.Verified });
      assert.equal(score, 60);
    });

    it("assigns 90 to Premium tier", () => {
      const score = computeConfidenceScore({ issuerTier: IssuerTier.Premium });
      assert.equal(score, 90);
    });

    it("defaults to Basic (30) when tier is unknown", () => {
      const score = computeConfidenceScore({ issuerTier: "UnknownTier" });
      assert.equal(score, 30);
    });
  });

  describe("2. Endorsements Progression (Before & After Two Peer Endorsements)", () => {
    it("increases confidence score by +2 per endorsement", () => {
      // Baseline before any endorsements
      const beforeScore = computeConfidenceScore({
        issuerTier: IssuerTier.Verified,
        endorsementCount: 0,
      });
      assert.equal(beforeScore, 60);

      // After 1st peer endorsement
      const afterFirstScore = computeConfidenceScore({
        issuerTier: IssuerTier.Verified,
        endorsementCount: 1,
      });
      assert.equal(afterFirstScore, 62);

      // After 2nd peer endorsement
      const afterSecondScore = computeConfidenceScore({
        issuerTier: IssuerTier.Verified,
        endorsementCount: 2,
      });
      assert.equal(afterSecondScore, 64);

      // Invariant: net increase of 4 points from two endorsements
      assert.equal(afterSecondScore - beforeScore, 4);
    });

    it("caps endorsement bonus at +10 points (5 endorsements)", () => {
      const score5 = computeConfidenceScore({
        issuerTier: IssuerTier.Verified,
        endorsementCount: 5,
      });
      assert.equal(score5, 70); // 60 + 10

      const score10 = computeConfidenceScore({
        issuerTier: IssuerTier.Verified,
        endorsementCount: 10,
      });
      assert.equal(score10, 70); // capped at +10
    });
  });

  describe("3. Inactivity Decay Dynamics", () => {
    it("halves activity score when days_inactive equals half_life_days", () => {
      const score = computeConfidenceScore({
        issuerTier: IssuerTier.Premium, // 90 base
        endorsementCount: 0,
        daysInactive: 100,
        halfLifeDays: 100, // penalty_bps = 100 * 5000 / 100 = 5000 bps (50% penalty)
      });
      // 90 * (10000 - 5000) / 10000 = 45
      assert.equal(score, 45);
    });

    it("ignores inactivity decay when half_life_days is 0 (disabled)", () => {
      const score = computeConfidenceScore({
        issuerTier: IssuerTier.Premium,
        endorsementCount: 0,
        daysInactive: 365,
        halfLifeDays: 0,
      });
      assert.equal(score, 90);
    });

    it("caps inactivity penalty at 10000 bps (zero score)", () => {
      const score = computeConfidenceScore({
        issuerTier: IssuerTier.Basic,
        endorsementCount: 0,
        daysInactive: 500,
        halfLifeDays: 100, // 500 * 5000 / 100 = 25000 bps -> capped at 10000 bps
      });
      assert.equal(score, 0);
    });
  });

  describe("4. Revocation-Ratio Decay Dynamics", () => {
    it("penalizes score proportionally to revocation ratio and weight", () => {
      // 20% revocation ratio (2 revocations out of 10)
      // weight = 100 -> penalty_bps = 2000 bps (20% penalty)
      const score = computeConfidenceScore({
        issuerTier: IssuerTier.Premium, // 90 base
        endorsementCount: 0,
        totalIssued: 10,
        revocations: 2,
        revocationWeight: 100,
      });
      // 90 * (10000 - 2000) / 10000 = 72
      assert.equal(score, 72);
    });

    it("disables revocation penalty when revocation_weight is 0", () => {
      const score = computeConfidenceScore({
        issuerTier: IssuerTier.Premium,
        totalIssued: 10,
        revocations: 5,
        revocationWeight: 0,
      });
      assert.equal(score, 90);
    });

    it("handles zero totalIssued safely without division by zero", () => {
      const score = computeConfidenceScore({
        issuerTier: IssuerTier.Verified,
        totalIssued: 0,
        revocations: 0,
        revocationWeight: 50,
      });
      assert.equal(score, 60);
    });
  });

  describe("5. Invalidation & Deletion Rules", () => {
    it("returns null when attestation is marked deleted", () => {
      const score = computeConfidenceScore({
        issuerTier: IssuerTier.Premium,
        endorsementCount: 5,
        isDeleted: true,
      });
      assert.equal(score, null);
    });

    it("returns null when attestation status is not Valid", () => {
      const score = computeConfidenceScore({
        issuerTier: IssuerTier.Premium,
        endorsementCount: 5,
        isValid: false,
      });
      assert.equal(score, null);
    });
  });
});
