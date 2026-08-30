# TrustLink Deployment Environments

## Overview

This document describes different deployment strategies and environments for the TrustLink Soroban smart contract, analogous to development/testing/production configurations.

## Deployment Scenarios

### 1. Local Development Environment

**Purpose:** Development and testing on a local Stellar network

**Setup:**
```bash
# Start local Stellar network (requires Docker)
stellar network start local

# Build the contract
cargo build --target wasm32-unknown-unknown --release

# Deploy to local network
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
  --source <local-key-alias> \
  --network local
```

**Characteristics:**
- ✅ Instant finality
- ✅ Unlimited test tokens
- ✅ Full control over state
- ✅ No gas costs
- ❌ Cannot interact with mainnet/testnet contracts
- ❌ Data reset when restarting network

**Use Cases:**
- Feature development
- Unit testing
- Contract behavior exploration
- Integration with other local contracts

### 2. Stellar Testnet Environment

**Purpose:** Pre-production testing on public testnet

**Setup:**
```bash
# Ensure testnet credentials are set up
export TESTNET_SECRET=<your-testnet-secret-key>

# Deploy to testnet
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
  --source testnet-key \
  --network testnet
```

**Network Details:**
- RPC URL: `https://soroban-testnet.stellar.org`
- Network ID: `Test SDF Network ; September 2015`
- Block time: ~5 seconds
- Data retention: 6 months (or as specified by Stellar)

**Characteristics:**
- ✅ Public, persistent ledger
- ✅ Real transaction costs (minimal)
- ✅ Close to mainnet behavior
- ✅ Can interact with other testnet contracts
- ⚠️ Data may be reset during network upgrades
- ⚠️ 100 XLM account minimum

**Use Cases:**
- Integration testing
- User acceptance testing (UAT)
- Security audit staging
- Pre-mainnet verification
- Cross-contract interactions

**Getting Test XLM:**
```bash
# Request testnet funds
curl "https://friendbot.stellar.org?addr=<your-public-key>"
```

### 3. Stellar Mainnet Environment

**Purpose:** Production deployment on Stellar mainnet

**Setup:**
```bash
# Verify mainnet credentials are secure
export MAINNET_SECRET=<your-mainnet-secret-key>

# Deploy to mainnet (REQUIRES AUDIT & APPROVAL)
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
  --source mainnet-key \
  --network mainnet
```

**Network Details:**
- RPC URL: `https://mainnet.stellar.validationcloud.io/v1/<api-key>`
- Network ID: `Public Global Stellar Network ; September 2015`
- Block time: ~5 seconds
- Data retention: Permanent

**Characteristics:**
- ✅ Production-grade ledger
- ✅ Permanent data storage
- ✅ Real economic value (real XLM costs)
- ✅ Global audience access
- ❌ Real financial implications
- ❌ Immutable contract deployments
- ❌ No reset/recovery mechanism

**Pre-Mainnet Checklist:**
- [ ] Security audit completed by reputable firm
- [ ] All audit findings resolved
- [ ] Comprehensive test coverage (>90%)
- [ ] Testnet deployment verified for minimum 2 weeks
- [ ] Admin key secured (hardware wallet / multisig recommended)
- [ ] Incident response runbook prepared
- [ ] Monitoring and alerting configured
- [ ] Rate limiting / access controls implemented
- [ ] Emergency pause mechanism documented (if applicable)
- [ ] Legal/compliance review completed

**Use Cases:**
- Live user deployments
- Real attestation issuance
- Production integrations

## Configuration Management

### Environment Variables

**Development (Local)**
```bash
export NETWORK=local
export CONTRACT_ID=<local-contract-id>
export RPC_URL=http://localhost:8000/soroban/rpc
export SIGNING_KEY=<local-test-key>
```

**Staging (Testnet)**
```bash
export NETWORK=testnet
export CONTRACT_ID=<testnet-contract-id>
export RPC_URL=https://soroban-testnet.stellar.org
export SIGNING_KEY=<testnet-key>  # Set only when needed
```

**Production (Mainnet)**
```bash
export NETWORK=mainnet
export CONTRACT_ID=<mainnet-contract-id>
export RPC_URL=https://mainnet.stellar.validationcloud.io/v1/<api-key>
export SIGNING_KEY=  # NEVER set in environment; use secure key management
```

### Build Configuration

**Debug Build (Local Development)**
```bash
cargo build --target wasm32-unknown-unknown
# Output: target/wasm32-unknown-unknown/debug/trustlink.wasm
# Size: ~800 KB
# Optimization: None (faster builds)
```

**Release Build (Testnet/Mainnet)**
```bash
cargo build --target wasm32-unknown-unknown --release
# Output: target/wasm32-unknown-unknown/release/trustlink.wasm
# Size: ~600 KB
# Optimization: Basic
```

**Optimized Build (Mainnet Production)**
```bash
cargo build --target wasm32-unknown-unknown --release
wasm-opt -Oz --enable-bulk-memory --strip-debug \
  target/wasm32-unknown-unknown/release/trustlink.wasm \
  -o target/trustlink.optimized.wasm
# Output: target/trustlink.optimized.wasm
# Size: ~200-300 KB
# Optimization: Maximum (slower builds, smaller binary)
```

## Migration Between Environments

### Local → Testnet
1. Ensure testnet account has minimum 100 XLM
2. Update environment variables to point to testnet
3. Deploy new instance to testnet
4. Populate with test data
5. Perform acceptance testing

### Testnet → Mainnet
1. ✅ Complete security audit
2. ✅ Verify testnet deployment for 2+ weeks
3. ✅ Secure admin key (hardware wallet/multisig)
4. Deploy new instance to mainnet
5. Monitor for 24-48 hours
6. Announce publicly

**Note:** Contract instances are environment-specific. You cannot "migrate" a contract; you deploy a new instance in each environment.

## Monitoring & Observability

### Logs & Events
- All state changes emit events (see `src/events.rs`)
- Query events via: `soroban contract invoke --id <id> --function get_events`
- Events are indexed by validators/indexers

### Health Checks
```bash
# Verify contract is callable
soroban contract invoke \
  --id <contract-id> \
  --function is_initialized \
  --network <network>

# Check admin address
soroban contract invoke \
  --id <contract-id> \
  --function get_admin \
  --network <network>
```

### Contract Upgrade Strategy
⚠️ **Important:** Soroban contracts cannot be upgraded in-place. To deploy new versions:

1. Deploy new contract instance with updated WASM
2. Migrate state to new instance (if needed)
3. Update clients to point to new contract ID
4. Archive old contract reference (for historical audit trail)

## Security Considerations

### Key Management
- **Development:** Use temporary, test-only keys
- **Testnet:** Use separate testnet-only keys
- **Mainnet:** Use hardware wallet or multisig setup
  - Never commit private keys to git
  - Use secure key management service (AWS KMS, HashiCorp Vault, etc.)
  - Implement key rotation policy

### Rate Limiting
- Monitor invocation rates
- Implement per-issuer rate limits (if applicable)
- Set up alerts for unusual activity

### Admin Functions
- Restrict initialization to admin-only
- Require multi-signature for sensitive operations
- Document admin procedures

## Related Files
- `DEPLOYMENT.md` - Step-by-step deployment procedures
- `docs/CARGO_LOCK_POLICY.md` - Dependency management
- `Makefile` - Build commands for each environment
- `.github/workflows/` - CI/CD configuration

## References
- [Stellar Documentation](https://developers.stellar.org/docs)
- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Stellar Testnet Guide](https://developers.stellar.org/docs/learn/networks)
