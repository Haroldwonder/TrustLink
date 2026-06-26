# TrustLink Video Tutorial Script

**Target duration:** 10–15 minutes  
**Audience:** Developers new to Soroban and TrustLink  
**Format:** Screen recording with narration

---

## Intro (0:00 – 0:45)

> "Welcome. In this tutorial you'll learn what TrustLink is, how to deploy it to the Stellar testnet, and how to integrate it into your own smart contract or frontend app. Let's get started."

Show: TrustLink GitHub repo homepage.

---

## Section 1 — What is TrustLink? (0:45 – 2:30)

> "TrustLink is a Soroban smart contract that acts as a shared trust layer on the Stellar blockchain. Instead of every dApp building its own KYC or identity system, TrustLink lets trusted issuers — like anchors or fintech companies — create attestations about wallet addresses. Other contracts can then query those attestations before executing sensitive operations."

Show: README overview section, then the data model diagram.

Key points to cover:
- Issuers are admin-approved addresses that can create attestations
- An attestation links a subject address to a claim type (e.g. `KYC_PASSED`)
- Attestations can have optional expiration and can be revoked
- Any contract can call `has_valid_claim` to gate access

---

## Section 2 — Prerequisites (2:30 – 3:30)

> "Before we deploy, make sure you have these installed."

Show terminal, run each command:

```bash
# Rust
rustup --version

# wasm target
rustup target add wasm32-unknown-unknown

# Soroban CLI
soroban --version

# Stellar CLI (alternative)
stellar --version
```

> "You'll also need a funded testnet account. Grab one from Friendbot:"

```bash
curl "https://friendbot.stellar.org?addr=YOUR_PUBLIC_KEY"
```

---

## Section 3 — Clone and Build (3:30 – 5:00)

> "Let's clone the repo and build the contract."

Show terminal:

```bash
git clone https://github.com/unixfundz/TrustLink.git
cd TrustLink

# Run tests to confirm everything works
make test

# Build optimized wasm
make optimize
```

> "The optimized wasm lands in `target/wasm32-unknown-unknown/release/trustlink.wasm`. That's what we deploy."

---

## Section 4 — Deploy to Testnet (5:00 – 7:00)

> "Now let's deploy to testnet."

Show terminal:

```bash
# Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
  --network testnet \
  --source YOUR_SECRET_KEY
```

> "Copy the contract ID from the output — you'll need it for every subsequent call."

```bash
# Initialize with your admin address
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source YOUR_SECRET_KEY \
  -- initialize \
  --admin YOUR_PUBLIC_KEY
```

> "The contract is live. Let's register an issuer and create an attestation."

```bash
# Register an issuer
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source YOUR_SECRET_KEY \
  -- register_issuer \
  --issuer ISSUER_PUBLIC_KEY

# Create a KYC attestation
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source ISSUER_SECRET_KEY \
  -- create_attestation \
  --issuer ISSUER_PUBLIC_KEY \
  --subject SUBJECT_PUBLIC_KEY \
  --claim_type KYC_PASSED \
  --expiration null

# Verify the claim
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- has_valid_claim \
  --subject SUBJECT_PUBLIC_KEY \
  --claim_type KYC_PASSED
```

---

## Section 5 — Cross-Contract Integration (7:00 – 10:30)

> "Now the real power: using TrustLink from another contract."

Show editor with a minimal lending contract:

```rust
mod trustlink {
    soroban_sdk::contractimport!(
        file = "../trustlink/target/wasm32-unknown-unknown/release/trustlink.wasm"
    );
}

#[contractimpl]
impl LendingContract {
    pub fn borrow(
        env: Env,
        borrower: Address,
        trustlink_id: Address,
        amount: i128,
    ) -> Result<(), Error> {
        borrower.require_auth();

        let trustlink = trustlink::Client::new(&env, &trustlink_id);
        let claim = String::from_str(&env, "KYC_PASSED");

        if !trustlink.has_valid_claim(&borrower, &claim) {
            return Err(Error::KYCRequired);
        }

        // lending logic here
        Ok(())
    }
}
```

> "Three lines is all it takes to gate a function behind a KYC check. The `contractimport!` macro generates a typed client from the wasm, so you get compile-time safety."

Show: running `cargo test` with a test that mocks TrustLink.

---

## Section 5.5 — Multi-Sig Attestations (10:30 – 12:00)

> "For high-value claims like `ACCREDITED_INVESTOR`, TrustLink supports M-of-N multi-sig attestations. Instead of one issuer acting alone, multiple registered issuers must co-sign before the attestation becomes active."

Show terminal — propose a 2-of-3 attestation:

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source ISSUER_A_SECRET \
  -- propose_attestation \
  --proposer ISSUER_A_PUBLIC_KEY \
  --subject SUBJECT_PUBLIC_KEY \
  --claim_type ACCREDITED_INVESTOR \
  --required_signers '[{"address":"ISSUER_A_PUBLIC_KEY"},{"address":"ISSUER_B_PUBLIC_KEY"},{"address":"ISSUER_C_PUBLIC_KEY"}]' \
  --threshold 2
```

> "The proposer counts as the first signer. Copy the proposal ID from the output."

```bash
# issuer_b co-signs — threshold of 2 is now reached
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source ISSUER_B_SECRET \
  -- cosign_attestation \
  --issuer ISSUER_B_PUBLIC_KEY \
  --proposal_id <PROPOSAL_ID>
```

> "Once the threshold is reached the attestation is automatically finalized and stored. Let's verify."

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- has_valid_claim \
  --subject SUBJECT_PUBLIC_KEY \
  --claim_type ACCREDITED_INVESTOR
```

Show output: `true`

> "You can also inspect the proposal to see who has signed and whether it's been finalized."

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_multisig_proposal \
  --proposal_id <PROPOSAL_ID>
```

> "Proposals expire after 7 days if the threshold isn't reached — protecting against stale partial approvals."

Show the equivalent Rust snippet:

```rust
let mut required_signers = soroban_sdk::Vec::new(&env);
required_signers.push_back(issuer_a.clone());
required_signers.push_back(issuer_b.clone());
required_signers.push_back(issuer_c.clone());

let proposal_id = contract.propose_attestation(
    &issuer_a,
    &user_address,
    &String::from_str(&env, "ACCREDITED_INVESTOR"),
    &required_signers,
    &2, // threshold
);

contract.cosign_attestation(&issuer_b, &proposal_id);
assert!(contract.has_valid_claim(&user_address, &String::from_str(&env, "ACCREDITED_INVESTOR")));
```

---

## Section 6 — JavaScript / TypeScript Integration (12:00 – 14:00)

> "If you're building a frontend, here's how to check a claim with the Stellar SDK."

Show editor with the TypeScript snippet from the integration guide:

```typescript
import { Contract, Networks, TransactionBuilder, SorobanRpc, nativeToScVal, scValToNative } from "@stellar/stellar-sdk";

const server = new SorobanRpc.Server("https://soroban-testnet.stellar.org");
const contract = new Contract("<CONTRACT_ID>");

async function hasValidClaim(subject: string, claimType: string): Promise<boolean> {
  const op = contract.call(
    "has_valid_claim",
    nativeToScVal(subject, { type: "address" }),
    nativeToScVal(claimType, { type: "string" })
  );
  const account = await server.getAccount(subject);
  const tx = new TransactionBuilder(account, { fee: "100", networkPassphrase: Networks.TESTNET })
    .addOperation(op).setTimeout(30).build();
  const sim = await server.simulateTransaction(tx);
  return scValToNative(sim.result?.retval);
}
```

> "Simulate the transaction — no signing needed for read-only calls. The result comes back as a native JS boolean."

---

## Outro (14:00 – 15:00)

> "That's TrustLink end to end: deploy, issue attestations, and verify them from both Rust contracts and TypeScript frontends."

> "For the full API reference, check the README. For deeper integration patterns including error handling and pagination, see the integration guide linked in the description."

> "If you run into issues, open a GitHub issue. Thanks for watching."

Show: links to README, integration guide, and GitHub issues page.

---

## Recording Checklist

- [ ] Terminal font size ≥ 18pt for readability
- [ ] Hide sensitive keys — use placeholder values on screen
- [ ] Pause 2 seconds after each command before showing output
- [ ] Add captions / subtitles to the final upload
- [ ] Set video visibility to Public on YouTube before linking
