# TrustLink Deployment Guide

## Prerequisites

Before deploying TrustLink, ensure you have:

1. **Rust** (1.70 or later)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Soroban CLI**
   ```bash
   cargo install --locked soroban-cli
   ```

3. **wasm32 target**
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

## Building

### Development Build
```bash
cargo build --target wasm32-unknown-unknown --release
```

### Optimized Build
```bash
cargo build --target wasm32-unknown-unknown --release
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm
```

## Testing

### Run All Tests
```bash
cargo test
```

### Run Specific Test
```bash
cargo test test_create_attestation
```

### Run with Output
```bash
cargo test -- --nocapture
```

## Deployment

### 1. Deploy to Testnet

```bash
# Deploy the contract
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
  --source ADMIN_SECRET_KEY \
  --network testnet

# Save the contract ID
export CONTRACT_ID=<returned_contract_id>
```

### 2. Initialize the Contract

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --source ADMIN_SECRET_KEY \
  --network testnet \
  -- initialize \
  --admin ADMIN_PUBLIC_ADDRESS
```

### 3. Register an Issuer

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --source ADMIN_SECRET_KEY \
  --network testnet \
  -- register_issuer \
  --admin ADMIN_PUBLIC_ADDRESS \
  --issuer ISSUER_PUBLIC_ADDRESS
```

### 4. Create an Attestation

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --source ISSUER_SECRET_KEY \
  --network testnet \
  -- create_attestation \
  --issuer ISSUER_PUBLIC_ADDRESS \
  --subject USER_PUBLIC_ADDRESS \
  --claim_type "KYC_PASSED" \
  --expiration null
```

### 5. Verify a Claim

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  -- has_valid_claim \
  --subject USER_PUBLIC_ADDRESS \
  --claim_type "KYC_PASSED"
```

## Mainnet Deployment

### Important Considerations

1. **Audit**: Have the contract professionally audited before mainnet deployment
2. **Testing**: Thoroughly test on testnet with real-world scenarios
3. **Admin Key**: Use a multisig or hardware wallet for the admin key
4. **Monitoring**: Set up monitoring for contract events
5. **Backup**: Keep backups of all deployment configurations

### Deployment Steps

```bash
# 1. Build optimized contract
make optimize

# 2. Deploy to mainnet
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trustlink_optimized.wasm \
  --source ADMIN_SECRET_KEY \
  --network mainnet

# 3. Initialize with production admin
soroban contract invoke \
  --id $CONTRACT_ID \
  --source ADMIN_SECRET_KEY \
  --network mainnet \
  -- initialize \
  --admin PRODUCTION_ADMIN_ADDRESS

# 4. Register production issuers
# (Repeat for each trusted issuer)
soroban contract invoke \
  --id $CONTRACT_ID \
  --source ADMIN_SECRET_KEY \
  --network mainnet \
  -- register_issuer \
  --admin PRODUCTION_ADMIN_ADDRESS \
  --issuer ISSUER_ADDRESS
```

## Configuration

### Network Configuration

Create a `soroban-config.toml`:

```toml
[network.testnet]
rpc-url = "https://soroban-testnet.stellar.org"
network-passphrase = "Test SDF Network ; September 2015"

[network.mainnet]
rpc-url = "https://soroban-mainnet.stellar.org"
network-passphrase = "Public Global Stellar Network ; September 2015"
```

## Monitoring

### Watch Contract Events

```bash
soroban events \
  --id $CONTRACT_ID \
  --network testnet \
  --start-ledger LEDGER_NUMBER
```

### Query Contract State

```bash
# Get admin
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  -- get_admin

# Check if address is issuer
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  -- is_issuer \
  --address ADDRESS
```

## Troubleshooting

### Build Errors

If you encounter build errors:
```bash
cargo clean
cargo update
cargo build --target wasm32-unknown-unknown --release
```

### Test Failures

Run tests with verbose output:
```bash
cargo test -- --nocapture --test-threads=1
```

### Deployment Issues

Check network connectivity:
```bash
soroban network ls
soroban config identity ls
```

## Security Best Practices

1. **Key Management**
   - Never commit private keys to version control
   - Use environment variables or secure key management systems
   - Rotate keys regularly

2. **Access Control**
   - Limit the number of authorized issuers
   - Implement a process for issuer vetting
   - Monitor issuer activity

3. **Monitoring**
   - Set up alerts for unusual activity
   - Monitor attestation creation rates
   - Track revocation patterns

4. **Upgrades**
   - Plan for contract upgrades if needed
   - Test upgrades thoroughly on testnet
   - Communicate changes to users

## Support

For issues or questions:
- GitHub Issues: [Your Repository]
- Documentation: See README.md
- Community: [Your Discord/Forum]
