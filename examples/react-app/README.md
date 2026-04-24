# TrustLink React dApp

Reference implementation for the TrustLink attestation contract on Stellar testnet.

## Panels

| Panel | Who uses it | What it does |
|---|---|---|
| My Attestations | Any user | View all attestations issued to your address |
| Issuer | Registered issuers | Create and revoke attestations |
| Verifier | Anyone | Check if an address holds a valid claim |
| Admin | Contract admin | Register and remove issuers |

## Prerequisites

- [Freighter wallet](https://freighter.app) browser extension
- A Stellar testnet account funded via [Friendbot](https://friendbot.stellar.org)
- A deployed TrustLink contract ID

## Run locally

```bash
cp .env.example .env
# fill in VITE_CONTRACT_ID with your deployed contract address

npm install
npm run dev
```

Open `http://localhost:5173`, connect Freighter, and switch to testnet inside the extension.

## Deploy to GitHub Pages

The app deploys automatically via GitHub Actions on every push to `main` that touches `examples/react-app/`.

Set these repository secrets before the first deploy:

| Secret | Value |
|---|---|
| `VITE_CONTRACT_ID` | Your deployed TrustLink contract address |
| `VITE_RPC_URL` | *(optional)* defaults to Stellar testnet RPC |

The deployed app will be available at:
```
https://<your-org>.github.io/TrustLink/
```

## Tech stack

- Vite + React 18 + TypeScript
- `@stellar/stellar-sdk` for contract interaction
- `@stellar/freighter-api` for wallet connection
