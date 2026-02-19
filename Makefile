.PHONY: build test optimize clean install help

help:
	@echo "TrustLink Smart Contract - Makefile Commands"
	@echo "============================================="
	@echo "make build     - Build the contract in debug mode"
	@echo "make test      - Run all unit tests"
	@echo "make optimize  - Build optimized release version"
	@echo "make clean     - Clean build artifacts"
	@echo "make install   - Install required dependencies"

install:
	@echo "Installing Rust and Soroban CLI..."
	@echo "Please ensure you have Rust installed: https://rustup.rs/"
	@echo "Install Soroban CLI: cargo install --locked soroban-cli"

build:
	@echo "Building TrustLink contract..."
	cargo build --target wasm32-unknown-unknown --release

test:
	@echo "Running tests..."
	cargo test

optimize:
	@echo "Building optimized contract..."
	cargo build --target wasm32-unknown-unknown --release
	@echo "Optimizing WASM..."
	soroban contract optimize --wasm target/wasm32-unknown-unknown/release/trustlink.wasm

clean:
	@echo "Cleaning build artifacts..."
	cargo clean

fmt:
	@echo "Formatting code..."
	cargo fmt

clippy:
	@echo "Running clippy..."
	cargo clippy --all-targets -- -D warnings
