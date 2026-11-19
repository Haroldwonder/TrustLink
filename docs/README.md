# TrustLink Documentation

Index of project docs. Architecture Decision Records live under [`adr/`](adr/) and have [their own index](adr/README.md).

## Getting started

| Doc | Description |
|-----|-------------|
| [quickstart.md](quickstart.md) | 5-minute path from zero to a verified testnet attestation |
| [stellar-concepts.md](stellar-concepts.md) | Stellar & Soroban basics for new contributors |
| [glossary.md](glossary.md) | Domain terms (attestation, issuer, subject, bridge, etc.) |
| [integration-guide.md](integration-guide.md) | Integrating TrustLink from Rust and TypeScript |
| [migration-guide.md](migration-guide.md) | Upgrading across TrustLink versions |
| [troubleshooting.md](troubleshooting.md) | Common failures and FAQ |
| [e2e-request-flow-guide.md](e2e-request-flow-guide.md) | End-to-end attestation request flow walkthrough |
| [video-tutorial-guide.md](video-tutorial-guide.md) | Companion commands and snippets for the video tutorial |
| [video-tutorial-script.md](video-tutorial-script.md) | Narration script for the TrustLink video tutorial |

## Architecture

| Doc | Description |
|-----|-------------|
| [architecture.md](architecture.md) | High-level system design and component overview |
| [storage-layout.md](storage-layout.md) | On-chain storage keys, TTL policy, and RPC read examples |
| [storage-backward-compatibility.md](storage-backward-compatibility.md) | Keeping storage layouts compatible across upgrades |
| [storage-migration-issue-266-attestation-origin.md](storage-migration-issue-266-attestation-origin.md) | Migration plan for attestation origin (issue #266) |
| [formal/README.md](formal/README.md) | Formal specifications overview |
| [formal/VERIFICATION.md](formal/VERIFICATION.md) | How to run and interpret formal verification |
| [formal/multisig_state_machine.md](formal/multisig_state_machine.md) | Multi-sig proposal lifecycle specification |
| [formal/multisig_state_machine.tla](formal/multisig_state_machine.tla) | TLA+ model of the multi-sig state machine |
| [formal/Makefile](formal/Makefile) | Make targets for formal specs |
| [formal/check-spec.sh](formal/check-spec.sh) | Helper script to check formal specs |

## Security & compliance

| Doc | Description |
|-----|-------------|
| [security.md](security.md) | Trust hierarchy, threat model, and security model |
| [security-review.md](security-review.md) | Pre-audit findings and remediation status |
| [bug-bounty.md](bug-bounty.md) | Bug bounty scope, severity tiers, and safe harbor |
| [compliance.md](compliance.md) | GDPR / privacy considerations (erasure, minimization) |
| [dependency-security.md](dependency-security.md) | Dependency review and supply-chain policy |
| [reentrancy-audit.md](reentrancy-audit.md) | Reentrancy review notes for TrustLink |
| [sanctions-screening.md](sanctions-screening.md) | Sanctions and PEP screening integration guidance |
| [slsa-provenance.md](slsa-provenance.md) | Verifying build provenance (SLSA) |

## Operations

| Doc | Description |
|-----|-------------|
| [mainnet-checklist.md](mainnet-checklist.md) | Pre-mainnet deployment checklist |
| [mainnet-runbook.md](mainnet-runbook.md) | Mainnet deployment runbook |
| [monitoring.md](monitoring.md) | Event streaming and alerting |
| [key-rotation-runbook.md](key-rotation-runbook.md) | Rotating admin and operational keys |
| [disaster-recovery.md](disaster-recovery.md) | Disaster recovery procedures |
| [disaster-recovery-drill-results.md](disaster-recovery-drill-results.md) | DR drill results and evidence |
| [canary-deployment.md](canary-deployment.md) | Canary deployment strategy for the indexer |
| [rpc-failover-list.md](rpc-failover-list.md) | Recommended RPC provider failover list |
| [release-workflow.md](release-workflow.md) | Release Please / conventional-commit release flow |
| [indexer-idempotency.md](indexer-idempotency.md) | Indexer idempotency and event resilience |
| [performance.md](performance.md) | Contract performance benchmarks and storage costs |
| [performance-load-testing.md](performance-load-testing.md) | Indexer GraphQL API load testing |
| [mutation-testing.md](mutation-testing.md) | Mutation testing guide |
| [snapshot-testing.md](snapshot-testing.md) | Snapshot testing for contract behaviour |

## SDKs & bindings

| Doc | Description |
|-----|-------------|
| [bindings-generation.md](bindings-generation.md) | Generating and updating TypeScript contract bindings |

## Architecture Decision Records

Significant design choices are recorded as ADRs. See the full index:

**[docs/adr/README.md](adr/README.md)**
