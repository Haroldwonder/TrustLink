# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

<!-- Add new changes here before they are released. Use the categories below:
### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security
-->

## [0.1.0] - 2026-03-25

### Added

- `initialize(admin, ttl_days)` — deploy and set the contract administrator with configurable storage TTL.
- `register_issuer(admin, issuer)` — admin registers a trusted attestation issuer.
- `remove_issuer(admin, issuer)` — admin removes an issuer from the registry.
- `is_issuer(address)` — query whether an address is an authorized issuer.
- `get_admin()` — return the current admin address.
- `transfer_admin(current_admin, new_admin)` — transfer contract administration rights.
- `create_attestation(issuer, subject, claim_type, expiration, metadata)` — issuer creates a new attestation with optional expiration and metadata; returns a deterministic hash-based ID.
- `revoke_attestation(issuer, attestation_id)` — issuer marks an attestation as revoked.
- `get_attestation(attestation_id)` — fetch full attestation data by ID.
- `get_attestation_status(attestation_id)` — return `Valid`, `Expired`, or `Revoked`; emits an `expired` event when status is `Expired`.
- `has_valid_claim(subject, claim_type)` — returns `true` if the subject holds a non-expired, non-revoked attestation of the given type; emits an `expired` event for any expired attestation encountered.
- `has_valid_claim_from_issuer(subject, claim_type, issuer)` — constrain verification to a specific issuer.
- `has_any_claim(subject, claim_types)` and `has_all_claims(subject, claim_types)` — OR/AND claim verification across multiple claim types.
- `get_subject_attestations(subject, start, limit)` — paginated list of attestation IDs for a subject.
- `get_issuer_attestations(issuer, start, limit)` — paginated list of attestation IDs issued by an issuer.
- `get_subject_attestation_count(subject)`, `get_issuer_attestation_count(issuer)`, and `get_valid_claim_count(subject)` — aggregate query helpers.
- Claim type registry: `register_claim_type`, `update_claim_type`, `remove_claim_type`, `get_claim_type_description`, and `list_claim_types`.
- Historical import support: `import_attestation(admin, issuer, subject, claim_type, timestamp, expiration)` and `Attestation.imported`.
- Fee configuration: `set_fee(admin, fee, collector, fee_token)` and `get_fee_config()` with optional token-denominated attestation fees.
- Bridge support: `register_bridge`, `remove_bridge`, `is_bridge`, and `bridge_attestation` with source-chain metadata.
- Batch operations: `create_attestations_batch` and `revoke_attestations_batch`.
- Expiration hooks: `register_expiration_hook`, `get_expiration_hook`, and `remove_expiration_hook` for callback notifications.
- Multi-signature attestations: `propose_attestation`, `cosign_attestation`, and `get_multisig_proposal`.
- Global and per-issuer statistics: `get_global_stats`, `get_issuer_stats`, and issuer tier/metadata management.
- Comprehensive event set for creation, revocation, bridge/import, fee updates, claim-type administration, multi-sig lifecycle, and expiration hooks.
- Integration examples under `examples/` including KYC token and governance-gated voting patterns.

### Fixed

- Validation coverage for metadata, tag cardinality/length, and timestamp/expiration edge cases.
- Deterministic storage/index consistency for issuer and subject attestation lookups.
- Authorization checks across admin, issuer, bridge, and multisig signer flows.

[Unreleased]: https://github.com/Haroldwonder/TrustLink/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Haroldwonder/TrustLink/releases/tag/v0.1.0
