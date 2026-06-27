# 5-Minute Quickstart

Get from zero to verifying a testnet attestation using only the TypeScript SDK.

**What you'll do:**
1. Fund a testnet account
2. Register an issuer and create an attestation
3. Verify the claim on-chain

No Rust, no contract deployment — TrustLink is already deployed on testnet.

---

## Prerequisites

- Node.js 18+ and npm
- A terminal

---

## Step 1 — Generate a testnet keypair

Install the Stellar CLI and generate two keypairs: one for the admin/issuer, one for the subject.

```bash
cargo install --locked stellar-cli --features opt
stellar keys generate issuer --network testnet
stellar keys generate subject --network testnet
stellar keys address issuer
stellar keys address subject
```

Save both addresses — you'll use them in the steps below.

---

## Step 2 — Fund both accounts via Friendbot

Testnet accounts need a small XLM balance before they can submit transactions.

```bash
curl "https://friendbot.stellar.org?addr=$(stellar keys address issuer)"
curl "https://friendbot.stellar.org?addr=$(stellar keys address subject)"
```

Both should return a JSON response with `"successful": true`.

---

## Step 3 — Bootstrap a Node project

```bash
mkdir trustlink-quickstart
cd trustlink-quickstart
npm init -y
npm install @stellar/stellar-sdk
```

---

## Step 4 — Create `quickstart.mjs`

Create the file below. Replace `ISSUER_SECRET` and `SUBJECT_ADDRESS` with the
values from Step 1.

```js
// quickstart.mjs
import {
  Keypair,
  Contract,
  TransactionBuilder,
  SorobanRpc,
  Networks,
  nativeToScVal,
  scValToNative,
  xdr,
} from "@stellar/stellar-sdk";

// ── Config ──────────────────────────────────────────────────────────────────
const RPC_URL    = "https://soroban-testnet.stellar.org";
const CONTRACT   = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8";
const PASSPHRASE = Networks.TESTNET;

const ISSUER_SECRET  = "SXXX...";          // stellar keys show issuer --secret
const SUBJECT_ADDRESS = "GYYY...";         // stellar keys address subject

// ── Helpers ──────────────────────────────────────────────────────────────────
const server  = new SorobanRpc.Server(RPC_URL);
const issuer  = Keypair.fromSecret(ISSUER_SECRET);
const contract = new Contract(CONTRACT);

async function simulate(publicKey, operation) {
  const account = await server.getAccount(publicKey);
  const tx = new TransactionBuilder(account, {
    fee: "100",
    networkPassphrase: PASSPHRASE,
  })
    .addOperation(operation)
    .setTimeout(30)
    .build();
  return server.simulateTransaction(tx);
}

async function invoke(keypair, operation) {
  const account = await server.getAccount(keypair.publicKey());
  let tx = new TransactionBuilder(account, {
    fee: "100",
    networkPassphrase: PASSPHRASE,
  })
    .addOperation(operation)
    .setTimeout(30)
    .build();

  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) throw new Error(sim.error);
  tx = SorobanRpc.assembleTransaction(tx, sim).build();
  tx.sign(keypair);

  const send = await server.sendTransaction(tx);
  if (send.status === "ERROR") throw new Error(JSON.stringify(send.errorResult));

  // Poll until confirmed
  let result;
  do {
    await new Promise((r) => setTimeout(r, 1500));
    result = await server.getTransaction(send.hash);
  } while (result.status === SorobanRpc.Api.GetTransactionStatus.NOT_FOUND);

  if (result.status === SorobanRpc.Api.GetTransactionStatus.FAILED)
    throw new Error("Transaction failed on-chain");

  return result.returnValue ? scValToNative(result.returnValue) : null;
}

// ── 1. Register the issuer ────────────────────────────────────────────────────
console.log("Registering issuer…");
// NOTE: on the shared testnet contract the admin key is controlled by the
// TrustLink project.  For a private deploy, use your own admin secret here.
// This step is skipped against the shared testnet; jump straight to step 2.

// ── 2. Create a KYC_PASSED attestation ───────────────────────────────────────
console.log("Creating attestation…");
const attestationId = await invoke(issuer, contract.call(
  "create_attestation",
  nativeToScVal(issuer.publicKey(), { type: "address" }),
  nativeToScVal(SUBJECT_ADDRESS,    { type: "address" }),
  nativeToScVal("KYC_PASSED",       { type: "string"  }),
  xdr.ScVal.scvVoid(),  // no expiration
  xdr.ScVal.scvVoid(),  // no metadata
));
console.log("Attestation ID:", attestationId);

// ── 3. Verify the claim ───────────────────────────────────────────────────────
console.log("Verifying claim…");
const sim = await simulate(issuer.publicKey(), contract.call(
  "has_valid_claim",
  nativeToScVal(SUBJECT_ADDRESS, { type: "address" }),
  nativeToScVal("KYC_PASSED",    { type: "string"  }),
));
if (SorobanRpc.Api.isSimulationError(sim)) throw new Error(sim.error);
const valid = scValToNative(sim.result.retval);

console.log(`\n✓ has_valid_claim("KYC_PASSED") → ${valid}`);
console.log("Done — subject is verified on testnet.");
```

---

## Step 5 — Run it

```bash
node quickstart.mjs
```

Expected output:

```
Creating attestation…
Attestation ID: <hex string>
Verifying claim…

✓ has_valid_claim("KYC_PASSED") → true
Done — subject is verified on testnet.
```

That's it. You've issued and verified your first attestation on Stellar testnet.

---

## What just happened

| Step | Contract call | What it does |
|------|--------------|--------------|
| Create | `create_attestation` | Writes a `KYC_PASSED` claim from issuer → subject |
| Verify | `has_valid_claim` | Returns `true` if any non-revoked, non-expired attestation exists |

Both calls go through [Soroban RPC simulation + assembly](https://developers.stellar.org/docs/build/guides/transactions/simulatetransaction-deep-dive) — the standard pattern for Stellar smart contract transactions.

---

## Next steps

- **Add expiration:** Pass a Unix timestamp instead of `xdr.ScVal.scvVoid()` to auto-expire credentials.
- **Require multiple claims:** Use `has_all_claims` to gate access behind KYC + AML clearance simultaneously.
- **Rust cross-contract calls:** See [docs/integration-guide.md](integration-guide.md) for integrating TrustLink into your own Soroban contract.
- **Example contracts:** Browse [`examples/`](../examples/) for KYC tokens, DAO governance, insurance, real-estate title, and healthcare credential patterns.
- **Revoke a credential:** Call `revoke_attestation(issuer, attestation_id)` at any time to invalidate the claim.
- **Private deploy:** Run `make deploy NETWORK=testnet` from the repo root to spin up your own instance with your own admin key.

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `account not found` | Re-run the Friendbot curl for that address |
| `Unauthorized` | The issuer must be registered by the contract admin before calling `create_attestation` on the shared testnet instance |
| `Transaction failed on-chain` | Check the Stellar testnet explorer at [stellar.expert](https://stellar.expert/explorer/testnet) with your transaction hash |
| `Simulation failed` | Ensure Node.js ≥ 18 and `@stellar/stellar-sdk` is installed |
