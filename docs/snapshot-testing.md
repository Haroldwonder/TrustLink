# Snapshot Testing

## How it works

The soroban-sdk automatically writes a JSON snapshot to `test_snapshots/` at
the end of every test that uses an `Env`. Each snapshot captures:

- Full ledger storage state (all contract data entries)
- All events published during the test
- Auth records

Snapshots are committed to the repository. CI re-runs the tests and fails if
any snapshot file differs from what is committed, catching unintended state or
event changes.

## Key snapshot tests

The four canonical state transitions are covered in `tests/snapshot_test.rs`:

| Test | Snapshot file | What it protects |
|------|--------------|-----------------|
| `snapshot_after_initialization` | `test_snapshots/tests/snapshot_after_initialization.1.json` | Admin, FeeConfig, TtlConfig, Version |
| `snapshot_after_issuer_registration` | `test_snapshots/tests/snapshot_after_issuer_registration.1.json` | Issuer key, GlobalStats.total_issuers, iss_reg event |
| `snapshot_after_attestation_creation` | `test_snapshots/tests/snapshot_after_attestation_creation.1.json` | Attestation record, indexes, AuditLog, created event |
| `snapshot_after_revocation` | `test_snapshots/tests/snapshot_after_revocation.1.json` | revoked flag, revocation_reason, AuditLog, total_revocations |

All other tests in `src/test.rs` and `tests/` also produce snapshots under
`test_snapshots/test/` and `test_snapshots/tests/` respectively.

## CI behaviour

The CI workflow (`.github/workflows/ci.yml`) runs `cargo test` twice:

1. First run executes the tests normally.
2. Second run regenerates snapshots and checks `git diff test_snapshots/`.

If any snapshot file differs from what is committed, CI fails with:

```
Snapshot files changed. Run 'cargo test' locally and commit the updated snapshots.
```

## Updating snapshots

Snapshots should be updated intentionally, not automatically. Follow this
process:

1. Make your contract or test change.
2. Run `cargo test` locally — this regenerates all snapshot files.
3. Review the diffs carefully:
   ```bash
   git diff test_snapshots/
   ```
4. Confirm every change is expected (new storage keys, changed event data, etc.).
5. Commit the updated snapshots together with the code change in the same PR.
6. In the PR description, explain why each snapshot changed.

## Reviewing a snapshot diff

Key things to look for in a diff:

- New or removed keys in `ledger_entries` — storage layout change
- Changed `val` for an existing key — logic change
- New or removed entries in `events` — event emission change
- Changes to `auth` — authorization flow change

A change in a snapshot for a test you did not touch is a red flag and should
be investigated before merging.

## Disabling snapshots (not recommended)

Snapshots can be disabled per-`Env` with:

```rust
env.set_snapshot_disabled(true);
```

Only do this for tests that intentionally produce non-deterministic state
(e.g. randomised fuzz inputs). The four canonical snapshot tests must never
have snapshots disabled.
