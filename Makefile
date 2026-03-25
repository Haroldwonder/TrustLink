.PHONY: build test optimize clean install help local-deploy

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
        help

# ─────────────────────────────────────────────────────────────────────────────
# Help
# ─────────────────────────────────────────────────────────────────────────────
help:
	@echo "TrustLink Smart Contract - Makefile Commands"
	@echo "============================================="
	@echo "make build     - Build the contract in debug mode"
	@echo "make test      - Run all unit tests"
	@echo "make optimize  - Build optimized release version"
	@echo "make clean     - Clean build artifacts"
	@echo "make install   - Install required dependencies"
	@echo "make local-deploy - Deploy and initialize contract on local Stellar network"

# ─────────────────────────────────────────────────────────────────────────────
# Build & test
# ─────────────────────────────────────────────────────────────────────────────
install:
	@echo "Required dependencies:"
	@echo "  Rust:        https://rustup.rs/"
	@echo "  Stellar CLI: cargo install --locked stellar-cli --features opt"
	@echo "  WASM target: rustup target add wasm32-unknown-unknown"

## Build the contract in debug mode
build:
	@echo "Building TrustLink ($(NETWORK))..."
	cargo build --target wasm32-unknown-unknown --release

## Run all unit tests
test:
	@echo "Running tests..."
	cargo test

optimize: build
	@echo "Optimizing WASM..."
	stellar contract optimize --wasm $(WASM)
	@echo "Optimized artifact: $(WASM_OPT)"

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
