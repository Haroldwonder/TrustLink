# TrustLink Migration Guide

This guide helps integrators upgrade between TrustLink contract versions. Each section documents breaking changes to the ABI, storage layout, and event format introduced in that release, along with a checklist of steps required to update your integration.

For a full list of changes see the [CHANGELOG](../CHANGELOG.md). For storage internals see [docs/storage-layout.md](./storage-layout.md).

---

## How Soroban contract upgrades work

When the TrustLink admin calls `upgrade(new_wasm_hash)`, the contract's executable code is replaced atomically. **All on-chain storage is preserved** — no keys are deleted or rewritten. The new WASM begins reading the same raw XDR bytes the old WASM wrote.

Adding a new storage key is always safe. Changing the shape of an existing stored struct is a breaking change that requires a `migrate` function to be called once by the admin immediately after `upgrade`.

---

## Version history

| Version | Release date | Breaking changes |
| ------- | ------------ | ---------------- |
| 0.1.0   | 2026-03-25   | Initial release — no prior version to migrate from |

> **Note:** The contract's public interface has grown significantly since the initial 0.1.0 release. The full current ABI is documented in the [v0.1.0 ABI reference](#abi-reference---current) below. This table will be updated with a new row for each versioned release that introduces ABI or storage changes.

---

## v0.1.0 — Initial release

This is the first public release of TrustLink. There is no prior version to migrate from.

### ABI reference — current

The tables below document all public entry points currently exposed by the contract, grouped by functional area. All functions were introduced in v0.1.0.

#### Lifecycle & admin

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `initialize` | `(admin: Address, ttl_days: Option<u32>)` | Deploy and configure the contract. Can only be called once. |
| `get_admin` | `() → Address` | Return the current admin address. |
| `transfer_admin` | `(current_admin: Address, new_admin: Address)` | Immediately transfer admin rights to a new address. |
| `propose_admin_transfer` | `(current_admin: Address, new_admin: Address)` | Initiate a two-step admin handover requiring acceptance. |
| `accept_admin_transfer` | `(new_admin: Address)` | New admin accepts a pending handover. |
| `cancel_admin_transfer` | `(current_admin: Address)` | Cancel a pending two-step transfer. |
| `get_pending_admin_transfer` | `() → Option<PendingAdminTransfer>` | Return the pending transfer record if one exists. |
| `add_admin` | `(existing_admin: Address, new_admin: Address)` | Add a member to the admin council. |
| `remove_admin` | `(existing_admin: Address, admin_to_remove: Address)` | Remove a member from the admin council. |
| `get_admin_council` | `() → Vec<Address>` | Return all current admin council members. |
| `pause` | `(admin: Address)` | Halt all state-changing operations. |
| `unpause` | `(admin: Address)` | Resume normal operation. |
| `is_paused` | `() → bool` | Check whether the contract is currently paused. |
| `upgrade` | `(admin: Address, new_wasm_hash: BytesN<32>)` | Replace the contract executable. Storage is preserved. |
| `get_version` | `() → String` | Return the deployed contract version string. |
| `health_check` | `() → HealthStatus` | Return a lightweight status snapshot (initialized, admin set, issuer count, total attestations). Safe for unauthenticated polling. |
| `get_contract_metadata` | `() → ContractMetadata` | Return contract-level metadata. |
| `get_config` | `() → ContractConfig` | Return the full runtime configuration snapshot. |

#### Issuer management

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `register_issuer` | `(admin: Address, issuer: Address)` | Register a trusted attestation issuer. |
| `remove_issuer` | `(admin: Address, issuer: Address)` | Remove an issuer from the registry. |
| `is_issuer` | `(address: Address) → bool` | Check whether an address is a registered issuer. |
| `get_issuer_list` | `(start: u32, limit: u32) → Vec<Address>` | Paginated list of all registered issuers. |
| `get_issuer_metadata` | `(issuer: Address) → Option<IssuerMetadata>` | Fetch issuer display metadata. |
| `set_issuer_metadata` | `(issuer: Address, metadata: IssuerMetadata)` | Set or update issuer display metadata. |
| `get_issuer_stats` | `(issuer: Address) → IssuerStats` | Fetch per-issuer attestation statistics. |
| `get_issuer_tier` | `(issuer: Address) → Option<IssuerTier>` | Return the trust tier assigned to an issuer. |
| `set_issuer_tier` | `(admin: Address, issuer: Address, tier: IssuerTier)` | Assign a trust tier to an issuer. |

#### Whitelist

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `add_to_whitelist` | `(issuer: Address, subject: Address)` | Add a subject to an issuer's subject whitelist. |
| `remove_from_whitelist` | `(issuer: Address, subject: Address)` | Remove a subject from an issuer's whitelist. |
| `is_whitelisted` | `(issuer: Address, subject: Address) → bool` | Check whether a subject is whitelisted for an issuer. |
| `is_whitelist_enabled` | `(issuer: Address) → bool` | Check whether whitelist mode is active for an issuer. |
| `set_whitelist_enabled` | `(issuer: Address, enabled: bool)` | Enable or disable whitelist enforcement for an issuer. |
| `enable_whitelist_mode` | `(issuer: Address)` | Convenience alias — enable whitelist mode. |

#### Delegation

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `delegate_claim_type` | `(issuer: Address, delegate: Address, claim_type: String, expiration: Option<u64>)` | Grant a delegate the right to issue a specific claim type on behalf of the issuer. |
| `revoke_delegation` | `(issuer: Address, delegate: Address, claim_type: String)` | Revoke a specific delegation. |
| `revoke_delegation_all` | `(delegator: Address)` | Revoke all delegations granted by an issuer. |
| `get_delegation` | `(delegator: Address, delegate: Address, claim_type: String) → Option<Delegation>` | Fetch a specific delegation record. |
| `list_delegations_by_delegator` | `(delegator: Address, start: u32, limit: u32) → Vec<Delegation>` | Paginated list of delegations granted by an issuer. |

#### Claim types

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `register_claim_type` | `(admin: Address, claim_type: String, description: String)` | Register a new allowed claim type. |
| `get_claim_type_description` | `(claim_type: String) → Option<String>` | Fetch the description for a registered claim type. |
| `list_claim_types` | `(start: u32, limit: u32) → Vec<String>` | Paginated list of all registered claim types. |
| `set_claim_type_constraints` | `(admin: Address, claim_type: String, constraints: ClaimTypeConstraints)` | Set issuance constraints (max per subject, required fields, etc.) for a claim type. |
| `get_claim_type_constraints` | `(claim_type: String) → Option<ClaimTypeConstraints>` | Fetch the constraints for a claim type. |
| `set_registered_claim_type` | `(admin: Address, require: bool)` | Enforce that only registered claim types may be used. |
| `get_registered_claim_type` | `() → bool` | Check whether claim type registration is enforced. |

#### Fees & rate limits

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `set_fee` | `(admin: Address, fee: i128, collector: Address, fee_token: Option<Address>)` | Configure the attestation fee and collector address. |
| `get_fee_config` | `() → FeeConfig` | Return the current fee configuration. |
| `set_rate_limit` | `(admin: Address, min_issuance_interval: u64)` | Set a global minimum interval between issuances (per issuer). |
| `get_rate_limit` | `() → Option<RateLimitConfig>` | Return the global rate limit configuration. |
| `set_rate_limit_for_claim_type` | `(admin: Address, claim_type: String, min_interval: u64)` | Set a per-claim-type issuance rate limit. |
| `get_rate_limit_for_claim_type` | `(claim_type: String) → Option<u64>` | Return the rate limit for a specific claim type. |

#### Storage limits & metadata settings

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `set_max_attestations_per_subject` | `(admin: Address, limit: Option<u32>)` | Cap how many attestations a subject may accumulate. |
| `get_max_attestations_per_subject` | `() → Option<u32>` | Return the per-subject cap. |
| `set_limits` | `(admin: Address, max_attestations_per_issuer: u32, max_attestations_per_subject: u32)` | Set both issuer and subject attestation caps in one call. |
| `get_limits` | `() → StorageLimits` | Return both caps. |
| `set_chunk_size` | `(admin: Address, chunk_size: u32)` | Configure the pagination chunk size for batch reads. |
| `get_chunk_size` | `() → u32` | Return the current pagination chunk size. |
| `set_metadata_hash_only` | `(admin: Address, enabled: bool)` | Enforce that the metadata field stores only a hash (not raw data). |
| `get_metadata_hash_only` | `() → bool` | Check whether hash-only metadata mode is active. |

#### Confidence scoring & decay

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `get_confidence_score` | `(attestation_id: String) → Option<u32>` | Return the computed confidence score for an attestation. |
| `set_decay_config` | `(admin: Address, config: DecayConfig)` | Configure the confidence score decay model. |
| `get_decay_config` | `() → DecayConfig` | Return the current decay configuration. |
| `is_decay_config_set` | `() → bool` | Check whether a decay config has been set. |

#### Expiration hooks

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `register_expiration_hook` | `(subject: Address, callback_contract: Address, notify_days_before: u32)` | Register an on-chain callback to be triggered before an attestation expires. |
| `get_expiration_hook` | `(subject: Address) → Option<ExpirationHook>` | Fetch the expiration hook for a subject. |
| `remove_expiration_hook` | `(subject: Address)` | Deregister the expiration hook for a subject. |

#### Bridges

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `register_bridge` | `(admin: Address, bridge_contract: Address)` | Register a trusted bridge contract. |
| `is_bridge` | `(address: Address) → bool` | Check whether an address is a registered bridge. |
| `get_bridge_list` | `(start: u32, limit: u32) → Vec<Address>` | Paginated list of all registered bridges. |

#### Creating attestations

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `create_attestation` | `(issuer: Address, subject: Address, claim_type: String, expiration: Option<u64>, metadata: Option<String>) → String` | Issue a standard attestation. Returns a deterministic hash-based ID. |
| `create_attestation_valid_from` | `(issuer, subject, claim_type, valid_from: u64, expiration, metadata) → String` | Issue an attestation with a future activation timestamp. |
| `create_attestation_versioned` | `(issuer, subject, claim_type, expiration, metadata, version: String) → String` | Issue a versioned attestation (useful for schema evolution). |
| `create_attestation_jurisdiction` | `(issuer, subject, claim_type, expiration, metadata, jurisdiction: String) → String` | Issue an attestation tagged with a jurisdiction. |
| `create_attestation_as_delegate` | `(delegate: Address, issuer: Address, subject, claim_type, expiration, metadata) → String` | Issue on behalf of an issuer under an active delegation. |
| `create_attestation_from_template` | `(issuer: Address, template_id: String, subject: Address, expiration, metadata) → String` | Issue an attestation conforming to a pre-registered template. |
| `create_attestations_batch` | `(issuer: Address, requests: Vec<AttestationRequest>) → Vec<String>` | Issue multiple attestations in a single transaction. |
| `create_attestation_bundle` | `(issuer: Address, subjects: Vec<Address>, claim_type: String, expiration, metadata) → String` | Issue a linked bundle of attestations across multiple subjects. |
| `import_attestation` | `(admin: Address, issuer, subject, claim_type, timestamp: u64, expiration, metadata) → String` | Import a historical attestation; sets the `imported` flag. |
| `bridge_attestation` | `(bridge: Address, subject, claim_type, source_chain: String, source_tx: String, expiration, metadata) → String` | Relay a cross-chain attestation from a registered bridge. |
| `simulate_create_attestation` | `(issuer, subject, claim_type, expiration, metadata) → SimulationResult` | Dry-run an attestation creation to check validity and fee cost without writing state. |

#### Revoking & amending attestations

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `revoke_attestation` | `(issuer: Address, attestation_id: String, reason: Option<String>)` | Mark a single attestation as revoked. |
| `revoke_attestations_batch` | `(issuer: Address, attestation_ids: Vec<String>, reason: Option<String>) → u32` | Revoke multiple attestations in one transaction. Returns the count revoked. |
| `renew_attestation` | `(issuer: Address, attestation_id: String, new_expiration: Option<u64>)` | Extend the expiration of an existing attestation. |
| `update_expiration` | `(issuer: Address, attestation_id: String, new_expiration: Option<u64>)` | Alias for `renew_attestation`. |
| `amend_attestation` | `(issuer: Address, attestation_id: String, new_metadata: Option<String>, new_expiration: Option<u64>)` | Update metadata and/or expiration of an existing attestation. Creates a version snapshot. |
| `transfer_attestation` | `(admin: Address, attestation_id: String, new_issuer: Address)` | Transfer ownership of an attestation to a different issuer. |
| `request_deletion` | `(subject: Address, attestation_id: String)` | Subject requests deletion of an attestation about them. |

#### Verification queries

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `has_valid_claim` | `(subject: Address, claim_type: String) → bool` | `true` if the subject holds a non-expired, non-revoked attestation of this type. |
| `has_valid_claim_from_issuer` | `(subject: Address, claim_type: String, issuer: Address) → bool` | Same as above, constrained to a specific issuer. |
| `has_any_claim` | `(subject: Address, claim_types: Vec<String>) → bool` | `true` if the subject holds a valid claim for any of the listed types (OR). |
| `has_all_claims` | `(subject: Address, claim_types: Vec<String>) → bool` | `true` if the subject holds valid claims for all listed types (AND). |
| `has_valid_claim_batch` | `(subjects: Vec<Address>, claim_type: String) → Vec<bool>` | Bulk version of `has_valid_claim` across multiple subjects. |

#### Fetching & querying attestations

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `get_attestation` | `(attestation_id: String) → Attestation` | Fetch the full attestation record by ID. |
| `get_attestation_status` | `(attestation_id: String) → AttestationStatus` | Return `Valid`, `Expired`, or `Revoked`; emits `attestation_expired` on lazy detection. |
| `get_attestation_by_type` | `(subject: Address, claim_type: String) → Option<Attestation>` | Fetch the most recent attestation of a given type for a subject. |
| `get_attestation_history` | `(attestation_id: String) → Vec<AttestationVersionSnapshot>` | Return all version snapshots for an amended attestation. |
| `get_audit_log` | `(attestation_id: String) → Vec<AuditEntry>` | Return the full audit trail for an attestation. |
| `get_subject_attestations` | `(subject: Address, start: u32, limit: u32) → Vec<String>` | Paginated list of attestation IDs held by a subject. |
| `get_issuer_attestations` | `(issuer: Address, start: u32, limit: u32) → Vec<String>` | Paginated list of attestation IDs issued by an issuer. |
| `get_attestations_in_range` | `(subject, from_ts: u64, to_ts: u64, start, limit) → Vec<Attestation>` | Fetch a subject's attestations created within a timestamp range. |
| `get_attestations_in_range_after` | `(subject, after_ts: u64, start, limit) → Vec<Attestation>` | Fetch attestations created after a given timestamp. |
| `get_attestations_by_tag` | `(subject: Address, tag: String) → Vec<String>` | Fetch attestation IDs for a subject that carry a specific tag. |
| `get_attestations_by_jurisdiction` | `(subject: Address, jurisdiction: String, start, limit) → Vec<String>` | Fetch attestation IDs for a subject filtered by jurisdiction. |
| `get_valid_claims` | `(subject: Address) → Vec<String>` | Return all currently valid claim types held by a subject. |
| `get_valid_claim_count` | `(subject: Address) → u32` | Return the count of currently valid claims for a subject. |
| `get_subject_attestation_count` | `(subject: Address) → u32` | Return the total attestation count (all statuses) for a subject. |
| `get_issuer_attestation_count` | `(issuer: Address) → u32` | Return the total attestation count issued by an issuer. |
| `get_expiring_attestations` | `(subject: Address, within_days: u32, start, limit) → Vec<Attestation>` | Return attestations expiring within the given number of days. |
| `get_issuer_expiring_attestations` | `(issuer: Address, within_days: u32, start, limit) → Vec<Attestation>` | Same, scoped to an issuer's issued attestations. |
| `export_revocation_list` | `(issuer: Address, start, limit) → Vec<String>` | Export the list of revoked attestation IDs for an issuer. |
| `get_global_stats` | `() → GlobalStats` | Return network-wide aggregate statistics. |

#### Endorsements

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `endorse_attestation` | `(endorser: Address, attestation_id: String)` | Add an endorsement to an attestation from a third party. |
| `get_endorsement_count` | `(attestation_id: String) → u32` | Return the number of endorsements on an attestation. |
| `list_endorsements_by_endorser` | `(endorser: Address, start: u32, limit: u32) → Vec<Endorsement>` | Paginated list of endorsements made by an address. |

#### Disputes

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `dispute_attestation` | `(disputant: Address, attestation_id: String, reason: String)` | Open a dispute against an attestation. |
| `get_dispute` | `(attestation_id: String) → Option<DisputeRecord>` | Fetch the dispute record for an attestation. |
| `resolve_dispute` | `(resolver: Address, attestation_id: String)` | Admin resolves an open dispute. |

#### Attestation requests (subject-initiated)

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `request_attestation` | `(subject: Address, issuer: Address, claim_type: String) → String` | Subject requests an attestation from a specific issuer. Returns a request ID. |
| `fulfill_request` | `(issuer: Address, request_id: String, expiration: Option<u64>) → String` | Issuer fulfills a pending request. Returns the resulting attestation ID. |
| `reject_request` | `(issuer: Address, request_id: String, reason: Option<String>)` | Issuer rejects a pending request. |
| `cancel_request` | `(subject: Address, request_id: String)` | Subject cancels their own pending request. |
| `get_attestation_request` | `(request_id: String) → AttestationRequest` | Fetch a request record by ID. |
| `get_pending_requests` | `(issuer: Address, start: u32, limit: u32) → Vec<AttestationRequest>` | Paginated list of pending requests for an issuer. |
| `cleanup_expired_requests` | `(issuer: Address)` | Purge expired pending requests for an issuer. |

#### Multi-signature attestations

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `propose_attestation` | `(proposer: Address, subject, claim_type, threshold: u32, expiration, metadata) → String` | Open a multi-sig proposal. Returns a proposal ID. |
| `cosign_attestation` | `(issuer: Address, proposal_id: String)` | Co-sign an open multi-sig proposal. Finalizes when the threshold is reached. |
| `cancel_multisig_proposal` | `(issuer: Address, proposal_id: String)` | Cancel an open proposal before it reaches threshold. |
| `get_multisig_proposal` | `(proposal_id: String) → MultiSigProposal` | Fetch a multi-sig proposal record. |
| `list_open_proposals` | `(issuer: Address, start: u32, limit: u32) → Vec<MultiSigProposal>` | Paginated list of open proposals for an issuer. |
| `set_multisig_ttl` | `(admin: Address, days: u32)` | Configure how long a proposal remains open before expiring. |
| `get_multisig_ttl` | `() → u32` | Return the current multi-sig TTL in days. |

#### Council governance

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `create_council_proposal` | `(proposer: Address, action: CouncilAction, description: String) → u32` | Create an admin council proposal. Returns a proposal ID. |
| `approve_council_proposal` | `(approver: Address, proposal_id: u32)` | Cast an approval vote on a council proposal. |
| `execute_council_action` | `(executor: Address, proposal_id: u32)` | Execute an approved council proposal after its timelock expires. |
| `get_council_proposal` | `(proposal_id: u32) → Option<CouncilProposal>` | Fetch a council proposal record. |
| `set_council_timelock_delay` | `(admin: Address, delay_seconds: u64)` | Set the timelock delay before an approved proposal can be executed. |
| `get_council_timelock_delay` | `() → u64` | Return the current timelock delay in seconds. |

#### Templates

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `create_template` | `(issuer: Address, template_id: String, claim_type: String, required_fields: Vec<String>, expiration: Option<u64>) → String` | Register a reusable attestation template. |
| `get_template` | `(issuer: Address, template_id: String) → Option<AttestationTemplate>` | Fetch a template by issuer and ID. |
| `list_templates` | `(issuer: Address) → Vec<String>` | List all template IDs registered by an issuer. |

#### Bundles

| Function | Signature | Description |
| -------- | --------- | ----------- |
| `get_bundle` | `(bundle_id: String) → AttestationBundle` | Fetch the bundle record by ID. |
| `get_bundle_attestations` | `(bundle_id: String) → Vec<Attestation>` | Fetch all attestation records belonging to a bundle. |
| `is_bundle_valid` | `(bundle_id: String) → bool` | `true` if all attestations in the bundle are valid. |
| `get_issuer_bundles` | `(issuer: Address) → Vec<String>` | List all bundle IDs created by an issuer. |
| `get_subject_bundles` | `(subject: Address) → Vec<String>` | List all bundle IDs that include a given subject. |

---

### Storage keys introduced

All keys described in [docs/storage-layout.md](./storage-layout.md) were introduced in this release:

- `Admin`, `Version`, `FeeConfig` (instance storage)
- `Issuer(Address)`, `Bridge(Address)`, `Attestation(String)` (persistent)
- `SubjectAttestations(Address)`, `IssuerAttestations(Address)` (persistent)
- `IssuerMetadata(Address)`, `ClaimType(String)`, `ClaimTypeList` (persistent)

### Events introduced

| Event name | Emitted by |
| ---------- | ---------- |
| `attestation_created` | `create_attestation`, `create_attestations_batch` |
| `attestation_revoked` | `revoke_attestation`, `revoke_attestations_batch` |
| `attestation_imported` | `import_attestation` |
| `attestation_bridged` | `bridge_attestation` |
| `attestation_expired` | `get_attestation_status`, `has_valid_claim` (lazy detection) |
| `issuer_registered` | `register_issuer` |
| `issuer_removed` | `remove_issuer` |
| `bridge_registered` | `register_bridge` |
| `fee_updated` | `set_fee` |
| `claim_type_registered` | `register_claim_type` |
| `multisig_proposed` | `propose_attestation` |
| `multisig_cosigned` | `cosign_attestation` |
| `expiration_hook_registered` | `register_expiration_hook` |

### Integrator checklist — new deployments

- [ ] Pin your SDK version to `v0.1.0` (TypeScript) or `0.1.0` (Python)
- [ ] Call `initialize(admin, ttl_days)` exactly once; subsequent calls are rejected
- [ ] Register at least one issuer with `register_issuer` before creating attestations
- [ ] If using fees, configure with `set_fee` after initialization
- [ ] Subscribe to the `attestation_created` and `attestation_revoked` event streams if you maintain an off-chain index
- [ ] Use `has_valid_claim` for on-chain verification in consuming contracts (avoids fetching the full `Attestation` struct)

---

## Upgrading to a future version

When a new version is released, this section will document:

1. **ABI changes** — functions added, removed, or with changed signatures
2. **Storage changes** — new keys, removed keys, or struct field changes
3. **Event format changes** — new event fields or renamed events
4. **Migration steps** — whether a `migrate` function must be called by the admin

### General upgrade procedure

```bash
# 1. Build the new WASM
make build

# 2. Upload the new WASM and capture the hash
NEW_HASH=$(stellar contract upload \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm)

# 3. Upgrade the contract (pauses execution while WASM is swapped)
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  -- upgrade \
  --admin "$ADMIN_PUBLIC" \
  --new_wasm_hash "$NEW_HASH"

# 4. If a migrate function exists, call it immediately after upgrade
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  -- migrate \
  --admin "$ADMIN_PUBLIC"

# 5. Verify the deployment
./scripts/verify_deployment.sh
```

### Integrator checklist for every upgrade

- [ ] Read the migration guide section for the target version before upgrading
- [ ] Test the upgrade on Testnet against a contract with representative data before Mainnet
- [ ] Update your SDK dependency to the version matching the new contract
- [ ] Check for ABI changes — look for renamed functions, new required parameters, or removed functions
- [ ] Check for storage changes — if struct fields changed, confirm the admin has run `migrate`
- [ ] Check for event format changes — update your indexer or event listeners accordingly
- [ ] Re-run your integration tests after upgrading
- [ ] Monitor `attestation_created` and `get_attestation` responses for unexpected field values in the first 24 hours after upgrade

---

## SDK compatibility matrix

| Contract version | TypeScript SDK | Python SDK |
| ---------------- | -------------- | ---------- |
| 0.1.0            | 0.1.x          | 0.1.x      |

> **Note:** Always use the SDK version that matches the deployed contract version. Using a newer SDK against an older contract (or vice versa) may result in ABI mismatches that produce runtime errors.

---

## Getting help

If you encounter issues during an upgrade:

- Open an issue using the [Bug Report](../.github/ISSUE_TEMPLATE/bug_report.md) template
- Check [docs/integration-guide.md](./integration-guide.md) for current API documentation
- For security-sensitive migration issues, use the [private disclosure process](../SECURITY.md)
