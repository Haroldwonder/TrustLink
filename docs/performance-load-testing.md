# Indexer GraphQL API Load Testing

This document describes the k6 load testing suite for the TrustLink GraphQL API.

## Prerequisites

Install k6:

```bash
# macOS
brew install k6

# Linux (Ubuntu)
sudo apt-get install k6

# Docker
docker run --rm -it -v $PWD:/scripts grafana/k6:latest run /scripts/k6-load-test.js
```

For detailed installation instructions, see [k6 Getting Started](https://k6.io/docs/getting-started/installation/).

## Running the Load Tests

### Full Load Test

Runs a 5.5-minute scenario with ramping concurrency (10 → 50 → 100 VUs):

```bash
cd indexer
npm run test:load
# or with custom base URL:
BASE_URL=http://api.example.com/graphql npm run test:load
```

### Smoke Test

Quick 10-second validation that the API is working:

```bash
npm run test:load:smoke
```

### Custom Execution

For more control, invoke k6 directly:

```bash
# Run with custom VU count and duration
k6 run --vus 50 --duration 1m k6-load-test.js

# Run in cloud (requires k6 Pro account)
k6 cloud k6-load-test.js

# Save results to JSON
k6 run --out json=results.json k6-load-test.js
```

## Test Scenarios

The load test exercises four representative GraphQL queries:

### 1. List Attestations (Paginated)

Queries 10 attestations at a time, useful for dashboard pagination:

```graphql
query ListAttestations($limit: Int!, $offset: Int!) {
  attestations(limit: $limit, offset: $offset) {
    id issuer subject claimType timestamp isRevoked metadata
  }
}
```

**Threshold:** p95 < 200ms, p99 < 500ms

---

### 2. Issuer Statistics

Retrieves aggregated stats for a single issuer:

```graphql
query IssuerStats($issuer: String!) {
  issuer(address: $issuer) {
    address totalIssued active revoked
  }
}
```

**Threshold:** p95 < 200ms, p99 < 500ms

---

### 3. Subject Claims

Lists claim types held by a subject with validity status:

```graphql
query SubjectClaims($subject: String!) {
  subject(address: $subject) {
    address
    claims { claimType count hasValid }
  }
}
```

**Threshold:** p95 < 200ms, p99 < 500ms

---

### 4. Attestations in Date Range

Range queries for historical analysis:

```graphql
query AttestationsInRange($subject: String!, $from: Int!, $to: Int!, $limit: Int!) {
  attestationsInRange(subject: $subject, fromTimestamp: $from, toTimestamp: $to, limit: $limit) {
    id claimType timestamp isRevoked
  }
}
```

**Threshold:** p95 < 500ms, p99 < 1000ms

---

## Load Profile

The test follows a gradual ramp-up to detect performance degradation:

| Phase | Duration | VU Count | Purpose |
|-------|----------|----------|---------|
| Warm-up | 30s | 0→10 | Cold start behavior |
| Ramp-up | 1m | 10→50 | Linear scaling test |
| Peak | 1m30s | 50→100 | High concurrency |
| Sustained | 2m | 100 | Production load simulation |
| Cool-down | 30s | 100→0 | Graceful shutdown |

**Total duration:** 5m 30s

## Expected Results

Baseline results from a development machine with a local indexer and PostgreSQL:

| Metric | Value | Notes |
|--------|-------|-------|
| Requests/sec (at peak) | 150–300 | Depends on hardware |
| p95 latency | 100–300ms | Most queries return quickly |
| p99 latency | 300–800ms | Range queries slower |
| Error rate | < 5% | Some query errors at peak load |
| Requests total | ~25,000 | Across full 5.5m test |

## Interpreting Results

### HTTP Metrics

```
http_req_duration
  - p95 < 200ms ✓
  - p99 < 1000ms ✓
  - max < 5000ms ?   (investigate if exceeded)

http_req_failed
  - rate < 5% ✓
  - rate > 10% ? (suggests API degradation)
```

### Performance Zones

| Category | p95 | Recommendation |
|----------|-----|-----------------|
| ✅ Excellent | < 100ms | Increase load, add features |
| ✅ Good | 100–300ms | Monitor, plan upgrade before 2x users |
| ⚠️ Fair | 300–800ms | Add caching, consider read replicas |
| ❌ Poor | > 800ms | Scale horizontally or optimize queries |

## Optimizations Tested

The load test assumes the following optimizations are in place:

- ✓ PostgreSQL indexes on `issuer`, `subject`, `claimType`
- ✓ GraphQL query batching / dataloader if used
- ✓ Connection pooling (Prisma)
- ✓ Indexer reindex runs at least hourly

If performance degrades, check:

1. **Database indexes** — Run `EXPLAIN ANALYZE` on slow queries
2. **Prisma client pool** — Ensure connection limit is high enough
3. **Indexer lag** — If reindex is behind, queries hit stale data
4. **Cache hit rate** — Monitor Redis or in-memory cache effectiveness

## Production Sizing

Based on load test results, use the following rule of thumb:

| Expected QPS | Recommended Resources | Notes |
|--------------|------------------------|-------|
| 100 | 1 vCPU, 2GB RAM | Development |
| 500 | 2 vCPU, 4GB RAM | Small production |
| 2,000 | 4 vCPU, 8GB RAM | Medium production |
| 5,000+ | Horizontal scaling + read replicas | Large production |

## Continuous Performance Monitoring

To track performance over time:

```bash
# Save results to S3 or artifact store
npm run test:load -- --out json=test-$(date +%s).json

# Parse results in CI/CD
cat results.json | jq '.metrics | keys[]'

# Alert if performance regresses
if [ "$(jq '.metrics.http_req_duration.values.p95' results.json)" -gt 500 ]; then
  echo "Performance regression detected!"
  exit 1
fi
```

## Troubleshooting

### "Too many open files" error

Increase OS file limits:

```bash
ulimit -n 65536
npm run test:load
```

### Port already in use

The test defaults to `localhost:4000/graphql`. If indexer runs on a different port:

```bash
BASE_URL=http://localhost:5000/graphql npm run test:load
```

### All requests failing

Ensure indexer is running:

```bash
npm run dev

# In another terminal:
npm run test:load
```

### Timeout after connecting

The test expects responses within 30s. If queries are very slow:

```bash
k6 run --timeout 60s k6-load-test.js
```

## Further Reading

- [k6 Documentation](https://k6.io/docs/)
- [Grafana Cloud k6](https://grafana.com/products/cloud/k6/)
- [Performance Testing Best Practices](https://k6.io/blog/performance-testing-best-practices/)
