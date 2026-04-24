# Implementation Plan: WASM Optimization

## Overview

Implement the `wasm-opt` optimization step by updating the Makefile `optimize` and `install` targets, adding a size-reporting helper, and updating the README. Property-based tests validate the size-reporting arithmetic; integration tests validate the end-to-end pipeline.

## Tasks

- [ ] 1. Update Makefile `optimize` target with wasm-opt and size reporting
  - Replace the existing `optimize` target body in `Makefile` with a recipe that:
    1. Runs `cargo build --target wasm32-unknown-unknown --release`
    2. Checks for `wasm-opt` on PATH; prints install instructions and exits non-zero if missing
    3. Captures pre-optimization size of `target/wasm32-unknown-unknown/release/trustlink.wasm` using `wc -c`
    4. Runs `wasm-opt -Oz trustlink.wasm -o trustlink_opt.wasm`
    5. Captures post-optimization size and prints "Before: N bytes / After: N bytes / Saved: N bytes (X.X%)" using `awk`
  - Update the `install` target to also print `binaryen` installation instructions (`brew install binaryen` / `apt install binaryen`)
  - Update the `help` target echo to mention that `make optimize` produces `trustlink_opt.wasm`
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3_

  - [ ]* 1.1 Write integration test: make optimize exits 0 and produces trustlink_opt.wasm
    - Shell test (e.g., in `tests/` or a `test_optimize.sh`): run `make optimize`, assert exit code 0, assert `trustlink_opt.wasm` exists
    - _Requirements: 2.2, 2.3_

  - [ ]* 1.2 Write integration test: missing wasm-opt exits non-zero with error message
    - Run `make optimize` with `PATH` stripped of `wasm-opt`, assert non-zero exit and stderr/stdout contains "binaryen"
    - _Requirements: 2.4_

  - [ ]* 1.3 Write integration test: output contains size report lines
    - Capture stdout of `make optimize`, assert it contains "Before:", "After:", "Saved:"
    - _Requirements: 3.1, 3.2, 3.3_

- [ ] 2. Implement size-reporting helper function in Rust (for property testing)
  - Add a small pure Rust function (e.g., in `src/build_utils.rs` or a test-only module) that takes `(pre_size: u64, post_size: u64)` and returns a formatted report string: `"Before: {pre} bytes\nAfter:  {post} bytes\nSaved:  {diff} bytes ({pct:.1}%)"`
  - This function is the unit under test for Property 1; it also serves as the reference implementation for the Makefile `awk` snippet
  - _Requirements: 3.1, 3.2, 3.3_

  - [ ]* 2.1 Write property test for size reduction reporting accuracy
    - **Property 1: Size reduction reporting is accurate**
    - **Validates: Requirements 3.1, 3.2, 3.3**
    - Use `proptest` (add to `[dev-dependencies]` in `Cargo.toml`): generate random `(pre, post)` pairs where `post < pre`, call the formatting function, assert `saved_bytes == pre - post` and `saved_pct` is within 0.05% of `(pre - post) as f64 / pre as f64 * 100.0`
    - Minimum 100 iterations
    - Tag: `Feature: wasm-optimization, Property 1: Size reduction reporting is accurate`

- [ ] 3. Checkpoint — ensure all tests pass
  - Run `cargo test` and `make optimize`; confirm exit codes are 0 and `trustlink_opt.wasm` is produced and smaller than `trustlink.wasm`
  - Ask the user if any questions arise before continuing

- [ ] 4. Write property test: source files unchanged after optimization
  - **Property 3: Source files are not modified by optimization**
  - **Validates: Requirements 4.2**
  - Add a Rust integration test (in `tests/`) that:
    1. Computes SHA-256 (using `sha2` crate or `std::process::Command` calling `sha256sum`) of all files under `src/` and `Cargo.toml`
    2. Invokes `make optimize` via `std::process::Command`
    3. Recomputes hashes and asserts they are identical
  - _Requirements: 4.2_

  - [ ]* 4.1 Write property test for deterministic optimization output
    - **Property 4: Optimization output is deterministic**
    - **Validates: Requirements 4.3**
    - Run `make optimize` twice in the same test, record `trustlink_opt.wasm` sizes, assert `|size1 - size2| as f64 / size1 as f64 <= 0.01`
    - Tag: `Feature: wasm-optimization, Property 4: Optimization output is deterministic`

- [ ] 5. Update README.md with optimization documentation
  - Add a "Building & Optimization" section (or extend the existing build section) that covers:
    - Prerequisites: Rust, Soroban CLI, and `binaryen` (`wasm-opt`) with install commands for macOS and Linux
    - `make optimize` command description and what it produces
    - Note that `trustlink_opt.wasm` is the artifact intended for Stellar deployment
    - Example output showing the size report lines
  - _Requirements: 1.3, 5.1, 5.2, 5.3_

  - [ ]* 5.1 Write example test: README contains required content
    - Read `README.md` in a Rust test and assert it contains "make optimize", "binaryen", and "trustlink_opt.wasm"
    - _Requirements: 5.1, 5.2, 5.3_

- [ ] 6. Final checkpoint — ensure all tests pass
  - Run `cargo test --all` and `make optimize`; confirm `trustlink_opt.wasm` is smaller than `trustlink.wasm` and all tests are green
  - Ask the user if any questions arise

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- `proptest` should be added to `[dev-dependencies]` only; it has no effect on the release binary
- The `trustlink_opt.wasm` output path should be added to `.gitignore` alongside other build artifacts
- Property tests require minimum 100 iterations (proptest default is 256, which satisfies this)
