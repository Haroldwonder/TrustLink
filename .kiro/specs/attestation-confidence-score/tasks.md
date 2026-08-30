# Implementation Plan: Attestation Confidence Score

## Overview

Implement dynamic confidence scoring (`get_confidence_score`) computed on-demand using issuer tier, endorsement count, inactivity decay, and revocation ratio decay.

## Tasks

- [x] 1. Define data models and types
  - [x] 1.1 Add `IssuerTier` enum (`Basic = 0`, `Verified = 1`, `Premium = 2`) in `src/types.rs`
  - [x] 1.2 Add `DecayConfig` struct with `half_life_days` and `revocation_weight` in `src/types.rs`
  - [x] 1.3 Add `IssuerTierUpdated` event in `src/events.rs`
  - _Requirements: 2.1, 4.3_

- [x] 2. Storage layer for tiers and decay config
  - [x] 2.1 Add `Storage::get_issuer_tier` and `Storage::set_issuer_tier` in `src/storage.rs`
  - [x] 2.2 Add `Storage::get_decay_config` and `Storage::set_decay_config` in `src/storage.rs`
  - [x] 2.3 Track `Storage::get_last_issuance_time` and `Storage::get_issuer_revocations` in `src/storage.rs`
  - _Requirements: 2.3, 2.4, 4.1, 4.2_

- [x] 3. Dynamic confidence scoring algorithm in `src/admin.rs`
  - [x] 3.1 Implement `get_confidence_score(env, attestation_id) -> Option<u32>`
  - [x] 3.2 Compute base score: tier (Basic=30, Verified=60, Premium=90) + endorsements (+2 each up to +10)
  - [x] 3.3 Apply inactivity decay factor via `half_life_days` and days since last issuance
  - [x] 3.4 Apply revocation ratio decay factor via `revocation_weight` and `revocations / total_issued`
  - [x] 3.5 Implement `set_decay_config`, `get_decay_config`, and `set_issuer_tier` admin handlers
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.2, 3.1, 3.2, 4.1, 4.2_

- [x] 4. Expose contract entry points in `src/lib.rs`
  - [x] 4.1 Expose `get_confidence_score(env, attestation_id) -> Option<u32>`
  - [x] 4.2 Expose `set_decay_config`, `get_decay_config`, `set_issuer_tier`, and `get_issuer_tier`
  - _Requirements: 1.1, 2.3, 2.4, 4.3, 4.4_

- [x] 5. Unit & Integration Tests
  - [x] 5.1 Test `get_confidence_score` returns `None` for non-existent attestation
  - [x] 5.2 Test tier impact on confidence score (Basic -> 30, Verified -> 60, Premium -> 90)
  - [x] 5.3 Test inactivity decay decreases score as timestamp advances
  - [x] 5.4 Test revocation ratio decay decreases score on revocations
  - _Requirements: 1.1, 1.2, 2.1, 4.1, 4.2_

