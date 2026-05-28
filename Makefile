.PHONY: build test optimize clean install help local-deploy check-size

# ─────────────────────────────────────────────────────────────────────────────
# TrustLink Makefile
# ─────────────────────────────────────────────────────────────────────────────
#
# Network targeting
# -----------------
# NETWORK      — target network name (default: testnet)
#                Recognised values: testnet | mainnet | local
#
# The three networks are pre-configured with their canonical RPC URLs and
# network passphrases. You can override any URL via environment variables:
#
#   TESTNET_RPC_URL   (default: https://soroban-testnet.stellar.org)
#   MAINNET_RPC_URL   (default: https://mainnet.stellar.validationcloud.io/v1/...)
#   LOCAL_RPC_URL     (default: http://localhost:8000/soroban/rpc)
#
# Signing identity
# ----------------
# ADMIN_SECRET  — Stellar secret key (S...) used to sign deploy/invoke txns.
#                 Required for deploy and invoke targets.
#                 Never hard-code this value; pass it via the environment:
#                   export ADMIN_SECRET=SXXX...
#                   make deploy
#
# Contract ID
# -----------
# CONTRACT_ID   — Required for invoke target. Set after a successful deploy:
#                   export CONTRACT_ID=C...
#                   make invoke ARGS="-- get_admin"
#
# ─────────────────────────────────────────────────────────────────────────────

NETWORK      ?= testnet
WASM          = target/wasm32-unknown-unknown/release/trustlink.wasm
WASM_OPT      = target/wasm32-unknown-unknown/release/trustlink.optimized.wasm

# ── RPC URLs (overridable via environment) ────────────────────────────────────
TESTNET_RPC_URL  ?= https://soroban-testnet.stellar.org
MAINNET_RPC_URL  ?= https://mainnet.stellar.validationcloud.io/v1/wI7lMGrm7ZU5UP9jKa7R3A
LOCAL_RPC_URL    ?= http://localhost:8000/soroban/rpc

# ── Network passphrases ───────────────────────────────────────────────────────
TESTNET_PASSPHRASE  = Test SDF Network ; September 2015
MAINNET_PASSPHRASE  = Public Global Stellar Network ; September 2015
LOCAL_PASSPHRASE    = Standalone Network ; February 2017

# ── Resolve active network settings ──────────────────────────────────────────
ifeq ($(NETWORK),mainnet)
  RPC_URL    = $(MAINNET_RPC_URL)
  PASSPHRASE = $(MAINNET_PASSPHRASE)
else ifeq ($(NETWORK),local)
  RPC_URL    = $(LOCAL_RPC_URL)
  PASSPHRASE = $(LOCAL_PASSPHRASE)
else
  # Default: testnet
  NETWORK    = testnet
  RPC_URL    = $(TESTNET_RPC_URL)
  PASSPHRASE = $(TESTNET_PASSPHRASE)
endif

.PHONY: build test optimize clean install fmt clippy \
        deploy invoke \
        testnet mainnet local \
        bindings check-bindings \
        check-size \
        help

# ─────────────────────────────────────────────────────────────────────────────
# Help
# ─────────────────────────────────────────────────────────────────────────────
help:
	@echo "TrustLink Smart Contract - Makefile Commands"
	@echo "============================================="
	@echo "make build          - Build the contract in debug mode"
	@echo "make test           - Run all unit tests"
	@echo "make optimize       - Build release WASM and run wasm-opt -Oz"
	@echo "make check-size     - Verify optimized WASM is under 100 KB"
	@echo "make clean          - Clean build artifacts"
	@echo "make install        - Install required dependencies"
	@echo "make local-deploy   - Deploy and initialize contract on local Stellar network"
	@echo "make bindings       - Generate TypeScript bindings from compiled WASM"
	@echo "make check-bindings - Fail if committed bindings are out of date"

# ─────────────────────────────────────────────────────────────────────────────
# Build & test
# ─────────────────────────────────────────────────────────────────────────────
install:
	@echo "Required dependencies:"
	@echo "  Rust:        https://rustup.rs/"
	@echo "  Stellar CLI: cargo install --locked stellar-cli --features opt"
	@echo "  WASM target: rustup target add wasm32-unknown-unknown"
	@echo "  wasm-opt:    cargo install --locked wasm-opt  (or: apt install binaryen)"

## Build the contract in debug mode
build:
	@echo "Building TrustLink ($(NETWORK))..."
	cargo build --target wasm32-unknown-unknown --release

## Run all unit tests
test:
	@echo "Running tests..."
	cargo test

## Build release WASM, then run wasm-opt -Oz for maximum size reduction.
## Typical reduction: ~30–50% vs the raw release binary.
## Output: $(WASM_OPT)
optimize: build
	@echo "Optimizing WASM with wasm-opt -Oz..."
	wasm-opt -Oz --enable-bulk-memory --strip-debug \
		$(WASM) -o $(WASM_OPT)
	@echo "--- Size report ---"
	@printf "  Before: %d bytes (%d KB)\n" \
		$$(stat -c%s $(WASM)) $$(( $$(stat -c%s $(WASM)) / 1024 ))
	@printf "  After:  %d bytes (%d KB)\n" \
		$$(stat -c%s $(WASM_OPT)) $$(( $$(stat -c%s $(WASM_OPT)) / 1024 ))
	@printf "  Saved:  %d bytes\n" \
		$$(( $$(stat -c%s $(WASM)) - $$(stat -c%s $(WASM_OPT)) ))
	@echo "Optimized artifact: $(WASM_OPT)"

## Verify the optimized WASM binary is under the 100 KB ledger-storage threshold.
check-size: optimize
	@SIZE=$$(stat -c%s $(WASM_OPT)); \
	MAX=$$((100 * 1024)); \
	echo "Optimized WASM size: $${SIZE} bytes ($$(( SIZE / 1024 )) KB) / limit 100 KB"; \
	if [ "$$SIZE" -gt "$$MAX" ]; then \
		echo "ERROR: $(WASM_OPT) is $${SIZE} bytes — exceeds 100 KB threshold."; \
		exit 1; \
	fi; \
	echo "OK: binary is within the 100 KB limit."

## Clean build artifacts and compiled outputs
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

## Format code according to Rust standards
fmt:
	@echo "Formatting code..."
	cargo fmt

## Run clippy linter and enforce strict warnings
clippy:
	@echo "Running clippy..."
	cargo clippy --all-targets -- -D warnings

local-deploy: build
	@echo "Deploying TrustLink contract to local Stellar network..."
	./scripts/setup_local.sh

## Generate TypeScript bindings from the compiled WASM
bindings: build
	@echo "Generating TypeScript bindings..."
	stellar contract bindings typescript \
		--wasm $(WASM) \
		--contract-id 0000000000000000000000000000000000000000000000000000000000000001 \
		--network testnet \
		--output-dir bindings/typescript
	@echo "Bindings written to bindings/typescript/"

## Fail if committed bindings are out of date with the current WASM
check-bindings: bindings
	@echo "Checking bindings are up to date..."
	git diff --exit-code bindings/typescript/ || \
		(echo "ERROR: TypeScript bindings are out of date. Run 'make bindings' and commit the result." && exit 1)
