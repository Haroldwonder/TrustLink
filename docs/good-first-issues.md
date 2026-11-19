# Good First Issues

TrustLink spans a Rust smart contract, two SDKs (TypeScript and Python), an off-chain indexer, several example apps, and a large set of documentation. This guide gives new contributors a concrete starting point in each subsystem, without having to first understand the whole codebase.

Browse all currently open starter tasks with the [`good first issue`](https://github.com/Haroldwonder/TrustLink/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22) label.

## Contract (Rust / Soroban)

The core on-chain logic lives at the repo root (`src/`, built with `cargo build`/`cargo test`).

- Start with [docs/stellar-concepts.md](stellar-concepts.md) if you're new to Soroban.
- Good starter tasks: adding a new read-only query function, improving error messages, adding unit tests for an existing code path, or fixing a Clippy warning.
- Setup: see [CONTRIBUTING.md](../CONTRIBUTING.md#prerequisites) for Rust/wasm32 toolchain install.

## SDKs (TypeScript & Python)

- TypeScript bindings: `bindings/typescript/`
- Python bindings: `bindings/python/`
- Good starter tasks: improving type coverage, adding usage examples to SDK README files, adding a convenience wrapper method, or fixing a bug in request/response parsing.
- See [docs/bindings-generation.md](bindings-generation.md) for how bindings are generated from the contract WASM.

## Indexer

The off-chain indexer lives in `indexer/` (Prisma + TypeScript).

- Good starter tasks: adding a new indexed field, writing a migration, improving indexer logging, or adding a query helper.
- See [docs/monitoring.md](monitoring.md) for how the indexer relates to event streaming.

## Examples

`examples/` contains standalone sample integrations: `react-app`, `python-verification`, `kyc-token`, `issuer-cli`, `anchor-integration`, `governance`.

- Good starter tasks: fixing a broken example, adding comments/README clarifications, updating an example to use a newer SDK method, or adding a small new example for an uncovered use case.

## Documentation

`docs/` holds all project documentation.

- Good starter tasks: fixing typos or broken links, clarifying a confusing section, adding a missing explanation, or translating a doc (see [docs/i18n/](i18n/)).
- The [Documentation issue template](../.github/ISSUE_TEMPLATE/documentation.yml) is a good way to report gaps you find while reading.

## After you pick a task

1. Comment on the issue to claim it.
2. Follow the [Local Setup](../CONTRIBUTING.md#local-setup) and [PR Process](../CONTRIBUTING.md#pr-process) sections of `CONTRIBUTING.md`.
3. Open a PR referencing the issue (`Closes #123`).
