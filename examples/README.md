# TrustLink Examples

This directory contains reference implementations demonstrating TrustLink integration patterns across different platforms and use cases. Each example is self-contained and can be run independently.

## Quick Navigation

### Soroban Smart Contract Examples

These examples demonstrate how to integrate TrustLink into Rust-based Soroban smart contracts, using attestation verification to gate access to contract functionality.

| Example | Description | Learn About |
|---------|-------------|-------------|
| [**governance/**](governance/) | KYC-gated DAO voting | Restrict governance operations to verified members |
| [**healthcare/**](healthcare/) | Health credential verification | Multi-claim attestation requirements |
| [**insurance/**](insurance/) | Policy underwriting | Combining multiple attestation checks (KYC + AML) |
| [**kyc-token/**](kyc-token/) | KYC-restricted token transfers | Token-level access control via attestations |
| [**real-estate/**](real-estate/) | Title registry & lien tracking | Long-lived attestation management patterns |
| [**supply-chain/**](supply-chain/) | Supply chain credential tracking | Entity verification in supply chains |

### JavaScript / Node.js Integration Examples

These examples show how to interact with TrustLink from JavaScript/Node.js backends and services, including issuer operations and client integration.

| Example | Description | Learn About |
|---------|-------------|-------------|
| [**anchor-integration/**](anchor-integration/) | Stellar anchor KYC flow | Sequence diagrams and anchor integration patterns |
| [**issuer-cli/**](issuer-cli/) | Issuer command-line tool | Creating, managing, and revoking attestations |
| [**freelance-reputation/**](freelance-reputation/) | Marketplace reputation scoring | Endorsement-weighted reputation systems |

### Reference Applications

Full reference applications demonstrating end-to-end TrustLink usage across different tech stacks.

| Example | Description | Learn About |
|---------|-------------|-------------|
| [**react-app/**](react-app/) | Full dApp UI (Vite + React + TypeScript) | Multi-wallet support, I18n, frontend integration |
| [**python-verification/**](python-verification/) | Backend verification service (Flask) | Server-side attestation verification, batch processing |

## Getting Started

### Prerequisites

All examples require:
- **Git** — for cloning the repository
- **Make** — for convenient command shortcuts
- **curl** or similar HTTP tool — for testnet interactions

**Language-specific requirements:**
- **Rust examples** — [Rust & Cargo](https://rustup.rs), [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup)
- **JavaScript examples** — [Node.js](https://nodejs.org) ≥ 18
- **React example** — [Node.js](https://nodejs.org) ≥ 18, modern browser
- **Python example** — [Python](https://www.python.org) ≥ 3.9, `pip` or `poetry`

### Running an Example

1. **Navigate to the example directory:**
   ```bash
   cd examples/<example-name>
   ```

2. **Read the example's README:**
   Each example has its own `README.md` with detailed setup and usage instructions.

3. **Install dependencies (if needed):**
   - **Rust:** Dependencies are managed via `Cargo.toml`
   - **JavaScript:** Run `npm install`
   - **Python:** Run `pip install -r requirements.txt`

4. **Deploy or run the example:**
   - **Rust examples:** Follow the contract deployment steps in the example's README
   - **JavaScript/Python examples:** Follow the service startup steps in the example's README

## Common Patterns

### Verifying Claims in a Soroban Contract

All Soroban examples follow a similar pattern for claim verification:

```rust
let trustlink = trustlink::Client::new(&env, &trustlink_contract_id);
let claim = String::from_str(&env, "KYC_PASSED");

if !trustlink.has_valid_claim(&subject, &claim) {
    return Err(Error::KYCRequired);
}
```

See [governance/](governance/) or [kyc-token/](kyc-token/) for complete examples.

### Checking Multiple Claims

To require multiple attestations:

```rust
let mut required = soroban_sdk::Vec::new(&env);
required.push_back(String::from_str(&env, "KYC_PASSED"));
required.push_back(String::from_str(&env, "AML_CLEARED"));

if !trustlink.has_all_claims(&borrower, &required) {
    return Err(Error::InsufficientCredentials);
}
```

See [insurance/](insurance/) for a complete example.

### Issuing Attestations (JavaScript)

Attestations are typically created via the TrustLink CLI or a JavaScript issuer service:

```javascript
const trustlink = sorobanServer.contract(trustlinkContractId);

await trustlink.invoke({
    method: 'create_attestation',
    params: {
        issuer: issuerAddress,
        subject: subjectAddress,
        claim_type: 'KYC_PASSED',
        expiration: null, // no expiration
    },
});
```

See [issuer-cli/](issuer-cli/) for command-line examples and [anchor-integration/](anchor-integration/) for sequence diagrams.

## Testing Your Contracts

All Rust examples include unit tests. Run them with:

```bash
cd examples/<contract-example>
cargo test
```

JavaScript examples may include integration tests. Check each example's README for instructions.

## Deployment

### Testnet

Most examples can be deployed to the Stellar testnet. Follow the deployment instructions in each example's README.

Fund a testnet account via [Friendbot](https://friendbot.stellar.org):

```bash
curl "https://friendbot.stellar.org?addr=YOUR_PUBLIC_KEY"
```

### Local Development

For local testing without a live testnet, use the Soroban local environment:

```bash
soroban network add --name local --rpc-url http://127.0.0.1:8000/soroban/rpc --network-passphrase "Local SorobanNet ; September 2024"
```

See individual example READMEs for local deployment steps.

## Environment Configuration

Many examples use `.env.example` files. To run an example:

```bash
cp .env.example .env
# Edit .env with your configuration (keys, contract IDs, etc.)
```

Never commit `.env` files containing private keys or secrets.

## Troubleshooting

### Build Failures

**Rust examples:**
- Ensure `wasm32-unknown-unknown` target is installed: `rustup target add wasm32-unknown-unknown`
- Check Rust version is up to date: `rustup update`

**JavaScript examples:**
- Clear npm cache: `npm cache clean --force`
- Reinstall dependencies: `rm -rf node_modules && npm install`

### Contract Deployment Issues

- Verify you have a funded testnet account (see Deployment section above)
- Check that your network configuration points to the correct testnet RPC
- Ensure the Soroban CLI is up to date: `cargo install --locked soroban-cli`

### Claim Verification Failures

- Verify the claim type matches exactly (case-sensitive)
- Check that the attestation hasn't expired
- Confirm the issuer is registered with the TrustLink contract
- Ensure the subject address is correct

## Contributing an Example

To add a new example:

1. Create a new directory under `examples/` following the naming convention (kebab-case)
2. Include a `README.md` explaining the example's purpose, setup, and usage
3. Include all necessary configuration files (Cargo.toml, package.json, etc.)
4. Add a one-line description and link in this main README's navigation table
5. Ensure the example is self-contained and can run independently

See [CONTRIBUTING.md](../CONTRIBUTING.md) for full contribution guidelines.

## Additional Resources

- **Main README** — [../../README.md](../README.md)
- **Integration Guide** — [../../docs/integration-guide.md](../docs/integration-guide.md)
- **Quickstart** — [../../docs/quickstart.md](../docs/quickstart.md)
- **API Reference** — [../../docs/](../docs/)
- **GitHub Issues** — [Report bugs or request features](https://github.com/unixfundz/TrustLink/issues)

## License

All examples are part of the TrustLink project and are licensed under the same terms. See [LICENSE](../LICENSE) for details.
