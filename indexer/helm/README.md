# TrustLink Indexer Helm Chart

Deploys the TrustLink event indexer on Kubernetes. The chart mirrors the configuration used in `docker-compose.yml`: the indexer connects to an external PostgreSQL database and a Soroban RPC endpoint, then exposes a REST API.

## Prerequisites

- Kubernetes cluster (1.24+)
- [Helm](https://helm.sh/docs/intro/install/) 3.x
- PostgreSQL database reachable from the cluster (the chart does not deploy Postgres)
- Docker image for the indexer (built from `indexer/Dockerfile` or pulled from GHCR on release)

## Configuration

| Value | Environment variable | Description | Default |
|---|---|---|---|
| `rpcUrl` | `RPC_URL` | Soroban RPC endpoint | `https://soroban-testnet.stellar.org` |
| `databaseUrl` | `DATABASE_URL` | PostgreSQL connection string (stored in Secret) | — (required) |
| `port` | `PORT` | REST API port | `3000` |
| `contractId` | `CONTRACT_ID` | Deployed TrustLink contract ID | — (required) |
| `genesisLedger` | `GENESIS_LEDGER` | First ledger to index | `0` |

Additional values control the container image, replica count, service type, and probes. See `values.yaml` for the full list.

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
contractId: CAK7PYYSWWQH6ML3ZPO4OB2EIONODOEESE3MIV3YGFDMHEU4EUOBUJQN
genesisLedger: "0"

image:
  repository: ghcr.io/od-hunter/trustlink/indexer
  tag: "1.0.0"
```

```bash
helm install trustlink-indexer ./indexer/helm \
  --namespace trustlink \
  --create-namespace \
  -f my-values.yaml
```

## Verify

```bash
kubectl get pods -n trustlink -l app.kubernetes.io/name=trustlink-indexer
kubectl port-forward svc/trustlink-indexer 3000:3000 -n trustlink
curl http://localhost:3000/health
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
