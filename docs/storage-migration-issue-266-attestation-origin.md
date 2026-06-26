# Storage Migration Plan: Issue #266

## Breaking Change

`Attestation` storage layout changed from two booleans:

- `imported: bool`
- `bridged: bool`

to one enum:

- `origin: AttestationOrigin` where `Native | Imported | Bridged`

This is a storage-breaking change for previously persisted attestations and requires migration before reading legacy state with the new contract.

## Why This Change

The old model allowed invalid combinations (`imported=true` and `bridged=true`) and duplicated intent. The enum enforces mutual exclusivity in a single field.

## Migration Strategy

1. Deploy a migration helper contract or one-time admin migration entrypoint.
2. Iterate over all existing attestation IDs.
3. For each legacy attestation record:
   - Map `(imported=false, bridged=false)` to `origin=Native`
   - Map `(imported=true, bridged=false)` to `origin=Imported`
   - Map `(imported=false, bridged=true)` to `origin=Bridged`
   - Map `(imported=true, bridged=true)` to `origin=Bridged` and write an audit note for legacy inconsistency
4. Re-write the attestation under the new layout.
5. Run post-migration validation:
   - Total attestation count unchanged
   - Subject/issuer indexes unchanged
   - `get_attestation` returns expected `origin` for sampled IDs
6. Switch clients/indexers to read `origin` and stop reading legacy booleans.

## Rollout Notes

- Perform migration during a maintenance window.
- Snapshot ledger state before migration.
- Keep rollback artifacts for at least one release cycle.
- Coordinate SDK/indexer updates in lockstep with contract upgrade.
