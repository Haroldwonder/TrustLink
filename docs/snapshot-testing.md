# Snapshot Testing

## What are snapshot tests?

TrustLink uses the Soroban SDK's built-in ledger snapshot facility to capture the exact on-chain state produced by critical contract operations. After each test that exercises the contract, the SDK serialises the full ledger state to a JSON file under `test_snapshots/`. These files are committed to the repository and act as a regression guard: if a code change silently alters storage layout, event emission, or any other observable state, the snapshot diff makes it immediately visible in CI and in code review.

Snapshot tests live in `tests/snapshot_test.rs`. Each test is named `snapshot_after_<operation>` and covers one key state transition (initialisation, issuer registration, attestation creation, revocation, bridging, transfer, pause/unpause, expiration hook, multisig activation).

## Normal test execution

Running the full test suite regenerates all snapshot files as a side-effect:

```bash
make test
# or equivalently:
cargo test
```

Snapshots are **always** regenerated on every test run — there is no flag to disable this. If the regenerated files are identical to the committed ones, `git status` shows no changes and everything is fine.

## When to update snapshots

Update snapshots when you have made an **intentional** change to contract behaviour or storage layout and the new output is correct. Examples:

- Adding a new field to `Attestation` or another stored struct
- Changing an event's topics or data payload
- Modifying storage keys or TTL configuration
- Any refactor that alters what gets written to the ledger

Do **not** update snapshots to silence a failing test caused by an unintentional regression. Investigate the root cause first.

## How to regenerate snapshots

```bash
make snapshot-update
```

This runs `cargo test`, which rewrites every file in `test_snapshots/`, then prints a reminder to review the diff. It is a convenience alias for `cargo test` that makes the intent explicit in the commit history.

## Reviewing regenerated snapshots

After running `make snapshot-update`, inspect every changed file before staging it:

```bash
git diff test_snapshots/
```

For each changed file, verify:

1. Only the fields you expected to change have changed.
2. No unrelated tests have drifted (address randomisation is seeded deterministically by the Soroban test environment, so snapshots are stable across runs).
3. The new JSON is structurally valid and human-readable.

Once satisfied, stage and commit:

```bash
git add test_snapshots/
git commit -m "test(snapshots): update snapshots after <brief description>"
```

## Handling failed snapshot tests in CI

CI runs `cargo test` twice and then checks `git diff --exit-code test_snapshots/`. A failure means the second run produced different snapshot files than the committed ones. This happens when:

- A code change altered contract behaviour but the developer forgot to commit updated snapshots.
- A non-deterministic value leaked into a snapshot (should not happen with the Soroban test environment, but check for `Address::generate` calls outside of a fixed-seed environment).

To fix a CI snapshot failure locally:

```bash
make snapshot-update
git diff test_snapshots/   # review
git add test_snapshots/
git commit -m "test(snapshots): update snapshots after <description>"
git push
```

## Avoiding accidental snapshot updates

Because snapshots regenerate on every `cargo test` run, it is easy to accidentally stage stale snapshot changes alongside unrelated work. To avoid this:

- Run `git diff test_snapshots/` before every commit and confirm any changes are intentional.
- Keep snapshot updates in their own commit, separate from the code change that caused them.
- Never run `git add .` without reviewing the diff first.

## Prerequisites

No additional tools are required beyond the standard development setup:

- Rust (stable, 1.70+)
- `soroban-sdk` with `testutils` feature (already in `[dev-dependencies]`)
- `wasm32-unknown-unknown` target (required for the build, not for running tests natively)

Snapshot files are plain JSON and require no special viewer. `git diff` and any standard diff tool work well.

## Environment variables

The Soroban ledger snapshot mechanism does not use an `UPDATE_EXPECT` or similar environment variable. Snapshots are unconditionally written during every test run. There is nothing to set.

## File layout

```
test_snapshots/
├── snapshot_after_initialization.1.json
├── snapshot_after_issuer_registration.1.json
├── snapshot_after_attestation_creation.1.json
├── snapshot_after_revocation.1.json
├── snapshot_after_bridge_attestation.1.json
├── snapshot_after_transfer_attestation.1.json
├── snapshot_after_contract_pause.1.json
├── snapshot_after_contract_unpause.1.json
├── snapshot_after_expiration_hook_triggered.1.json
├── snapshot_after_multisig_activation.1.json
└── ...  (other tests that exercise the contract also produce snapshots)
```

The `.1.json` suffix is the ledger checkpoint index appended by the SDK. Most tests produce a single checkpoint; tests with multiple ledger mutations may produce `.1.json`, `.2.json`, etc.
