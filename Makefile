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
WASM_LOOKUP_DIR ?= target/wasm32-unknown-unknown/release
SHA256SUM ?= $(shell command -v sha256sum 2>/dev/null || command -v shasum 2>/dev/null)
SHA256SUM_ARGS ?= $(if $(filter shasum,$(notdir $(SHA256SUM))),-a 256,)

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

.PHONY: build test snapshot-update optimize clean install fmt clippy deny \
        deploy invoke \
        testnet mainnet local \
        bindings check-bindings \
        check-size rollback \
        help

# ─────────────────────────────────────────────────────────────────────────────
# Help
# ─────────────────────────────────────────────────────────────────────────────
help:
	@echo "TrustLink Smart Contract - Makefile Commands"
	@echo "============================================="
	@echo "make build          - Build the contract in debug mode"
	@echo "make test           - Run all unit tests"
	@echo "make snapshot-update - Regenerate test_snapshots/ after intentional behaviour changes"
	@echo "make optimize       - Build release WASM and run wasm-opt -Oz"
	@echo "make check-size     - Verify optimized WASM is under 100 KB"
	@echo "make clean          - Clean build artifacts"
	@echo "make fmt            - Format source code"
	@echo "make clippy         - Run clippy linter"
	@echo "make deny           - Run cargo-deny (licenses, advisories, bans, sources)"
	@echo "make install        - Install required dependencies"
	@echo "make local-deploy   - Deploy and initialize contract on local Stellar network"
	@echo "make rollback       - Redeploy a verified WASM hash to a specified network"
	@echo "make bindings       - Generate TypeScript bindings from compiled WASM"
	@echo "make check-bindings - Fail if committed bindings are out of date"
	@echo "make deploy         - Build, optimize, and deploy to NETWORK (default: testnet)"
	@echo "                      Requires: ADMIN_SECRET=<secret>  SOURCE=<key-alias>"
	@echo "                      Example:  make deploy NETWORK=testnet SOURCE=deployer"
	@echo "make verify         - Run post-deployment verification against a live contract"
	@echo "                      Requires: CONTRACT_ID=<id>  SOURCE=<key-alias>"
	@echo "                      Optional: NETWORK=testnet|mainnet (default: testnet)"
	@echo "                      Example:  make verify CONTRACT_ID=C... SOURCE=deployer NETWORK=testnet"

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

## Regenerate all snapshot files in test_snapshots/.
## Use this after intentional contract behaviour changes to accept new snapshots.
## Review the diff with: git diff test_snapshots/
## See docs/snapshot-testing.md for the full workflow.
snapshot-update:
	@echo "Regenerating snapshots..."
	cargo test
	@echo "Snapshots updated. Review changes with: git diff test_snapshots/"

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

## Roll back to a previously built WASM hash and redeploy it to a selected network.
rollback:
	@if [ -z "$(WASM_HASH)" ]; then \
		echo "ERROR: WASM_HASH is required. Example: make rollback NETWORK=mainnet WASM_HASH=<hash>"; \
		exit 1; \
	fi; \
	@if [ -z "$(SHA256SUM)" ]; then \
		echo "ERROR: sha256sum or shasum is required to verify WASM hashes."; \
		exit 1; \
	fi; \
	@echo "Searching for a matching WASM artifact in $(WASM_LOOKUP_DIR)..."; \
	WASM_FILE=""; \
	for f in $$(find $(WASM_LOOKUP_DIR) -type f -name '*.wasm' 2>/dev/null); do \
		HASH=$$($(SHA256SUM) $(SHA256SUM_ARGS) "$$f" | awk '{print $$1}'); \
		if [ "$$HASH" = "$(WASM_HASH)" ]; then \
			WASM_FILE="$$f"; break; \
		fi; \
	done; \
	if [ -z "$$WASM_FILE" ]; then \
		echo "ERROR: no compiled WASM artifact found matching hash $(WASM_HASH)."; \
		echo "Restore or build the matching WASM artifact and retry."; \
		exit 1; \
	fi; \
	@if [ "$(NETWORK)" = "mainnet" ]; then \
		echo "WARNING: mainnet rollback is sensitive. This will redeploy WASM hash $(WASM_HASH) to mainnet."; \
		printf "Type 'ROLLBACK' to confirm: "; \
		read CONFIRM; \
		if [ "$$CONFIRM" != "ROLLBACK" ]; then \
			echo "Aborted rollback."; \
			exit 1; \
		fi; \
	fi; \
	@echo "Rolling back with artifact: $$WASM_FILE"; \
	soroban contract deploy --wasm "$$WASM_FILE" --network $(NETWORK)

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

## Run cargo-deny to check licenses, advisories, bans, and sources.
## Requires: cargo install --locked cargo-deny
deny:
	@echo "Running cargo-deny checks..."
	cargo deny check

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

# ── Signing key alias (used by deploy and verify) ─────────────────────────────
# SOURCE is the stellar key alias (not the raw secret) passed to stellar CLI.
# For deploy, ADMIN_SECRET must also be exported so the CLI can sign.
SOURCE ?= deployer

## Build, optimize, and deploy the contract to NETWORK.
## Requires: ADMIN_SECRET exported in the environment; SOURCE set to a key alias.
## After deploy, note the printed CONTRACT_ID and run: make verify CONTRACT_ID=... SOURCE=...
deploy: optimize
ifeq ($(NETWORK),mainnet)
	@echo "⚠  WARNING: Deploying to MAINNET. Press Ctrl-C within 5 seconds to abort."
	@sleep 5
endif
	@echo "Deploying TrustLink to $(NETWORK)..."
	stellar contract deploy \
		--wasm $(WASM_OPT) \
		--source $(SOURCE) \
		--network $(NETWORK)
	@echo ""
	@echo "════════════════════════════════════════════════════════════════"
	@echo "  Deployment complete."
	@echo "  ⚠  Run post-deployment verification before considering this done:"
	@echo "     make verify CONTRACT_ID=<printed-above> SOURCE=$(SOURCE) NETWORK=$(NETWORK)"
	@echo "════════════════════════════════════════════════════════════════"

## Invoke any contract function on NETWORK.
## Usage: make invoke ARGS="-- <function> [--param value ...]"
invoke:
	@test -n "$(CONTRACT_ID)" || (echo "Error: CONTRACT_ID is required. Export it or pass CONTRACT_ID=C..."; exit 1)
	stellar contract invoke \
		--id $(CONTRACT_ID) \
		--source $(SOURCE) \
		--network $(NETWORK) \
		$(ARGS)

testnet:
	$(MAKE) deploy NETWORK=testnet

mainnet:
	$(MAKE) deploy NETWORK=mainnet

local:
	$(MAKE) deploy NETWORK=local

## Run post-deployment verification against a live TrustLink contract.
## Executes scripts/verify_deployment.sh which creates a temporary issuer,
## issues a test attestation, verifies it, revokes it, and cleans up.
##
## Required: CONTRACT_ID=<contract-id>  SOURCE=<stellar-key-alias>
## Optional: NETWORK=testnet|mainnet    (default: testnet)
##
## Example:
##   make verify CONTRACT_ID=CABC...XYZ SOURCE=deployer NETWORK=testnet
verify:
	@test -n "$(CONTRACT_ID)" || (echo "Error: CONTRACT_ID is required.  Usage: make verify CONTRACT_ID=C... SOURCE=<alias> [NETWORK=testnet|mainnet]"; exit 1)
	@test -n "$(SOURCE)"      || (echo "Error: SOURCE is required.  Usage: make verify CONTRACT_ID=C... SOURCE=<alias> [NETWORK=testnet|mainnet]"; exit 1)
	@echo "Running deployment verification for $(CONTRACT_ID) on $(NETWORK)..."
	bash scripts/verify_deployment.sh \
		--contract $(CONTRACT_ID) \
		--source   $(SOURCE) \
		--network  $(NETWORK)
