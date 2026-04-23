# TrustLink Deployment Guide

## Testnet Deployment (Current)

| Field            | Value                                                          |
|------------------|----------------------------------------------------------------|
| Network          | Stellar Testnet                                                |
| Contract ID      | `CAK7PYYSWWQH6ML3ZPO4OB2EIONODOEESE3MIV3YGFDMHEU4EUOBUJQN`  |
| Admin Address    | `GAZVF7TR4TVVSQDRK3BFSJ45B346GXDJIN2UWFHQKR7VIC4YK22DURP3`  |
| Deploy Tx        | `8f71db61279bde4667f216ad7e5d3a6c34155ccf7ac13862d8b7b11327acd68d` |
| Init Tx          | `c52d9687ac81205ddece7bef9736b0a882c80aadc08505ebd2b1431ba8177add` |
| WASM Hash        | `241d5e76ed3425335ad0220d2d73d8622fc71da9add7bee47eac35227cc5d4ff` |
| Explorer         | https://stellar.expert/explorer/testnet/contract/CAK7PYYSWWQH6ML3ZPO4OB2EIONODOEESE3MIV3YGFDMHEU4EUOBUJQN |

### Verified
```
$ stellar contract invoke --id CAK7PYYSWWQH6ML3ZPO4OB2EIONODOEESE3MIV3YGFDMHEU4EUOBUJQN \
    --source deployer --network testnet -- get_admin
"GAZVF7TR4TVVSQDRK3BFSJ45B346GXDJIN2UWFHQKR7VIC4YK22DURP3"
```

## Deployment Verification Script

`scripts/verify_deployment.sh` runs an end-to-end check against a deployed contract:

1. Verifies `get_admin` returns the expected admin address
2. Registers a temporary test issuer
3. Creates a test attestation (`VERIFY_TEST` claim type)
4. Asserts `has_valid_claim` returns `true`
5. Revokes the attestation
6. Asserts `has_valid_claim` returns `false`
7. Cleans up temporary test identities

### Usage

```bash
./scripts/verify_deployment.sh \
  --contract CAK7PYYSWWQH6ML3ZPO4OB2EIONODOEESE3MIV3YGFDMHEU4EUOBUJQN \
  --source deployer \
  --network testnet
```

Against mainnet:
```bash
./scripts/verify_deployment.sh \
  --contract <MAINNET_CONTRACT_ID> \
  --source <ADMIN_KEY_ALIAS> \
  --network mainnet
```

Exits with code `0` on success, non-zero on any failure. All steps are logged to stdout.

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

### Health Check

The `health_check` function returns a lightweight status snapshot without requiring authentication – ideal for uptime probes and monitoring dashboards.

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  -- health_check
```

Example response:

```json
{
  "initialized": true,
  "admin_set": true,
  "issuer_count": 3,
  "total_attestations": 142
}
```

Integrate this into automated monitoring by polling periodically and alerting when `initialized` is `false` or `issuer_count` drops to zero unexpectedly.

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

## Upgrades

TrustLink supports in-place WASM upgrades via Soroban's built-in upgrade mechanism. The contract address and all storage (admin, issuers, attestations) are preserved — only the executable code is replaced.

### How It Works

1. The new WASM is uploaded to the ledger, producing a 32-byte hash.
2. The admin calls `upgrade(admin, new_wasm_hash)` on the deployed contract.
3. Soroban replaces the contract's executable atomically.
4. If the new version requires storage schema changes, call a `migrate` function (defined in the new WASM) immediately after upgrading.

### Step-by-Step Upgrade Process

**1. Build and optimize the new contract version**

```bash
cargo build --target wasm32-unknown-unknown --release
stellar contract optimize \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm
```

**2. Upload the new WASM to the network**

```bash
stellar contract upload \
  --source ADMIN_SECRET_KEY \
  --network testnet \
  --wasm target/wasm32-unknown-unknown/release/trustlink.optimized.wasm
# Outputs: <NEW_WASM_HASH>
```

**3. Invoke the upgrade function**

```bash
stellar contract invoke \
  --id $CONTRACT_ID \
  --source ADMIN_SECRET_KEY \
  --network testnet \
  -- upgrade \
  --admin ADMIN_PUBLIC_ADDRESS \
  --new_wasm_hash <NEW_WASM_HASH>
```

**4. (If applicable) Run migration**

If the new contract version includes a `migrate` function for storage schema changes, call it immediately after upgrading:

```bash
stellar contract invoke \
  --id $CONTRACT_ID \
  --source ADMIN_SECRET_KEY \
  --network testnet \
  -- migrate
```

**5. Verify the upgrade**

```bash
# Confirm admin and state are intact
stellar contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  -- get_admin
```

### Security Notes

- Only the registered admin can trigger an upgrade.
- The new WASM must be uploaded to the ledger before calling `upgrade` — the hash must resolve.
- An `upgraded` event is emitted on success, allowing off-chain indexers to detect the change.
- Always test the upgrade on testnet before mainnet.
- Use a multisig or hardware wallet for the admin key on mainnet.

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
