# TrustLink RPC Provider Failover List

This document maintains an up-to-date list of tested RPC providers for Stellar mainnet and testnet. Used during `Scenario 2: RPC Provider Outage` in [docs/disaster-recovery.md](./disaster-recovery.md).

**Last updated:** 2026-06-27  
**Next review:** 2026-09-27 (quarterly)

---

## Mainnet RPC Providers

Test all providers quarterly. Update this list if a provider becomes unavailable or degrades.

### Priority 1 (Preferred)

| Provider | URL | Operator | Status | Last Tested | Notes |
|----------|-----|----------|--------|-------------|-------|
| Stellar Soroban Mainnet | `https://soroban-mainnet.stellar.org` | Stellar Foundation | ✅ Active | 2026-06-27 | Primary endpoint; high availability |
| SDF Public RPC | `https://rpc.soroban.io` | Stellar Dev Foundation | ✅ Active | 2026-06-27 | Backup; reliable alternative |

### Priority 2 (Backup)

| Provider | URL | Operator | Status | Last Tested | Notes |
|----------|-----|----------|--------|-------------|-------|
| Private RPC Node | `https://rpc-internal.company.io` | TrustLink Operations | 🟡 Pending setup | — | In-house RPC; setup planned for Q3 2026 |

### Deprecated / Unavailable

| Provider | URL | Reason | Sunset Date |
|----------|-----|--------|-------------|
| (none currently) | — | — | — |

---

## Testnet RPC Providers

Testnet is used for pre-production testing and disaster recovery drills.

| Provider | URL | Operator | Status | Last Tested | Notes |
|----------|-----|----------|--------|-------------|-------|
| Stellar Soroban Testnet | `https://soroban-testnet.stellar.org` | Stellar Foundation | ✅ Active | 2026-06-27 | Primary testnet endpoint |
| SDF Public Testnet | `https://rpc-testnet.soroban.io` | Stellar Dev Foundation | ✅ Active | 2026-06-27 | Backup testnet endpoint |

---

## RPC Health Check Procedure

Run these tests quarterly (or when adding a new provider) to ensure all providers are responsive:

```bash
#!/bin/bash
# docs/scripts/test-rpc-health.sh

set -e

MAINNET_PROVIDERS=(
  "https://soroban-mainnet.stellar.org"
  "https://rpc.soroban.io"
)

TESTNET_PROVIDERS=(
  "https://soroban-testnet.stellar.org"
  "https://rpc-testnet.soroban.io"
)

echo "=== Testing Mainnet RPC Providers ==="
for rpc in "${MAINNET_PROVIDERS[@]}"; do
  echo -n "Testing $rpc ... "
  response=$(curl -s -X POST "$rpc/soroban/rpc" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger","params":[]}')
  
  if echo "$response" | jq -e '.result.sequence' > /dev/null 2>&1; then
    ledger=$(echo "$response" | jq '.result.sequence')
    echo "✅ OK (ledger: $ledger)"
  else
    echo "❌ FAILED"
    echo "Response: $response"
  fi
done

echo ""
echo "=== Testing Testnet RPC Providers ==="
for rpc in "${TESTNET_PROVIDERS[@]}"; do
  echo -n "Testing $rpc ... "
  response=$(curl -s -X POST "$rpc/soroban/rpc" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger","params":[]}')
  
  if echo "$response" | jq -e '.result.sequence' > /dev/null 2>&1; then
    ledger=$(echo "$response" | jq '.result.sequence')
    echo "✅ OK (ledger: $ledger)"
  else
    echo "❌ FAILED"
    echo "Response: $response"
  fi
done
```

**Run the health check:**

```bash
chmod +x docs/scripts/test-rpc-health.sh
./docs/scripts/test-rpc-health.sh
```

**Expected output on success:**
```
=== Testing Mainnet RPC Providers ===
Testing https://soroban-mainnet.stellar.org ... ✅ OK (ledger: 123456)
Testing https://rpc.soroban.io ... ✅ OK (ledger: 123456)

=== Testing Testnet RPC Providers ===
Testing https://soroban-testnet.stellar.org ... ✅ OK (ledger: 987654)
Testing https://rpc-testnet.soroban.io ... ✅ OK (ledger: 987654)
```

---

## Failover Procedure Quick Reference

When the primary RPC provider is unavailable:

1. **Confirm outage:** Run `test-rpc-health.sh` to identify which providers are down
2. **Switch environment variables:**
   ```bash
   export SOROBAN_RPC_URL=https://rpc.soroban.io  # backup
   ```
3. **Restart indexer and API services** to pick up the new endpoint
4. **Verify operations succeed** on the new endpoint
5. **Document in incident log** which provider failed and when failover occurred

For full details, see [Scenario 2: RPC Provider Outage](./disaster-recovery.md#scenario-2-rpc-provider-outage) in the disaster recovery guide.

---

## SLA & Monitoring

**Target:** 99.9% uptime across all primary RPC providers  
**Monitoring:** Health checks run every 5 minutes (Prometheus + Alertmanager)

| Alert | Threshold | Action |
|-------|-----------|--------|
| RPC unavailable | 3 consecutive failed checks (15 min) | Page on-call engineer |
| RPC latency high | p99 > 5 seconds | Investigate and potentially switch provider |
| Failover activated | Any provider outage | Notify #trustlink-ops Slack channel |

See [docs/monitoring.md](./monitoring.md) for full monitoring setup.

---

## Adding a New RPC Provider

1. Test the provider's `/soroban/rpc` endpoint with `getLatestLedger`
2. Verify it can deploy and invoke contracts (use testnet first)
3. Add to this list in the appropriate section (Priority 1, 2, or Deprecated)
4. Update environment variable documentation in `docs/deployment.md`
5. Run health check script to confirm availability
6. Update monitoring system (Prometheus scrape config)

---

## Contacts

For issues with specific RPC providers:

| Provider | Support Contact | Response Time |
|----------|-----------------|----------------|
| Stellar Foundation | https://stellar.org/developers/support | 24–48 hours |
| SDF Public RPC | https://github.com/stellar/soroban-rpc/issues | Community-driven |
| Private RPC Node | [DevOps Lead Name] | On-call |

---

*Reference: [docs/disaster-recovery.md — Scenario 2](./disaster-recovery.md#scenario-2-rpc-provider-outage)*
