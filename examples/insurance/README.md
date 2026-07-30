# Insurance Example

This example demonstrates how to build a parametric insurance smart contract on Stellar using TrustLink's oracle infrastructure.

## Overview

The insurance contract accepts premiums from policyholders and automatically pays out claims when predefined conditions are met, as verified by TrustLink's decentralized oracle network.

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│ Policyholder │────▶│   Insurance   │◀────│   Oracle    │
│   (User)     │     │   Contract    │     │  (TrustLink)│
└─────────────┘     └──────────────┘     └─────────────┘
                            │
                            ▼
                    ┌──────────────┐
                    │   Payout     │
                    │   (XLM/USDC) │
                    └──────────────┘
```

## Features

- **Parametric triggers**: Payouts are triggered automatically when oracle-reported conditions cross predefined thresholds
- **Multi-oracle consensus**: Requires confirmation from multiple independent oracles before triggering payouts
- **Premium management**: Handles premium collection, escrow, and refund logic
- **Dispute resolution**: Built-in time windows for challenging oracle reports
- **Gas optimization**: Batched operations minimize transaction costs on Stellar

## Prerequisites

- Rust 1.70+ with `wasm32-unknown-unknown` target
- Stellar CLI (`soroban-cli`) for local testing
- A funded Stellar testnet account

```bash
rustup target add wasm32-unknown-unknown
cargo install soroban-cli
```

## Quick Start

### 1. Build the contract

```bash
cd examples/insurance
cargo build --target wasm32-unknown-unknown --release
```

### 2. Run tests

```bash
cargo test
```

Expected output:
```
running 5 tests
test test_initialize_contract ... ok
test test_collect_premium ... ok
test test_trigger_payout ... ok
test test_dispute_window ... ok
test test_multi_oracle_consensus ... ok
```

### 3. Deploy to testnet

```bash
soroban contract deploy   --wasm target/wasm32-unknown-unknown/release/insurance.wasm   --source <your-key>   --network testnet
```

### 4. Initialize the contract

```bash
soroban contract invoke   --id <contract-id>   --source <your-key>   --network testnet   -- initialize   --oracle <oracle-address>   --threshold 1000   --premium 100   --coverage 10000
```

## Contract Interface

### Data Structures

| Structure | Fields | Description |
|-----------|--------|-------------|
| `Policy` | `policyholder`, `premium`, `coverage`, `start_time`, `end_time`, `status` | Represents an active insurance policy |
| `OracleReport` | `oracle_id`, `value`, `timestamp`, `signature` | Oracle-submitted data point |
| `Claim` | `policy_id`, `amount`, `status`, `filed_at` | Payout claim record |

### Functions

| Function | Access | Description |
|----------|--------|-------------|
| `initialize(oracle, threshold, premium, coverage)` | Admin | Set up contract parameters |
| `purchase_policy()` | Public | Pay premium and activate coverage |
| `submit_report(value)` | Oracle | Submit oracle data point |
| `file_claim(policy_id)` | Policyholder | Request payout |
| `dispute_report(report_id)` | Policyholder | Challenge an oracle report |
| `resolve_dispute(report_id)` | Admin | Resolve after dispute window |
| `withdraw_premiums()` | Admin | Collect accumulated premiums |

## Testing

The test suite covers:

- **Unit tests**: Contract functions in isolation (`tests/test_contract.rs`)
- **Integration tests**: End-to-end flows with mock oracles (`tests/test_integration.rs`)
- **Edge cases**: Expired policies, duplicate reports, invalid signatures

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_trigger_payout

# With verbose output
cargo test -- --nocapture
```

## Configuration

Environment variables for local development:

| Variable | Default | Description |
|----------|---------|-------------|
| `STELLAR_NETWORK` | `testnet` | Stellar network to use |
| `ORACLE_COUNT` | `3` | Minimum oracles for consensus |
| `DISPUTE_WINDOW` | `3600` | Dispute window in seconds |
| `PREMIUM_DENOM` | `XLM` | Premium payment token |

## Security Considerations

- **Oracle trust**: The contract relies on TrustLink's oracle reputation system — verify oracle identities before deployment
- **Premium escrow**: All premiums are held in the contract until payout or expiry
- **Dispute window**: A 1-hour window allows challenging potentially incorrect oracle reports
- **Re-entrancy protection**: All state-changing functions follow checks-effects-interactions pattern
- **Overflow protection**: Uses Rust's safe math with explicit bounds checking

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `Error: Contract not found` | Ensure the contract was deployed with `soroban contract deploy` |
| `Error: Insufficient balance` | Fund your testnet account at the Stellar friendbot |
| `Error: Oracle not authorized` | Only registered oracle addresses can submit reports |
| `wasm32 target not installed` | Run `rustup target add wasm32-unknown-unknown` |

## Related Examples

- [Lending](../lending/README.md) — DeFi lending with oracle-backed collateral
- [Prediction Market](../prediction-market/README.md) — Binary outcome markets
- [Escrow](../escrow/README.md) — Time-locked escrow with oracle release conditions

## License

This example is part of TrustLink and is licensed under the same terms as the parent project.
