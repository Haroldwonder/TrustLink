# Design Document: WASM Optimization Step

## Overview

The optimization pipeline adds a `wasm-opt -Oz` pass from the Binaryen toolchain to the existing `make optimize` target. After `cargo build --release` produces the initial WASM binary, `wasm-opt` applies aggressive size-reduction passes (dead code elimination, instruction combining, etc.) to shrink the artifact further. The Makefile reports pre- and post-optimization sizes so developers can verify the improvement. The optimized binary is the artifact intended for Stellar deployment.

The existing `Cargo.toml` already configures `[profile.release]` with `opt-level = "z"`, `lto = true`, `codegen-units = 1`, and `strip = "symbols"`, which gives a good baseline. `wasm-opt -Oz` then applies WebAssembly-specific passes on top of that baseline.

## Architecture

```
make optimize
    │
    ├─ 1. cargo build --target wasm32-unknown-unknown --release
    │       └─ produces: target/wasm32-unknown-unknown/release/trustlink.wasm
    │
    ├─ 2. print pre-optimization file size
    │
    ├─ 3. wasm-opt -Oz <input.wasm> -o <output.wasm>
    │       └─ produces: target/wasm32-unknown-unknown/release/trustlink_opt.wasm
    │
    └─ 4. print post-optimization size + reduction summary
```

The optimized artifact is written to a separate file (`trustlink_opt.wasm`) so the unoptimized binary is preserved for comparison and debugging. The Makefile uses shell utilities (`wc -c`, `awk`) that are available on Linux and macOS without additional dependencies.

## Components and Interfaces

### Makefile `optimize` target

The sole implementation surface. It orchestrates the build steps using standard GNU Make shell recipes.

```makefile
optimize:
    @echo "Building optimized contract..."
    cargo build --target wasm32-unknown-unknown --release
    @$(MAKE) _check_wasm_opt
    @$(MAKE) _run_wasm_opt

_check_wasm_opt:
    @command -v wasm-opt >/dev/null 2>&1 || \
        { echo "ERROR: wasm-opt not found. Install binaryen: brew install binaryen / apt install binaryen"; exit 1; }

_run_wasm_opt:
    $(eval WASM := target/wasm32-unknown-unknown/release/trustlink.wasm)
    $(eval OPT  := target/wasm32-unknown-unknown/release/trustlink_opt.wasm)
    $(eval PRE  := $(shell wc -c < $(WASM) | tr -d ' '))
    @wasm-opt -Oz $(WASM) -o $(OPT)
    $(eval POST := $(shell wc -c < $(OPT) | tr -d ' '))
    @awk 'BEGIN { \
        pre=$(PRE); post=$(POST); \
        diff=pre-post; pct=(diff/pre)*100; \
        printf "Before: %d bytes\nAfter:  %d bytes\nSaved:  %d bytes (%.1f%%)\n", pre, post, diff, pct \
    }'
```

> Note: The actual Makefile implementation uses a single shell block per target to avoid `$(eval)` scoping issues. See the Tasks section for the exact recipe.

### Makefile `install` target (updated)

Adds `binaryen` installation instructions alongside the existing Soroban CLI instructions.

### README.md (updated)

Adds a "Building & Optimization" section covering prerequisites, the `make optimize` command, and a note that `trustlink_opt.wasm` is the deployment artifact.

## Data Models

No new data structures are introduced. The relevant data is:

| Name | Type | Description |
|---|---|---|
| `WASM_PATH` | file path | `target/wasm32-unknown-unknown/release/trustlink.wasm` |
| `OPT_PATH` | file path | `target/wasm32-unknown-unknown/release/trustlink_opt.wasm` |
| `pre_size` | integer (bytes) | File size of `WASM_PATH` before optimization |
| `post_size` | integer (bytes) | File size of `OPT_PATH` after optimization |
| `saved_bytes` | integer (bytes) | `pre_size - post_size` |
| `saved_pct` | float (%) | `(saved_bytes / pre_size) * 100` |

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Size reduction reporting is accurate

*For any* pair of (pre_size, post_size) values printed by the build pipeline, the reported absolute saving must equal `pre_size - post_size` and the reported percentage must equal `(pre_size - post_size) / pre_size * 100` (within floating-point rounding to one decimal place).

**Validates: Requirements 3.1, 3.2, 3.3**

### Property 2: Optimization strictly reduces binary size

*For any* successful `make optimize` run on a non-trivial contract, the Optimized_WASM file size must be strictly less than the Unoptimized_WASM file size.

**Validates: Requirements 3.4**

### Property 3: Source files are not modified by optimization

*For any* `make optimize` run, the SHA-256 hashes of all files under `src/` and `Cargo.toml` must be identical before and after the run.

**Validates: Requirements 4.2**

### Property 4: Optimization output is deterministic

*For any* two consecutive `make optimize` runs on the same source tree, the byte size of the resulting `trustlink_opt.wasm` must be equal (within 1% tolerance on the same machine).

**Validates: Requirements 4.3**

## Error Handling

| Condition | Detection | Response |
|---|---|---|
| `wasm-opt` not on PATH | `command -v wasm-opt` returns non-zero | Print install instructions, exit non-zero |
| `cargo build` fails | Non-zero exit from cargo | Make propagates failure; optimization step is skipped |
| WASM file missing after build | `wc -c` fails | Shell error propagates; non-zero exit |
| `wasm-opt` produces larger output | Post-size ≥ pre-size | Reported as negative saving; no hard failure (edge case for trivial contracts) |

## Testing Strategy

### Dual Testing Approach

Both unit/example tests and property-based tests are used. They are complementary: example tests verify concrete end-to-end behavior, while property tests verify universal invariants.

### Example Tests (Shell / Integration)

These are shell-level integration tests that invoke `make` targets and inspect outputs and exit codes.

| Test | What it checks | Requirements |
|---|---|---|
| `make optimize` exits 0 | Happy-path exit code | 2.3 |
| Output contains "Before:", "After:", "Saved:" | Size reporting format | 3.1, 3.2, 3.3 |
| `trustlink_opt.wasm` exists after run | Output artifact created | 2.2 |
| `make optimize` with no `wasm-opt` on PATH exits non-zero | Error handling | 2.4 |
| `make install` output contains "binaryen" | Install instructions | 1.1, 1.2 |
| README contains "make optimize" and "binaryen" | Documentation | 5.1, 5.2, 5.3 |
| `cargo test` passes after optimization | Correctness preservation | 4.1 |

### Property-Based Tests

Property-based tests use a Rust test harness with `proptest` (or equivalent) to validate the size-reporting arithmetic independently of the Makefile.

**Property Test Configuration**: Minimum 100 iterations per property test.

#### Property 1 test: Size reduction reporting accuracy
- **Feature: wasm-optimization, Property 1: Size reduction reporting is accurate**
- Generate random `(pre_size, post_size)` pairs where `post_size < pre_size`
- Call the size-reporting formatting function
- Assert reported `saved_bytes == pre_size - post_size`
- Assert reported `saved_pct` is within 0.05% of `(pre_size - post_size) / pre_size * 100`

#### Property 3 test: Source files unchanged
- **Feature: wasm-optimization, Property 3: Source files are not modified by optimization**
- Hash all files under `src/` and `Cargo.toml` before running the optimization shell command
- Run the optimization
- Hash again and assert all hashes are equal

#### Property 4 test: Deterministic output
- **Feature: wasm-optimization, Property 4: Optimization output is deterministic**
- Run `make optimize` twice
- Assert `|size_run1 - size_run2| / size_run1 <= 0.01`

Properties 2 (size strictly reduced) is validated by the integration test suite comparing file sizes before and after the run.
