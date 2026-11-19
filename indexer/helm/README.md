# TrustLink Indexer Helm Chart

Deploys the TrustLink event indexer on Kubernetes. The chart mirrors the configuration used in `docker-compose.yml`: the indexer connects to an external PostgreSQL database and a Soroban RPC endpoint, then exposes REST and GraphQL/WS APIs.

## Prerequisites

- Kubernetes cluster (1.24+)
- [Helm](https://helm.sh/docs/intro/install/) 3.x
- PostgreSQL database reachable from the cluster (the chart does not deploy Postgres)
- Docker image for the indexer (built from `indexer/Dockerfile` or pulled from GHCR on release)

## Configuration

| Value           | Environment variable | Description                                     | Default                               |
| --------------- | -------------------- | ----------------------------------------------- | ------------------------------------- |
| `rpcUrl`        | `RPC_URL`            | Soroban RPC endpoint                            | `https://soroban-testnet.stellar.org` |
| `databaseUrl`   | `DATABASE_URL`       | PostgreSQL connection string (stored in Secret) | — (required)                          |
| `port`          | `PORT`               | REST API port                                   | `3000`                                |
| `gqlPort`       | `GQL_PORT`           | GraphQL HTTP/WS API port                        | `4000`                                |
| `contractId`    | `CONTRACT_ID`        | Deployed TrustLink contract ID                  | — (required)                          |
| `genesisLedger` | `GENESIS_LEDGER`     | First ledger to index                           | `0`                                   |

Additional values control the container image, replica count, service type, and probes. See `values.yaml` for the full list.

### Deployment Modes

The chart supports three deployment modes:

1. **Standard Deployment** (default): Creates a single Deployment named `{{ fullname }}` with `replicaCount` replicas. The Service routes to pods labeled with `deployment-type: standard`.

2. **Main Deployment**: When `main.enabled=true`, creates a "main" Deployment for stable production traffic. The standard deployment is not created when this is enabled. The Service routes to pods labeled with `deployment-type: main`. This is useful for production configurations where you want a dedicated main deployment instead of the standard one.

3. **Canary Deployment**: When `canary.enabled=true`, creates a canary Deployment for gradual rollout testing. This can be used alongside either the standard or main deployment. You would typically set up a separate Service for canary traffic to split traffic between deployments.

By default, only the standard deployment is created and the Service routes to it. Enable the main or canary deployments explicitly if you need them.

## Install

From the repository root:

```bash
helm install trustlink-indexer ./indexer/helm \
  --namespace trustlink \
  --create-namespace \
  --set contractId=YOUR_CONTRACT_ID \
  --set databaseUrl='postgresql://user:pass@postgres-host:5432/trustlink'
```

Or provide a custom values file:

```yaml
# my-values.yaml
rpcUrl: https://soroban-testnet.stellar.org
databaseUrl: postgresql://trustlink:secret@postgres.example.com:5432/trustlink
port: 3000
gqlPort: 4000
contractId: CAK7PYYSWWQH6ML3ZPO4OB2EIONODOEESE3MIV3YGFDMHEU4EUOBUJQN
genesisLedger: "0"

image:
  repository: ghcr.io/haroldwonder/trustlink/indexer
  tag: "1.0.0"
```

```bash
helm install trustlink-indexer ./indexer/helm \
  --namespace trustlink \
  --create-namespace \
  -f my-values.yaml
```

### Using the Main Deployment

To use the main deployment instead of the standard deployment:

```yaml
# my-values-with-main.yaml
main:
  enabled: true
  replicaCount: 3
  imageTag: "1.0.0"  # Use a specific stable version

rpcUrl: https://soroban-testnet.stellar.org
databaseUrl: postgresql://trustlink:secret@postgres.example.com:5432/trustlink
port: 3000
gqlPort: 4000
contractId: CAK7PYYSWWQH6ML3ZPO4OB2EIONODOEESE3MIV3YGFDMHEU4EUOBUJQN
genesisLedger: "0"

image:
  repository: ghcr.io/haroldwonder/trustlink/indexer
  tag: "1.0.0"
```

This will create the main deployment with 3 replicas instead of the standard deployment. The Service will route traffic to the main deployment. Note that when `main.enabled=true`, the standard deployment is not created.

## Verify

```bash
kubectl get pods -n trustlink -l app.kubernetes.io/name=trustlink-indexer
kubectl port-forward svc/trustlink-indexer 3000:3000 4000:4000 -n trustlink
curl http://localhost:3000/health
curl http://localhost:4000/graphql -H 'Content-Type: application/json' -d '{"query":"{ __typename }"}'
```

## Upgrade

```bash
helm upgrade trustlink-indexer ./indexer/helm \
  --namespace trustlink \
  -f my-values.yaml
```

## Uninstall

```bash
helm uninstall trustlink-indexer --namespace trustlink
```

## Notes

- The container runs `prisma migrate deploy` on startup before starting the indexer, matching the Docker image behavior.
- `databaseUrl` is stored in a Kubernetes Secret; non-sensitive settings (`rpcUrl`, `port`, `contractId`, `genesisLedger`) are stored in a ConfigMap.
- PostgreSQL must be provisioned separately. For local development, use `docker compose up db` from the `indexer/` directory and point `databaseUrl` at the exposed port.
