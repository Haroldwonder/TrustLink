# Cross-SDK Conformance Suite

Runs the same scripted contract scenario through all three TrustLink client packages and
asserts they observe identical results:

- `sdk/typescript`
- `bindings/typescript`
- `bindings/python`

## Prerequisites

1. Local Stellar Quickstart: `docker compose up -d`
2. Built WASM: `cargo build --target wasm32-unknown-unknown --release`
3. Deployed contract: `./scripts/setup_local.sh`

## Run locally

```bash
cd conformance
npm install
pip install -e ../bindings/python
npm run test:local
```

## CI

The `conformance` job in `.github/workflows/ci.yml` starts Quickstart, deploys the contract,
and runs this suite automatically.

## Scenario covered

- `has_valid_claim` for an existing claim type (true)
- `has_valid_claim` for a missing claim type (false)
- `get_subject_attestations` pagination (normalized attestation IDs)
- `get_attestation` on a missing ID (normalized contract error code/name)

Any divergence between clients fails the suite immediately.
