#!/usr/bin/env bash
set -euo pipefail

NETWORK_NAME="${NETWORK_NAME:-local}"
RPC_URL="${RPC_URL:-http://localhost:8000/soroban/rpc}"
NETWORK_PASSPHRASE="${NETWORK_PASSPHRASE:-Standalone Network ; February 2017}"
ADMIN_IDENTITY="${ADMIN_IDENTITY:-local-admin}"
WASM_PATH="${WASM_PATH:-target/wasm32-unknown-unknown/release/trustlink.wasm}"
TTL_DAYS="${TTL_DAYS:-30}"
CONTRACT_ID_FILE="${CONTRACT_ID_FILE:-.local.contract-id}"

if ! command -v soroban >/dev/null 2>&1; then
  echo "soroban CLI not found. Install it with: cargo install --locked soroban-cli"
  exit 1
fi

echo "Ensuring local network config '${NETWORK_NAME}' exists..."
if ! soroban network ls | grep -q "^${NETWORK_NAME}$"; then
  soroban network add \
    --global "${NETWORK_NAME}" \
    --rpc-url "${RPC_URL}" \
    --network-passphrase "${NETWORK_PASSPHRASE}"
fi

echo "Ensuring identity '${ADMIN_IDENTITY}' exists..."
if ! soroban config identity ls | grep -q "^${ADMIN_IDENTITY}$"; then
  soroban config identity generate "${ADMIN_IDENTITY}"
fi

echo "Funding identity on local friendbot..."
soroban config identity fund "${ADMIN_IDENTITY}" \
  --network "${NETWORK_NAME}" >/dev/null

if [ ! -f "${WASM_PATH}" ]; then
  echo "WASM not found at '${WASM_PATH}'. Build first with: make build"
  exit 1
fi

echo "Deploying contract to local network..."
CONTRACT_ID="$(soroban contract deploy \
  --wasm "${WASM_PATH}" \
  --source "${ADMIN_IDENTITY}" \
  --network "${NETWORK_NAME}")"

echo "Resolving admin address..."
ADMIN_ADDRESS="$(soroban config identity address "${ADMIN_IDENTITY}")"

echo "Initializing contract ${CONTRACT_ID}..."
soroban contract invoke \
  --id "${CONTRACT_ID}" \
  --source "${ADMIN_IDENTITY}" \
  --network "${NETWORK_NAME}" \
  -- initialize \
  --admin "${ADMIN_ADDRESS}" \
  --ttl_days "${TTL_DAYS}" >/dev/null

echo "${CONTRACT_ID}" > "${CONTRACT_ID_FILE}"

echo "Local deploy complete."
echo "Contract ID: ${CONTRACT_ID}"
echo "Saved to: ${CONTRACT_ID_FILE}"
echo "RPC URL: ${RPC_URL}"
