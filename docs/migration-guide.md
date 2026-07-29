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

---

## v0.1.0 — Initial release

This is the first public release of TrustLink. There is no prior version to migrate from.

### ABI summary

All functions introduced in v0.1.0 are listed in the [CHANGELOG](../CHANGELOG.md#010---2026-03-25). Key entry points for integrators:

| Function | Description |
| -------- | ----------- |
| `initialize(admin, ttl_days)` | Deploy and configure the contract |
| `create_attestation(issuer, subject, claim_type, expiration, metadata)` | Issue a new attestation |
| `revoke_attestation(issuer, attestation_id)` | Revoke an existing attestation |
| `has_valid_claim(subject, claim_type)` | Check whether a subject holds a valid claim |
| `get_attestation(attestation_id)` | Fetch a full attestation record |
| `bridge_attestation(bridge, ...)` | Create an attestation from a trusted bridge contract |

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
