# Contributing to TrustLink

Thanks for your interest in contributing! This guide covers everything you need to go from zero to a merged PR.

## Development environment

The fastest way to get a fully configured Rust + Soroban environment is to use the included devcontainer. No manual tool installation needed.

**Open in VS Code (one click):**

[![Open in Dev Containers](https://img.shields.io/static/v1?label=Dev%20Containers&message=Open&color=blue&logo=visualstudiocode)](vscode://ms-vscode-remote.remote-containers/cloneInVolume?url=https://github.com/Olisachukwuma1/TrustLink)

**Open in GitHub Codespaces:**

[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/Olisachukwuma1/TrustLink)

After the container finishes building, verify everything works:

```bash
# Run the test suite
cargo test

# Confirm Soroban CLI is available
soroban --version

# Confirm the wasm32 target is installed
rustup target list --installed | grep wasm32
```

`cargo-watch` is pre-installed for live test feedback during development:

```bash
cargo watch -x "test"
```

---

## New to Stellar or Soroban?

Before diving in, read [docs/stellar-concepts.md](docs/stellar-concepts.md) for a beginner-friendly explanation of ledger timestamps, storage TTL, `require_auth`, and the WASM deployment model — concepts that come up throughout the codebase.

## Prerequisites

| Tool          | Version                            | Install                                    |
| ------------- | ---------------------------------- | ------------------------------------------ |
| Rust          | stable (see `rust-toolchain.toml`) | https://rustup.rs                          |
| wasm32 target | —                                  | `rustup target add wasm32-unknown-unknown` |
| Soroban CLI   | latest                             | `cargo install --locked soroban-cli`       |

Verify your setup:

```bash
rustc --version
cargo --version
soroban --version
rustup target list --installed | grep wasm32
```

## Local Setup

```bash
# 1. Fork and clone
git clone https://github.com/<your-username>/TrustLink.git
cd TrustLink

# 2. Install the wasm target (rust-toolchain.toml handles the Rust version)
rustup target add wasm32-unknown-unknown

# 3. Confirm the project compiles
cargo check
```

## Running Tests

```bash
# Run all unit and integration tests
cargo test

# Or via make
make test
```

All tests must pass before submitting a PR.

## Local Stellar Development Workflow

Use a local Stellar Quickstart node when iterating on deployment and invoke flows to avoid testnet rate limits.

### 1. Start local network

```bash
docker compose up -d
# or: docker-compose up -d
```

This starts the `stellar/quickstart` standalone network from [docker-compose.yml](docker-compose.yml).

### 2. Deploy and initialize locally

```bash
make local-deploy
```

What this does:

- Builds the contract WASM.
- Ensures local Soroban network + identity are configured.
- Funds the local identity via Friendbot.
- Deploys the contract.
- Invokes `initialize`.
- Writes the deployed contract ID to `.local.contract-id`.

### 3. Local RPC endpoint

Use this RPC URL for local calls and scripts:

```text
http://localhost:8000/soroban/rpc
```

Default local network values used by `scripts/setup_local.sh`:

- Network name: `local`
- Network passphrase: `Standalone Network ; February 2017`

### 4. Stop local network

```bash
docker compose down
```

## Building the Contract

```bash
# Debug build
make build

# Optimized release build (requires soroban-cli)
make optimize
```

## Code Style

This project enforces formatting and lint rules in CI.

```bash
# Format code (must be clean before committing)
make fmt        # or: cargo fmt

# Run linter — zero warnings allowed
make clippy     # or: cargo clippy --all-targets -- -D warnings
```

Run both before every commit.

## PR Process

1. **Branch** off `main` with a descriptive name:

   ```bash
   git checkout -b feat/your-feature
   # or
   git checkout -b fix/your-bugfix
   ```

2. **Commit** with clear messages following the format:

   ```
   <type>: short description

   Optional longer explanation.
   ```

   Common types: `feat`, `fix`, `docs`, `test`, `refactor`.

3. **Before pushing**, make sure:

   - [ ] `cargo test` passes
   - [ ] `cargo fmt -- --check` is clean
   - [ ] `cargo clippy --all-targets -- -D warnings` is clean

4. **Open a PR** against `main`. Include:

   - What the change does and why
   - Any relevant issue numbers (`Closes #123`)
   - Notes for reviewers if the change is non-obvious

5. **Review**: at least one approval is required before merging. Address all review comments; force-push to the same branch to update the PR.

## Reporting Issues

Open a GitHub issue with:

- A clear description of the problem or feature request
- Steps to reproduce (for bugs)
- Expected vs actual behaviour
