.PHONY: help build test optimize clean install fmt clippy

## Display this help message with all available targets
help:
	@echo "TrustLink Smart Contract - Available Make Targets"
	@echo "=================================================="
	@echo ""
	@awk '/^## / {desc=$$0; gsub(/^## /, "", desc); getline; if (/^[a-z]/) {target=$$1; gsub(/:/, "", target); printf "  make %-12s - %s\n", target, desc}}' $(MAKEFILE_LIST)
	@echo ""

## Install required dependencies (Rust and Soroban CLI)
install:
	@echo "Installing Rust and Soroban CLI..."
	@echo "Please ensure you have Rust installed: https://rustup.rs/"
	@echo "Install Soroban CLI: cargo install --locked soroban-cli"

## Build the contract in debug mode
build:
	@echo "Building TrustLink contract..."
	cargo build --target wasm32-unknown-unknown --release

## Run all unit tests
test:
	@echo "Running tests..."
	cargo test

## Build optimized release version with WASM optimization
optimize:
	@echo "Building optimized contract..."
	cargo build --target wasm32-unknown-unknown --release
	@echo "Optimizing WASM..."
	soroban contract optimize --wasm target/wasm32-unknown-unknown/release/trustlink.wasm

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

coverage:
	@echo "Generating coverage report..."
	cargo llvm-cov --html
	@echo "Coverage report generated in target/llvm-cov/html/index.html"
