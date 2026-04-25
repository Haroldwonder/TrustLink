#!/usr/bin/env node

/**
 * TrustLink Issuer CLI
 * 
 * Simple command-line tool for issuers to manage attestations without writing code.
 * 
 * Usage:
 *   issuer-cli issue <subject> <claim_type> [--expiry <days>] [--metadata <json>]
 *   issuer-cli revoke <attestation_id> [--reason <text>]
 *   issuer-cli list-issued [--page <n>] [--limit <n>]
 *   issuer-cli check <subject> <claim_type>
 */

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
import * as fs from "fs";
import * as path from "path";

const args = process.argv.slice(2);
const command = args[0];

// Configuration
const config = {
  rpcUrl: process.env.RPC_URL || "https://soroban-testnet.stellar.org",
  networkPassphrase: process.env.NETWORK_PASSPHRASE || Networks.TESTNET,
  contractId: process.env.TRUSTLINK_CONTRACT_ID || "",
  issuerSecret: process.env.ISSUER_SECRET || "",
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

async function issueAttestation() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const subject = args[1];
  const claimType = args[2];
  const expiryDays = parseInt(
    args.find((a) => a === "--expiry") ? args[args.indexOf("--expiry") + 1] : "365"
  );
  const metadataIdx = args.indexOf("--metadata");
  const metadata = metadataIdx >= 0 ? args[metadataIdx + 1] : null;

  if (!subject || !claimType) {
    console.error("Usage: issuer-cli issue <subject> <claim_type> [--expiry <days>] [--metadata <json>]");
    process.exit(1);
  }

  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);
  const issuer = Keypair.fromSecret(config.issuerSecret);

  const expiration = Math.floor(Date.now() / 1000) + expiryDays * 24 * 60 * 60;

  console.log(`\n📝 Issuing attestation...`);
  console.log(`   Subject: ${subject}`);
  console.log(`   Claim Type: ${claimType}`);
  console.log(`   Expires in: ${expiryDays} days`);
  if (metadata) console.log(`   Metadata: ${metadata}`);

  const createOp = contract.call(
    "create_attestation",
    nativeToScVal(Address.fromString(issuer.publicKey()), { type: "address" }),
    nativeToScVal(Address.fromString(subject), { type: "address" }),
    nativeToScVal(claimType, { type: "string" }),
    nativeToScVal(expiration, { type: "u64" }),
    metadata ? nativeToScVal(metadata, { type: "string" }) : nativeToScVal(null, { type: "void" })
  );

  try {
    const writeRes = await submitWrite(server, issuer, createOp, config.networkPassphrase);
    const attestationId = writeRes.returnValue ? scValToNative(writeRes.returnValue) : null;
    console.log(`✓ Attestation created: ${attestationId}`);
    console.log(`✓ Expires: ${new Date(expiration * 1000).toISOString()}`);
  } catch (err) {
    console.error(`✗ Failed to create attestation: ${err.message}`);
    process.exit(1);
  }
}

async function revokeAttestation() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const attestationId = args[1];
  const reasonIdx = args.indexOf("--reason");
  const reason = reasonIdx >= 0 ? args[reasonIdx + 1] : null;

  if (!attestationId) {
    console.error("Usage: issuer-cli revoke <attestation_id> [--reason <text>]");
    process.exit(1);
  }

  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);
  const issuer = Keypair.fromSecret(config.issuerSecret);

  console.log(`\n🗑️  Revoking attestation...`);
  console.log(`   ID: ${attestationId}`);
  if (reason) console.log(`   Reason: ${reason}`);

  const revokeOp = contract.call(
    "revoke_attestation",
    nativeToScVal(Address.fromString(issuer.publicKey()), { type: "address" }),
    nativeToScVal(attestationId, { type: "string" }),
    reason ? nativeToScVal(reason, { type: "string" }) : nativeToScVal(null, { type: "void" })
  );

  try {
    await submitWrite(server, issuer, revokeOp, config.networkPassphrase);
    console.log(`✓ Attestation revoked`);
  } catch (err) {
    console.error(`✗ Failed to revoke attestation: ${err.message}`);
    process.exit(1);
  }
}

async function listIssued() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const pageIdx = args.indexOf("--page");
  const page = pageIdx >= 0 ? parseInt(args[pageIdx + 1]) : 0;
  const limitIdx = args.indexOf("--limit");
  const limit = limitIdx >= 0 ? parseInt(args[limitIdx + 1]) : 10;

  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);
  const issuer = Keypair.fromSecret(config.issuerSecret);

  console.log(`\n📋 Listing issued attestations...`);
  console.log(`   Page: ${page}, Limit: ${limit}`);

  const listOp = contract.call(
    "get_issuer_attestations",
    nativeToScVal(Address.fromString(issuer.publicKey()), { type: "address" }),
    nativeToScVal(page, { type: "u64" }),
    nativeToScVal(limit, { type: "u64" })
  );

  try {
    const listRet = await simulateRead(server, issuer.publicKey(), listOp, config.networkPassphrase);
    const attestations = listRet ? scValToNative(listRet) : [];
    
    if (attestations.length === 0) {
      console.log(`   (no attestations)`);
      return;
    }

    console.log(`\n   Found ${attestations.length} attestation(s):`);
    attestations.forEach((att, i) => {
      console.log(`   ${i + 1}. ID: ${att.id}`);
      console.log(`      Subject: ${att.subject}`);
      console.log(`      Claim: ${att.claim_type}`);
      console.log(`      Status: ${att.revoked ? "Revoked" : "Active"}`);
    });
  } catch (err) {
    console.error(`✗ Failed to list attestations: ${err.message}`);
    process.exit(1);
  }
}

async function checkClaim() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const subject = args[1];
  const claimType = args[2];

  if (!subject || !claimType) {
    console.error("Usage: issuer-cli check <subject> <claim_type>");
    process.exit(1);
  }

  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);
  const issuer = Keypair.fromSecret(config.issuerSecret);

  console.log(`\n🔍 Checking claim...`);
  console.log(`   Subject: ${subject}`);
  console.log(`   Claim Type: ${claimType}`);

  const checkOp = contract.call(
    "has_valid_claim_from_issuer",
    nativeToScVal(Address.fromString(subject), { type: "address" }),
    nativeToScVal(claimType, { type: "string" }),
    nativeToScVal(Address.fromString(issuer.publicKey()), { type: "address" })
  );

  try {
    const checkRet = await simulateRead(server, issuer.publicKey(), checkOp, config.networkPassphrase);
    const hasValid = checkRet ? scValToNative(checkRet) : false;
    
    if (hasValid) {
      console.log(`✓ Subject has valid ${claimType} claim from this issuer`);
    } else {
      console.log(`✗ Subject does NOT have valid ${claimType} claim from this issuer`);
    }
  } catch (err) {
    console.error(`✗ Failed to check claim: ${err.message}`);
    process.exit(1);
  }
}

function showHelp() {
  console.log(`
TrustLink Issuer CLI

Commands:
  issue <subject> <claim_type> [--expiry <days>] [--metadata <json>]
    Issue a new attestation
    
  revoke <attestation_id> [--reason <text>]
    Revoke an existing attestation
    
  list-issued [--page <n>] [--limit <n>]
    List attestations issued by this issuer
    
  check <subject> <claim_type>
    Check if subject has a valid claim

Environment Variables:
  RPC_URL                 Stellar RPC endpoint (default: testnet)
  NETWORK_PASSPHRASE      Stellar network (default: testnet)
  TRUSTLINK_CONTRACT_ID   TrustLink contract address
  ISSUER_SECRET           Issuer's secret key

Examples:
  issuer-cli issue GBRPYHIL... KYC_PASSED --expiry 365
  issuer-cli revoke att_abc123 --reason "User requested"
  issuer-cli list-issued --page 0 --limit 10
  issuer-cli check GBRPYHIL... KYC_PASSED
`);
}

async function main() {
  try {
    if (!command || command === "--help" || command === "-h") {
      showHelp();
      return;
    }

    switch (command) {
      case "issue":
        await issueAttestation();
        break;
      case "revoke":
        await revokeAttestation();
        break;
      case "list-issued":
        await listIssued();
        break;
      case "check":
        await checkClaim();
        break;
      default:
        console.error(`Unknown command: ${command}`);
        showHelp();
        process.exit(1);
    }
  } catch (err) {
    console.error(`Error: ${err.message}`);
    process.exit(1);
  }
}

main();
