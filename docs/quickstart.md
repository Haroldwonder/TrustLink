# Quickstart

Get a TrustLink contract deployed and verifying claims in a few minutes.

## 1. Prerequisites

| Tool | Install |
|---|---|
| Rust (stable) | https://rustup.rs |
| wasm32 target | `rustup target add wasm32-unknown-unknown` |
| Soroban CLI | `cargo install --locked soroban-cli` |

## 2. Use the testnet deployment

A TrustLink instance is already deployed on Stellar Testnet, so you can start integrating without deploying your own:

```
Contract ID: CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8
Network Passphrase: Test SDF Network ; September 2015
RPC URL: https://soroban-testnet.stellar.org
```

## 3. Or run your own local instance

```bash
# Clone
git clone https://github.com/Haroldwonder/TrustLink.git
cd TrustLink

# Start a local Stellar Quickstart node
docker compose up -d

# Build, deploy, and initialize locally
make local-deploy
```

The deployed contract ID is written to `.local.contract-id`.

## 4. Verify a claim (Rust cross-contract call)

```rust
let trustlink = trustlink::Client::new(&env, &trustlink_id);
let claim = String::from_str(&env, "KYC_PASSED");

if !trustlink.has_valid_claim(&subject, &claim) {
    return Err(Error::KYCRequired);
}
```

## 5. Next steps

- Full Rust and TypeScript integration patterns: [docs/integration-guide.md](integration-guide.md)
- Key terms used throughout the docs: [docs/glossary.md](glossary.md)
- Security model and trust hierarchy: [docs/security.md](security.md)
- Unfamiliar with Soroban concepts like TTL or `require_auth`? See [docs/stellar-concepts.md](stellar-concepts.md)

Translations of this guide: [Español](i18n/es/quickstart.md)
