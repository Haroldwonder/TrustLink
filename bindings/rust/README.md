# trustlink-client

Rust RPC client for the [TrustLink](https://github.com/afurious/TrustLink) on-chain attestation contract on Stellar/Soroban.

This crate is a **thin async HTTP client** that talks to a Soroban RPC node. It is distinct from the on-chain contract crate (`trustlink`) — no Soroban SDK or WASM target is required. It lets Rust backend services, CLIs, and Rust-based indexers query TrustLink without going through a TypeScript or Python SDK.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
trustlink-client = { path = "../bindings/rust" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Quick Start

```rust
use trustlink_client::{TrustLinkClient, Networks};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TrustLinkClient::new(
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8",
        Networks::TESTNET,
    )?;

    // Check a wallet's KYC status
    let has_kyc = client
        .has_valid_claim("GABC...SUBJECT_ADDRESS", "KYC_PASSED")
        .await?;
    println!("Has valid KYC: {has_kyc}");

    // Fetch a single attestation by ID
    let att = client.get_attestation("att_abc123").await?;
    println!("Claim: {} issued by {}", att.claim_type, att.issuer);

    // Page through all attestations for a subject
    let page = client
        .get_subject_attestations("GABC...SUBJECT_ADDRESS", 0, 20)
        .await?;
    println!("First page: {} attestations", page.len());

    Ok(())
}
```

## API Reference

### Claim verification

| Method | Contract function |
|---|---|
| `has_valid_claim(subject, claim_type)` | `has_valid_claim` |
| `has_valid_claim_from_issuer(subject, claim_type, issuer)` | `has_valid_claim_from_issuer` |
| `has_any_claim(subject, &[claim_types])` | `has_any_claim` |
| `has_all_claims(subject, &[claim_types])` | `has_all_claims` |

### Attestation queries

| Method | Contract function |
|---|---|
| `get_attestation(id)` | `get_attestation` |
| `get_attestation_status(id)` | `get_attestation_status` |
| `get_subject_attestations(subject, offset, limit)` | `get_subject_attestations` |
| `get_issuer_attestations(issuer, offset, limit)` | `get_issuer_attestations` |
| `is_issuer(address)` | `is_issuer` |
| `get_global_stats()` | `get_global_stats` |

### Networks

```rust
Networks::TESTNET  // https://soroban-testnet.stellar.org
Networks::MAINNET  // Stellar mainnet RPC
Networks::LOCAL    // http://localhost:8000/soroban/rpc
```

Or pass any custom URL string directly to `TrustLinkClient::new`.

## Error handling

All methods return `Result<T, TrustLinkError>`. Contract-level errors (e.g. `NotFound`, `Unauthorized`) surface as `TrustLinkError::Contract` with a typed `ContractErrorCode`:

```rust
use trustlink_client::{TrustLinkError, ContractErrorCode};

match client.get_attestation("bad_id").await {
    Ok(att) => println!("{}", att.id),
    Err(TrustLinkError::Contract { code: ContractErrorCode::NotFound, .. }) => {
        eprintln!("attestation not found");
    }
    Err(e) => eprintln!("error: {e}"),
}
```

## Design

All queries are executed as **simulated** (read-only) Soroban transactions via the JSON-RPC `simulateTransaction` endpoint. No signing key or XLM balance is required. The client is a pure async HTTP client — no Soroban SDK runtime or WASM toolchain dependency.

## Testing

```bash
cd bindings/rust
cargo test
```

Tests use `mockito` to stub the Soroban RPC endpoint and run fully offline.

## Relationship to other packages

| Package | Purpose |
|---|---|
| `trustlink` (repo root) | On-chain Soroban contract (WASM) |
| `bindings/rust` (this crate) | **Rust RPC client for off-chain use** |
| `bindings/typescript` | Auto-generated TypeScript contract bindings |
| `sdk/typescript` | Higher-level TypeScript SDK |
| `bindings/python` | Python RPC client |

## License

MIT
