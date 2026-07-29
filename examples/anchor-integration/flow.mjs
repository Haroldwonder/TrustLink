import {
  Contract,
  Networks,
  SorobanRpc,
  TransactionBuilder,
  Keypair,
  nativeToScVal,
  scValToNative,
  Address,
} from "@stellar/stellar-sdk";
import { parseTrustLinkError } from "./errors.mjs";

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------
const cfg = {
  rpcUrl: process.env.RPC_URL || "https://soroban-testnet.stellar.org",
  networkPassphrase: process.env.NETWORK_PASSPHRASE || Networks.TESTNET,
  trustlinkContractId: process.env.TRUSTLINK_CONTRACT_ID || "",
  anchorSecret: process.env.ANCHOR_SECRET || "",
  userAddress: process.env.USER_ADDRESS || "",
  defiCallerSecret: process.env.DEFI_CALLER_SECRET || "",
};

function required(value, name) {
  if (!value) {
    console.error(`Error: Missing ${name}. Set it in environment variables.`);
    process.exit(1);
  }
}

// ---------------------------------------------------------------------------
// RPC helpers
// ---------------------------------------------------------------------------
async function simulateRead(server, sourceAddress, operation, networkPassphrase) {
  const account = await server.getAccount(sourceAddress);
  const tx = new TransactionBuilder(account, { fee: "100", networkPassphrase })
    .addOperation(operation)
    .setTimeout(30)
    .build();

  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) {
    throw new Error(`Simulation failed: ${sim.error}`);
  }
  return sim.result?.retval;
}

async function submitWrite(server, sourceKeypair, operation, networkPassphrase) {
  const account = await server.getAccount(sourceKeypair.publicKey());
  let tx = new TransactionBuilder(account, { fee: "1000000", networkPassphrase })
    .addOperation(operation)
    .setTimeout(60)
    .build();

  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) {
    throw new Error(`Write simulation failed: ${sim.error}`);
  }

  tx = SorobanRpc.assembleTransaction(tx, sim, networkPassphrase);
  tx.sign(sourceKeypair);

  const sent = await server.sendTransaction(tx);
  if (sent.status === "ERROR") {
    throw new Error(`Transaction failed: ${sent.errorResultXdr || "unknown"}`);
  }

  const hash = sent.hash;
  while (true) {
    const res = await server.getTransaction(hash);
    if (res.status === "SUCCESS") return res;
    if (res.status === "FAILED") throw new Error("Transaction status FAILED");
    await new Promise((resolve) => setTimeout(resolve, 1200));
  }
}

// ---------------------------------------------------------------------------
// Main flow
// ---------------------------------------------------------------------------
async function main() {
  required(cfg.trustlinkContractId, "TRUSTLINK_CONTRACT_ID");
  required(cfg.anchorSecret, "ANCHOR_SECRET");
  required(cfg.userAddress, "USER_ADDRESS");
  required(cfg.defiCallerSecret, "DEFI_CALLER_SECRET");

  const server = new SorobanRpc.Server(cfg.rpcUrl);
  const contract = new Contract(cfg.trustlinkContractId);

  const anchor = Keypair.fromSecret(cfg.anchorSecret);
  const defiCaller = Keypair.fromSecret(cfg.defiCallerSecret);
  const claimType = "KYC_PASSED";

  console.log("\n=== ANCHOR INTEGRATION FLOW ===");

  // ── Step 1: Check issuer registration ──────────────────────────────────
  console.log("\n1) Anchor issuer registration check");
  let isIssuer;
  try {
    const isIssuerOp = contract.call(
      "is_issuer",
      nativeToScVal(Address.fromString(anchor.publicKey()), { type: "address" })
    );
    const issuerRet = await simulateRead(
      server,
      anchor.publicKey(),
      isIssuerOp,
      cfg.networkPassphrase
    );
    isIssuer = issuerRet ? scValToNative(issuerRet) : false;
  } catch (err) {
    console.error("✗ Failed to check issuer status:", parseTrustLinkError(err));
    process.exit(1);
  }

  console.log("✓ Anchor registered as issuer:", isIssuer);
  if (!isIssuer) {
    console.error(
      "✗ Anchor is not a registered issuer. Run register_issuer(admin, anchorAddress) first."
    );
    process.exit(1);
  }

  // ── Step 2: Off-chain KYC ───────────────────────────────────────────────
  console.log("\n2) User completes KYC off-chain");
  console.log("✓ User submitted KYC documents");
  console.log("✓ Anchor verified identity and compliance");

  // ── Step 3: Issue attestation ───────────────────────────────────────────
  console.log("\n3) Anchor issues KYC_PASSED attestation");
  const expiration = Math.floor(Date.now() / 1000) + 180 * 24 * 60 * 60;
  const metadata = JSON.stringify({
    provider: "Example Anchor",
    level: "basic",
    checked_at: new Date().toISOString(),
  });

  const createOp = contract.call(
    "create_attestation",
    nativeToScVal(Address.fromString(anchor.publicKey()), { type: "address" }),
    nativeToScVal(Address.fromString(cfg.userAddress), { type: "address" }),
    nativeToScVal(claimType, { type: "string" }),
    nativeToScVal(expiration, { type: "u64" }),
    nativeToScVal(metadata, { type: "string" })
  );

  let attestationId;
  try {
    const writeRes = await submitWrite(server, anchor, createOp, cfg.networkPassphrase);
    attestationId = writeRes.returnValue ? scValToNative(writeRes.returnValue) : null;
    console.log("✓ Created attestation id:", attestationId);
    console.log("✓ Attestation expires at:", new Date(expiration * 1000).toISOString());
  } catch (err) {
    const human = parseTrustLinkError(err);
    // DuplicateAttestation (#5) is recoverable — the attestation already exists.
    if (human.includes("DuplicateAttestation")) {
      console.log("⚠ Attestation already exists for this subject/claim — continuing.");
    } else {
      console.error("✗ Failed to create attestation:", human);
      process.exit(1);
    }
  }

  // ── Step 4: DeFi verification ───────────────────────────────────────────
  console.log("\n4) DeFi contract verifies attestation before allowing deposit");
  let verified;
  try {
    const verifyOp = contract.call(
      "has_valid_claim_from_issuer",
      nativeToScVal(Address.fromString(cfg.userAddress), { type: "address" }),
      nativeToScVal(claimType, { type: "string" }),
      nativeToScVal(Address.fromString(anchor.publicKey()), { type: "address" })
    );
    const verifiedRet = await simulateRead(
      server,
      defiCaller.publicKey(),
      verifyOp,
      cfg.networkPassphrase
    );
    verified = verifiedRet ? scValToNative(verifiedRet) : false;
  } catch (err) {
    console.error("✗ Failed to verify claim:", parseTrustLinkError(err));
    process.exit(1);
  }

  console.log("✓ DeFi verification result:", verified);
  if (verified) {
    console.log("✓ Action: ALLOW deposit - user has valid KYC attestation");
  } else {
    console.log("✗ Action: DENY deposit - KYC attestation not valid");
  }

  // ── Step 5: Attestation status ──────────────────────────────────────────
  console.log("\n5) Simulate KYC expiration scenario");
  if (!attestationId) {
    console.log("⚠ No attestation ID available (attestation pre-existed); skipping status check.");
    console.log("\n=== FLOW COMPLETE ===");
    return;
  }

  console.log("⏱ Checking attestation status...");
  let status;
  try {
    const statusOp = contract.call(
      "get_attestation_status",
      nativeToScVal(attestationId, { type: "string" })
    );
    const statusRet = await simulateRead(
      server,
      defiCaller.publicKey(),
      statusOp,
      cfg.networkPassphrase
    );
    status = statusRet ? scValToNative(statusRet) : null;
  } catch (err) {
    console.error("✗ Failed to fetch attestation status:", parseTrustLinkError(err));
    process.exit(1);
  }

  console.log("✓ Current attestation status:", status);
  if (status === "Valid") {
    console.log("✓ Attestation is currently valid");
    console.log("⏱ After expiration date, status will become 'Expired'");
    console.log("✗ DeFi contract will then DENY deposits until KYC is renewed");
  }

  console.log("\n=== FLOW COMPLETE ===");
}

main().catch((err) => {
  console.error("Unhandled error:", parseTrustLinkError(err));
  process.exit(1);
});
