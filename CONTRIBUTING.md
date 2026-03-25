# Contributing to TrustLink

Thanks for your interest in contributing! This guide covers everything you need to go from zero to a merged PR.

## Local Development Setup

TrustLink uses [pre-commit](https://pre-commit.com) to enforce formatting and linting before every commit.

**Install the hooks once after cloning:**

```bash
pip install pre-commit   # or: brew install pre-commit
pre-commit install
```

After that, every `git commit` automatically runs:

| Hook | What it checks |
|---|---|
| `cargo fmt --all -- --check` | Rust formatting (Rustfmt) |
| `cargo clippy --all-targets --all-features -- -D warnings` | Rust lints (Clippy) |
| `check-yaml` | Valid YAML syntax |
| `end-of-file-fixer` | Files end with a newline |
| `trailing-whitespace` | No trailing spaces |

If a hook fails the commit is blocked. Fix the reported issues and `git commit` again.

**Run hooks manually at any time:**

```bash
pre-commit run --all-files   # check everything
pre-commit run cargo-fmt     # check one hook by id
```

## New to Stellar or Soroban?

Before diving in, read [docs/stellar-concepts.md](docs/stellar-concepts.md) for a beginner-friendly explanation of ledger timestamps, storage TTL, `require_auth`, and the WASM deployment model — concepts that come up throughout the codebase.

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | stable (see `rust-toolchain.toml`) | https://rustup.rs |
| wasm32 target | — | `rustup target add wasm32-unknown-unknown` |
| Soroban CLI | latest | `cargo install --locked soroban-cli` |

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
