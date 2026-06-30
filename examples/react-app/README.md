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

- A Stellar wallet browser extension — **Freighter** ([freighter.app](https://freighter.app)) or **xBull** ([xbull.app](https://xbull.app))
- A Stellar testnet account funded via [Friendbot](https://friendbot.stellar.org)
- A deployed TrustLink contract ID

## Run locally

```bash
cp .env.example .env
# fill in VITE_CONTRACT_ID with your deployed contract address

npm install
npm run dev
```

Open `http://localhost:5173`, choose a wallet, and switch to testnet inside the extension.

## Wallet support

The app uses [Stellar Wallets Kit](https://github.com/Creit-Tech/stellar-wallets-kit) to support multiple Stellar wallets from a single connection UI. Currently supported:

| Wallet | Notes |
|--------|-------|
| **Freighter** | Full support including network-mismatch detection |
| **xBull** | Full support; network detection defers to the configured testnet |

Adding more wallets (e.g. LOBSTR, Rabet) only requires importing the relevant module from `@creit.tech/stellar-wallets-kit` and appending it to the `modules` array in `src/wallet.ts`.

## Internationalization (i18n)

The app ships with English (`en`) and Spanish (`es`) locales using [react-i18next](https://react.i18next.com/).

- The language is **auto-detected** from `navigator.language` on first load.
- Users can **switch language** at any time with the `EN / ES` button in the header.
- Translation files live in `src/locales/en.json` and `src/locales/es.json`.

To add a new language:

1. Copy `src/locales/en.json` to `src/locales/<locale>.json` and translate the values.
2. Import it in `src/i18n.ts` and add it to the `resources` map.
3. Add the locale code to `SUPPORTED_LANGS` in `src/App.tsx`.

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
- `@creit.tech/stellar-wallets-kit` for multi-wallet connection (Freighter, xBull, …)
- `i18next` + `react-i18next` for internationalization
