# Canary Deployment Strategy for Indexer

## Overview

Canary deployments enable safe, gradual rollouts of new indexer versions. Instead of deploying to 100% of traffic immediately, a canary starts at 5-10% traffic while monitoring error rates and latency. If metrics are healthy, traffic is gradually increased. If issues arise, the deployment is rolled back with minimal impact.

## Problem Statement

Previously, `.github/workflows/publish-indexer.yml` deployed directly to production (100% traffic). A regression in event processing logic would immediately affect all users:

```
Old Flow:
  Release Tag → Docker Build → Push → Deploy (100% traffic)
  
  If Bug: All indexing fails instantly; requires manual rollback
```

The new canary strategy mitigates this risk:

```
New Flow:
  Release Tag → Docker Build → Push → Deploy Canary (5%) → Monitor → Gradual Increase → Stable
  
  If Bug: Only 5% of traffic affected; easy rollback without impact
```

## Architecture

### Deployments

**Main Deployment** (Stable)
- Runs the currently stable version
- Scales to production capacity (e.g., 3 replicas)
- Handles 90-95% of traffic initially

**Canary Deployment** (Candidate)
- Runs the new version
- Smaller replica count (e.g., 1 replica)
- Handles 5-10% of traffic initially
- Isolated database or read-replica for safety

### Service Routing

Traffic is split via Kubernetes weighted load balancing:

```yaml
service:
  sessionAffinity: None
  endpoints:
    - main-deployment: weight=90%
    - canary-deployment: weight=10%
```

### Monitoring

Automated checks validate canary health:

| Metric | Threshold | Action |
|--------|-----------|--------|
| Error Rate | > 1% | Alert + Auto-Rollback |
| P99 Latency | > 500ms | Alert |
| Ledger Lag | > 100 blocks | Alert + Auto-Rollback |

## Deployment Workflow

### Step 1: Deploy Canary (5% Traffic)

```bash
# Enable canary in values
helm upgrade trustlink-indexer . \
  -f values.yaml \
  -f values-canary.yaml \
  --set canary.enabled=true \
  --set canary.weight=5 \
  --set canary.replicaCount=1
```

Output:
```
trustlink-indexer-main (3 replicas, 95% traffic)
trustlink-indexer-canary (1 replica, 5% traffic)
```

### Step 2: Monitor (5-10 minutes)

Watch metrics from Prometheus/Grafana:

```bash
# Error rate
sum(rate(indexer_errors_total[1m])) by (deployment)

# Latency (p99)
histogram_quantile(0.99, rate(indexer_request_duration_seconds_bucket[1m]))

# Ledger lag
max(indexer_ledger_lag_blocks)
```

**Healthy Canary**: No increase in errors, latency within baseline.

**Unhealthy Canary**: Error spikes or timeout.

### Step 3: Increase to 25% (If Healthy)

```bash
helm upgrade trustlink-indexer . \
  -f values.yaml \
  -f values-canary.yaml \
  --set canary.weight=25
```

### Step 4: Increase to 50% (If Healthy)

```bash
helm upgrade trustlink-indexer . \
  -f values.yaml \
  -f values-canary.yaml \
  --set canary.weight=50 \
  --set canary.replicaCount=2
```

### Step 5: Full Promotion (100%)

```bash
# Option A: Promote canary to stable
helm upgrade trustlink-indexer . \
  -f values.yaml \
  --set image.tag=v0.2.0 \
  --set replicaCount=3

# Option B: Keep canary disabled and set new stable version
helm upgrade trustlink-indexer . \
  -f values.yaml \
  --set canary.enabled=false \
  --set image.tag=v0.2.0
```

### Rollback (If Issues Detected)

Automatic (if monitoring threshold exceeded):
```bash
# Prometheus alert triggers automated rollback
kubectl set env deployment/trustlink-indexer-canary CANARY_WEIGHT=0
```

Manual:
```bash
helm upgrade trustlink-indexer . \
  -f values.yaml \
  -f values-canary.yaml \
  --set canary.enabled=false
```

## Configuration

### Canary Values

Edit `indexer/helm/values-canary.yaml`:

```yaml
canary:
  enabled: false           # Enable/disable canary
  weight: 5                # 0-100: traffic percentage
  replicaCount: 1          # Usually 1 for canary
  imageTag: v0.2.0-rc1     # Candidate version
  
main:
  replicaCount: 3          # Stable version replicas
  imageTag: v0.1.0         # Stable version

monitoring:
  errorRateThreshold: "0.01"      # 1%
  latencyP99Threshold: "500ms"    # 500ms
  ledgerLagThreshold: "100"       # blocks
  evaluationWindow: "5m"          # rolling window
```

### Database Strategy

**Option 1: Shared Database (Risky)**
- Both main and canary write to the same database
- Risk: Bug in canary corrupts data for all users
- Use only if canary is read-only

**Option 2: Read Replica (Recommended)**
- Canary reads from read-replica of primary database
- Main (stable) uses primary database for writes
- Risk: Minimal; canary can't corrupt production data

**Option 3: Separate Database (Safest)**
- Canary has completely isolated database
- Requires dual DB setup; higher cost
- Risk: None; completely isolated

Helm setup (Option 2):
```yaml
main:
  database: primary-db

canary:
  database: read-replica-db
  env:
    - name: DATABASE_URL
      valueFrom:
        secretKeyRef:
          name: trustlink-indexer-db-replica
```

## CI/CD Integration

### GitHub Actions Workflow

Add to `.github/workflows/publish-indexer-canary.yml`:

```yaml
name: Canary Deploy Indexer

on:
  release:
    types: [released]  # Full release (not pre-release)

jobs:
  canary-deploy:
    name: Deploy to canary (5% traffic)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Deploy canary
        run: |
          helm upgrade trustlink-indexer . \
            -f indexer/helm/values.yaml \
            -f indexer/helm/values-canary.yaml \
            --set canary.enabled=true \
            --set canary.weight=5 \
            --set canary.imageTag=${{ github.ref_name }}
      
      - name: Wait and monitor (5 min)
        run: sleep 300
      
      - name: Check metrics
        run: |
          ERROR_RATE=$(curl -s prometheus:9090/api/v1/query \
            --data-urlencode 'query=rate(indexer_errors[1m])' \
            | jq '.data.result[].value[1]')
          
          if (( $(echo "$ERROR_RATE > 0.01" | bc -l) )); then
            echo "Error rate too high: $ERROR_RATE"
            exit 1
          fi
      
      - name: Promote to 25%
        if: success()
        run: |
          helm upgrade trustlink-indexer . \
            --set canary.weight=25

  # Add more stages for 50%, 100% (in separate job or workflow)
```

## Metrics & Monitoring

### Prometheus Queries

```promql
# Error rate by deployment
sum(rate(indexer_errors_total[1m])) by (deployment_type)

# Request latency (p99)
histogram_quantile(0.99, rate(indexer_request_duration_seconds_bucket[1m]))

# Ledger lag in blocks
max(indexer_ledger_lag_blocks)

# Events indexed per minute
rate(indexer_events_indexed_total[1m])
```

### Grafana Dashboard

Create a dashboard with panels:

1. **Traffic Distribution** (Pie chart)
   - Main vs Canary traffic %

2. **Error Rates** (Time series)
   - Main errors
   - Canary errors
   - Threshold line at 1%

3. **Latency** (Heatmap)
   - p50, p95, p99 for main and canary

4. **Ledger Lag** (Gauge)
   - Current lag vs. threshold

5. **Events Indexed** (Counter)
   - Events processed by main and canary

### Alerts (Prometheus AlertManager)

```yaml
- alert: CanaryHighErrorRate
  expr: |
    sum(rate(indexer_errors_total{deployment_type="canary"}[1m]))
    > 0.01
  for: 2m
  annotations:
    summary: "Canary indexer error rate too high"
    action: "Rollback canary deployment"

- alert: CanaryHighLatency
  expr: |
    histogram_quantile(0.99, 
      rate(indexer_request_duration_seconds_bucket{deployment_type="canary"}[1m])
    ) > 0.5
  for: 2m
  annotations:
    summary: "Canary indexer latency degraded"
    action: "Review canary code for performance regressions"

- alert: CanaryLedgerLag
  expr: max(indexer_ledger_lag_blocks{deployment_type="canary"}) > 100
  for: 3m
  annotations:
    summary: "Canary indexer falling behind ledger"
    action: "Investigate indexing performance"
```

## Runbook: Promoting Canary to Stable

### Prerequisites
- Canary has been running at 5% for ≥ 5 minutes
- No alert fires (error_rate, latency, lag within thresholds)
- Team has reviewed the release notes

### Procedure

1. **Check health metrics**
   ```bash
   # Error rate should be < 0.01
   kubectl logs -f -l deployment-type=canary
   ```

2. **Increase to 25%**
   ```bash
   helm upgrade trustlink-indexer . \
     --set canary.weight=25
   ```

3. **Wait 5 minutes**, watch metrics

4. **If healthy, increase to 50%**
   ```bash
   helm upgrade trustlink-indexer . \
     --set canary.weight=50 \
     --set canary.replicaCount=2
   ```

5. **Wait 5 minutes**, verify no issues

6. **Promote to 100%** (option A: scale up stable, disable canary)
   ```bash
   # Update main to new version
   helm upgrade trustlink-indexer . \
     -f values.yaml \
     --set image.tag=<new-version>
   
   # Disable canary
   helm upgrade trustlink-indexer . \
     --set canary.enabled=false
   ```

### Rollback Procedure

If metrics degrade at any stage:

1. **Disable canary immediately**
   ```bash
   helm upgrade trustlink-indexer . \
     --set canary.weight=0 \
     --set canary.enabled=false
   ```

2. **Verify main is healthy** (allow 1-2 min for recovery)
   ```bash
   kubectl logs -f -l deployment-type=main
   ```

3. **Investigate issue**
   - Check release notes
   - Review code changes
   - Reproduce locally if possible

4. **Document lesson learned**
   - Update runbook if needed
   - Add test case to catch regression

## Best Practices

- **Always start at 5%** – Even if tests pass, there's always a 1% chance
- **Wait 5 minutes per stage** – Give time for issues to surface
- **Monitor all three metrics** – Error rate, latency, ledger lag
- **Use isolated database for canary** – Prevents data corruption
- **Tag canary images clearly** – E.g., `v0.2.0-rc1` for release candidates
- **Document every promotion** – Update release notes with timing and metrics
- **Have rollback ready** – Test rollback procedure before deploying
- **Notify team** – Announce canary deploy in #deployments Slack channel

## Troubleshooting

### Canary pods stuck in CrashLoopBackOff

```bash
kubectl logs -f deployment/trustlink-indexer-canary
```

Check for:
- Connection to database
- Invalid environment variables
- Schema migration failures

### Traffic not being routed to canary

```bash
# Check service endpoints
kubectl get endpoints trustlink-indexer

# Verify weight is set
kubectl get service trustlink-indexer -o yaml | grep weight
```

### Metrics not appearing in Prometheus

```bash
# Verify canary has metrics enabled
kubectl port-forward svc/trustlink-indexer-canary 9090:3000
curl http://localhost:9090/metrics
```

## References

- [Kubernetes Canary Deployments](https://kubernetes.io/docs/concepts/cluster-administration/manage-deployment/#canary-deployments)
- [Flagger: Automated Canary Analysis](https://flagger.app/)
- [Blue-Green vs Canary Deployments](https://martinfowler.com/bliki/BlueGreenDeployment.html)
- [Helm Templating Guide](https://helm.sh/docs/chart_template_guide/)
