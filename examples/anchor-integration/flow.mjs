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

const cfg = {
  rpcUrl: process.env.RPC_URL || "https://soroban-testnet.stellar.org",
  networkPassphrase:
    process.env.NETWORK_PASSPHRASE || Networks.TESTNET,
  trustlinkContractId: process.env.TRUSTLINK_CONTRACT_ID || "",
  anchorSecret: process.env.ANCHOR_SECRET || "",
  userAddress: process.env.USER_ADDRESS || "",
  defiCallerSecret: process.env.DEFI_CALLER_SECRET || "",
};

function required(value, name) {
  if (!value) {
    throw new Error(`Missing ${name}. Set it in environment variables.`);
  }
}

async function simulateRead(server, sourceAddress, operation, networkPassphrase) {
  const account = await server.getAccount(sourceAddress);

  const tx = new TransactionBuilder(account, {
    fee: "100",
    networkPassphrase,
  })
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

  let tx = new TransactionBuilder(account, {
    fee: "1000000",
    networkPassphrase,
  })
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
    if (res.status === "SUCCESS") {
      return res;
    }
    if (res.status === "FAILED") {
      throw new Error("Transaction status FAILED");
    }
    await new Promise((resolve) => setTimeout(resolve, 1200));
  }
}

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

  console.log("\\n=== ANCHOR INTEGRATION FLOW ===");
  console.log("\\n1) Anchor issuer registration check");
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
  const isIssuer = issuerRet ? scValToNative(issuerRet) : false;
  console.log("✓ Anchor registered as issuer:", isIssuer);
  if (!isIssuer) {
    console.log(
      "⚠ Register this anchor from admin first: register_issuer(admin, anchorAddress)"
    );
    return;
  }

  console.log("\\n2) User completes KYC off-chain");
  console.log("✓ User submitted KYC documents");
  console.log("✓ Anchor verified identity and compliance");

  console.log("\\n3) Anchor issues KYC_PASSED attestation");
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
    const writeRes = await submitWrite(
      server,
      anchor,
      createOp,
      cfg.networkPassphrase
    );
    attestationId = writeRes.returnValue ? scValToNative(writeRes.returnValue) : null;
  } catch (err) {
    console.log("⚠ Could not create new attestation (possibly already exists or fee setup missing).");
    console.log("  Reason:", err.message);
  }

  if (attestationId) {
    console.log("✓ Created attestation id:", attestationId);
    console.log("✓ Attestation expires at:", new Date(expiration * 1000).toISOString());
  }

  console.log("\\n4) DeFi contract verifies attestation before allowing deposit");
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
  const verified = verifiedRet ? scValToNative(verifiedRet) : false;

  console.log("✓ DeFi verification result:", verified);
  if (verified) {
    console.log("✓ Action: ALLOW deposit - user has valid KYC attestation");
  } else {
    console.log("✗ Action: DENY deposit - KYC attestation not valid");
  }

  console.log("\\n5) Simulate KYC expiration scenario");
  console.log("⏱ Checking attestation status...");
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
  const status = statusRet ? scValToNative(statusRet) : null;
  console.log("✓ Current attestation status:", status);

  if (status === "Valid") {
    console.log("✓ Attestation is currently valid");
    console.log("⏱ After expiration date, status will become 'Expired'");
    console.log("✗ DeFi contract will then DENY deposits until KYC is renewed");
  }

  console.log("\\n=== FLOW COMPLETE ===");
}

main().catch((err) => {
  console.error("Flow failed:", err.message);
  process.exit(1);
});
