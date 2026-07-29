# Requirements Document

## Introduction

This feature adds a `wasm-opt` optimization step to the TrustLink smart contract build pipeline. The Stellar/Soroban platform charges deployment fees based on contract size, so reducing WASM binary size directly lowers deployment costs. The optimization uses `wasm-opt` from the Binaryen toolchain, applied after the standard Rust release build, to further shrink the contract beyond what the Rust compiler alone produces. The build output must report pre- and post-optimization file sizes so developers can verify the improvement.

## Glossary

- **WASM**: WebAssembly binary format produced by the Rust compiler targeting `wasm32-unknown-unknown`
- **wasm-opt**: A WASM optimizer from the Binaryen toolchain that applies size and performance passes to a WASM binary
- **Binaryen**: An open-source compiler and toolchain infrastructure library for WebAssembly
- **Optimized_WASM**: The WASM binary produced after running `wasm-opt` on the release build output
- **Unoptimized_WASM**: The WASM binary produced directly by `cargo build --release` before any `wasm-opt` pass
- **Makefile**: The project's GNU Make build file containing named targets such as `build`, `test`, and `optimize`
- **Build_Pipeline**: The sequence of steps executed by `make optimize` to produce a deployable contract artifact

## Requirements

### Requirement 1: wasm-opt Toolchain Installation

**User Story:** As a developer, I want a documented way to install `wasm-opt`, so that I can set up my environment and run the optimization step without manual research.

#### Acceptance Criteria

1. THE Makefile `install` target SHALL print instructions for installing the `binaryen` package that provides `wasm-opt`
2. WHEN a developer runs `make install`, THE Build_Pipeline SHALL display the platform-appropriate installation command for `binaryen` (e.g., `apt install binaryen`, `brew install binaryen`)
3. THE README SHALL document the `wasm-opt` installation requirement under the prerequisites section

---

### Requirement 2: Optimized Build Target

**User Story:** As a developer, I want `make optimize` to run `wasm-opt -Oz` on the release WASM, so that the deployed contract binary is as small as possible.

#### Acceptance Criteria

1. WHEN a developer runs `make optimize`, THE Build_Pipeline SHALL first execute `cargo build --target wasm32-unknown-unknown --release` to produce the Unoptimized_WASM
2. WHEN the release build succeeds, THE Build_Pipeline SHALL run `wasm-opt -Oz` on `target/wasm32-unknown-unknown/release/trustlink.wasm` and write the Optimized_WASM to the same path or a designated output path
3. WHEN `make optimize` completes successfully, THE Build_Pipeline SHALL exit with status code 0
4. IF `wasm-opt` is not found on `PATH`, THEN THE Build_Pipeline SHALL exit with a non-zero status code and print a descriptive error message directing the developer to install `binaryen`

---

### Requirement 3: File Size Reporting

**User Story:** As a developer, I want the build output to show pre- and post-optimization file sizes, so that I can confirm the optimization is effective and track size changes over time.

#### Acceptance Criteria

1. WHEN `make optimize` runs, THE Build_Pipeline SHALL print the Unoptimized_WASM file size in bytes before invoking `wasm-opt`
2. WHEN `wasm-opt` completes, THE Build_Pipeline SHALL print the Optimized_WASM file size in bytes
3. WHEN both sizes are available, THE Build_Pipeline SHALL print the size reduction as both an absolute byte count and a percentage
4. THE Optimized_WASM file size SHALL be strictly less than the Unoptimized_WASM file size for a non-trivial contract

---

### Requirement 4: Correctness Preservation

**User Story:** As a developer, I want the optimized WASM to remain functionally correct, so that optimization does not introduce regressions in contract behavior.

#### Acceptance Criteria

1. WHEN `wasm-opt` produces the Optimized_WASM, THE Optimized_WASM SHALL pass all existing `cargo test` unit tests without modification
2. THE Build_Pipeline SHALL NOT alter any Rust source files or `Cargo.toml` as part of the optimization step
3. WHEN `make optimize` is run multiple times on the same source, THE Build_Pipeline SHALL produce an Optimized_WASM of deterministic size (within a 1% tolerance across runs on the same machine)

---

### Requirement 5: README Documentation

**User Story:** As a developer, I want the README to document the optimization step, so that contributors understand how to build and verify the optimized contract.

#### Acceptance Criteria

1. THE README SHALL include a section describing the `make optimize` command and its purpose
2. THE README SHALL document the `wasm-opt` prerequisite and how to install it
3. THE README SHALL explain that the Optimized_WASM is the artifact intended for Stellar deployment
