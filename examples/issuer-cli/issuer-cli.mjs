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
  indexerUrl: process.env.INDEXER_URL || "",
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

async function proposeMultisig() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const subject = args[1];
  const claimType = args[2];
  if (!subject || !claimType) throw new Error("Usage: propose-multisig <subject> <claim_type> --signers <addr1,addr2,...> --threshold <n>");

  const signersIdx = args.indexOf("--signers");
  const thresholdIdx = args.indexOf("--threshold");
  const signerAddrs = signersIdx !== -1 ? args[signersIdx + 1].split(",") : [];
  const threshold = thresholdIdx !== -1 ? parseInt(args[thresholdIdx + 1]) : 1;

  const issuer = Keypair.fromSecret(config.issuerSecret);
  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);

  const signersScVal = nativeToScVal(signerAddrs.map(a => new Address(a)), { type: "vec" });

  const op = contract.call(
    "propose_attestation",
    new Address(issuer.publicKey()).toScVal(),
    new Address(subject).toScVal(),
    nativeToScVal(claimType, { type: "string" }),
    signersScVal,
    nativeToScVal(threshold, { type: "u32" }),
  );

  const account = await server.getAccount(issuer.publicKey());
  let tx = new TransactionBuilder(account, { fee: "100", networkPassphrase: config.networkPassphrase })
    .addOperation(op).setTimeout(30).build();

  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) throw new Error(`Simulation failed: ${sim.error}`);

  tx = SorobanRpc.assembleTransaction(tx, sim).build();
  tx.sign(issuer);
  const result = await server.sendTransaction(tx);
  console.log(`Multisig proposal submitted. TX: ${result.hash}`);
}

async function cosignProposal() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const proposalId = args[1];
  if (!proposalId) throw new Error("Usage: cosign-proposal <proposal_id>");

  const issuer = Keypair.fromSecret(config.issuerSecret);
  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);

  const op = contract.call(
    "cosign_attestation",
    new Address(issuer.publicKey()).toScVal(),
    nativeToScVal(proposalId, { type: "string" }),
  );

  const account = await server.getAccount(issuer.publicKey());
  let tx = new TransactionBuilder(account, { fee: "100", networkPassphrase: config.networkPassphrase })
    .addOperation(op).setTimeout(30).build();

  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) throw new Error(`Simulation failed: ${sim.error}`);

  tx = SorobanRpc.assembleTransaction(tx, sim).build();
  tx.sign(issuer);
  const result = await server.sendTransaction(tx);
  console.log(`Co-signed proposal ${proposalId}. TX: ${result.hash}`);
}

async function listProposals() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");

  const server = new SorobanRpc.Server(config.rpcUrl);
  const events = await server.getEvents({
    startLedger: 0,
    filters: [{ type: "contract", contractIds: [config.contractId], topics: [["ms_prop"]] }],
    limit: 100,
  });

  if (!events.events || events.events.length === 0) {
    console.log("No multisig proposals found.");
    return;
  }

  console.log("Multisig Proposals:");
  for (const ev of events.events) {
    const [proposalId, proposer, threshold] = ev.value ? scValToNative(ev.value) : [];
    console.log(`  Proposal: ${proposalId}  Proposer: ${proposer}  Threshold: ${threshold}  Ledger: ${ev.ledger}`);
  }
}

async function importAttestation() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const subject = args[1];
  const claimType = args[2];
  if (!subject || !claimType) throw new Error("Usage: import <subject> <claim_type> --source-ref <ref> [--expiry <days>] [--metadata <json>]");

  const issuer = Keypair.fromSecret(config.issuerSecret);
  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);

  // Validate caller is admin
  const adminOp = contract.call("get_admin");
  const adminVal = await simulateRead(server, issuer.publicKey(), adminOp, config.networkPassphrase);
  const admin = adminVal ? scValToNative(adminVal) : null;
  if (admin !== issuer.publicKey()) {
    throw new Error(`Unauthorized: only the admin (${admin}) can import attestations.`);
  }

  const sourceRefIdx = args.indexOf("--source-ref");
  const expiryIdx = args.indexOf("--expiry");
  const metaIdx = args.indexOf("--metadata");
  const sourceRef = sourceRefIdx !== -1 ? args[sourceRefIdx + 1] : null;
  if (!sourceRef) throw new Error("--source-ref is required for import");
  const expiryDays = expiryIdx !== -1 ? parseInt(args[expiryIdx + 1]) : null;
  const metadata = metaIdx !== -1 ? args[metaIdx + 1] : null;
  const now = Math.floor(Date.now() / 1000);
  const expiration = expiryDays ? now + expiryDays * 86400 : null;

  const op = contract.call(
    "import_attestation",
    new Address(issuer.publicKey()).toScVal(),
    new Address(subject).toScVal(),
    nativeToScVal(claimType, { type: "string" }),
    nativeToScVal(sourceRef, { type: "string" }),
    expiration ? nativeToScVal(expiration, { type: "u64" }) : nativeToScVal(null),
    metadata ? nativeToScVal(metadata, { type: "string" }) : nativeToScVal(null),
  );

  const account = await server.getAccount(issuer.publicKey());
  let tx = new TransactionBuilder(account, { fee: "100", networkPassphrase: config.networkPassphrase })
    .addOperation(op).setTimeout(30).build();

  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) throw new Error(`Simulation failed: ${sim.error}`);

  tx = SorobanRpc.assembleTransaction(tx, sim).build();
  tx.sign(issuer);
  const result = await server.sendTransaction(tx);
  console.log(`Attestation imported. TX: ${result.hash}`);
}

// ---------------------------------------------------------------------------
// export-audit-trail
// ---------------------------------------------------------------------------

/**
 * Fetch all attestations for an issuer from the GraphQL indexer, following
 * cursor-based pagination until all pages are consumed.
 */
async function fetchAttestationsByIssuer(issuerAddr) {
  const query = `
    query($issuer: String!, $after: String) {
      attestationsByIssuer(issuer: $issuer, first: 100, after: $after) {
        edges {
          node {
            id
            claimType
            subject
            timestamp
            isRevoked
            createdAt
          }
          cursor
        }
        pageInfo {
          hasNextPage
          endCursor
        }
      }
    }
  `;

  const attestations = [];
  let after = null;

  while (true) {
    const response = await fetch(config.indexerUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query, variables: { issuer: issuerAddr, after } }),
    });

    if (!response.ok) {
      throw new Error(`Indexer request failed: ${response.status} ${response.statusText}`);
    }

    const { data, errors } = await response.json();
    if (errors?.length) throw new Error(errors[0].message);

    const { edges, pageInfo } = data.attestationsByIssuer;
    attestations.push(...edges.map((e) => e.node));

    if (!pageInfo.hasNextPage) break;
    after = pageInfo.endCursor;
  }

  return attestations;
}

/**
 * Normalize an AuditAction value from scValToNative, which may decode a
 * Soroban enum as a string, an array, or an object with a single key.
 */
function parseActionName(raw) {
  if (typeof raw === "string") return raw;
  if (Array.isArray(raw) && raw.length > 0) return String(raw[0]);
  if (raw && typeof raw === "object") return Object.keys(raw)[0] ?? String(raw);
  return String(raw);
}

async function exportAuditTrail() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.indexerUrl, "INDEXER_URL");

  const issuerIdx = args.indexOf("--issuer");
  const fromIdx = args.indexOf("--from");
  const toIdx = args.indexOf("--to");
  const formatIdx = args.indexOf("--format");
  const outputIdx = args.indexOf("--output");

  const issuerAddr = issuerIdx !== -1 ? args[issuerIdx + 1] : null;
  const fromDate = fromIdx !== -1 ? args[fromIdx + 1] : null;
  const toDate = toIdx !== -1 ? args[toIdx + 1] : null;
  const format = formatIdx !== -1 ? args[formatIdx + 1] : "json";
  const outputFile = outputIdx !== -1 ? args[outputIdx + 1] : null;

  if (!issuerAddr || !fromDate || !toDate) {
    console.error(
      "Usage: issuer-cli export-audit-trail --issuer <address> --from <ISO-date> --to <ISO-date> [--format csv|json] [--output <file>]"
    );
    console.error("Example: issuer-cli export-audit-trail --issuer GABC... --from 2024-01-01 --to 2024-12-31 --format csv --output audit.csv");
    process.exit(1);
  }

  const fromTs = Math.floor(new Date(fromDate).getTime() / 1000);
  const toTs = Math.floor(new Date(toDate).getTime() / 1000);

  if (isNaN(fromTs) || isNaN(toTs)) {
    console.error("Invalid date format. Use ISO 8601 dates, e.g. 2024-01-01 or 2024-01-01T00:00:00Z.");
    process.exit(1);
  }

  console.log(`\nExporting audit trail for issuer: ${issuerAddr}`);
  console.log(`Date range: ${fromDate} → ${toDate}`);
  console.log("Fetching attestations from indexer...");

  const attestations = await fetchAttestationsByIssuer(issuerAddr);
  console.log(`Found ${attestations.length} attestation(s).`);

  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);

  // Use the issuer's own account as simulation source if secret is provided;
  // otherwise fall back to the issuer address itself (must exist on-chain).
  const sourceAddress = config.issuerSecret
    ? Keypair.fromSecret(config.issuerSecret).publicKey()
    : issuerAddr;

  console.log("Fetching audit log entries from contract...");

  const rows = [];
  for (const att of attestations) {
    let auditEntries = [];
    try {
      const auditOp = contract.call(
        "get_audit_log",
        nativeToScVal(att.id, { type: "string" })
      );
      const retval = await simulateRead(server, sourceAddress, auditOp, config.networkPassphrase);
      auditEntries = retval ? scValToNative(retval) : [];
    } catch (err) {
      console.warn(`  Warning: Could not fetch audit log for ${att.id}: ${err.message}`);
      continue;
    }

    for (const entry of auditEntries) {
      const ts = Number(typeof entry.timestamp === "bigint" ? entry.timestamp : BigInt(entry.timestamp));
      if (ts >= fromTs && ts <= toTs) {
        const actorStr =
          typeof entry.actor === "string"
            ? entry.actor
            : entry.actor?.toString?.() ?? String(entry.actor);

        rows.push({
          issuer: issuerAddr,
          attestation_id: att.id,
          claim_type: att.claimType,
          subject: att.subject,
          action: parseActionName(entry.action),
          actor: actorStr,
          timestamp: new Date(ts * 1000).toISOString(),
          details: entry.details ?? "",
        });
      }
    }
  }

  if (rows.length === 0) {
    console.log("No audit entries found in the specified date range.");
    return;
  }

  let output;
  if (format === "csv") {
    const header = "issuer,attestation_id,claim_type,subject,action,actor,timestamp,details";
    const csvRows = rows.map((r) =>
      [
        r.issuer,
        r.attestation_id,
        r.claim_type,
        r.subject,
        r.action,
        r.actor,
        r.timestamp,
        `"${String(r.details).replace(/"/g, '""')}"`,
      ].join(",")
    );
    output = [header, ...csvRows].join("\n");
  } else {
    output = JSON.stringify(rows, null, 2);
  }

  if (outputFile) {
    fs.writeFileSync(outputFile, output, "utf8");
    console.log(`✓ Written ${rows.length} audit entries to ${outputFile}`);
  } else {
    console.log(output);
    console.log(`\n✓ Exported ${rows.length} audit entries`);
  }
}

// ---------------------------------------------------------------------------
// CSV helpers
// ---------------------------------------------------------------------------

function parseCsvLine(line) {
  const values = [];
  let current = "";
  let inQuotes = false;
  for (const char of line) {
    if (char === '"') {
      inQuotes = !inQuotes;
    } else if (char === "," && !inQuotes) {
      values.push(current);
      current = "";
    } else {
      current += char;
    }
  }
  values.push(current);
  return values.map((v) => v.trim());
}

function parseCsv(content) {
  const lines = content.trim().split(/\r?\n/).filter((l) => l.trim());
  if (lines.length < 2) throw new Error("CSV must have a header row and at least one data row.");
  const headers = parseCsvLine(lines[0]).map((h) => h.toLowerCase());
  const addrIdx = headers.indexOf("address");
  if (addrIdx === -1) throw new Error("CSV must have an 'address' column.");
  const ctIdx = headers.indexOf("claim_type");
  const expIdx = headers.indexOf("expiry_days");
  const metaIdx = headers.indexOf("metadata");
  const rows = [];
  for (let i = 1; i < lines.length; i++) {
    const vals = parseCsvLine(lines[i]);
    const address = vals[addrIdx];
    if (!address) continue;
    rows.push({
      lineNum: i + 1,
      address,
      claim_type: ctIdx >= 0 ? vals[ctIdx] || null : null,
      expiry_days: expIdx >= 0 && vals[expIdx] ? parseInt(vals[expIdx], 10) : null,
      metadata: metaIdx >= 0 ? vals[metaIdx] || null : null,
    });
  }
  return rows;
}

// ---------------------------------------------------------------------------
// bulk-create
// ---------------------------------------------------------------------------

async function bulkCreate() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const fileIdx = args.indexOf("--file");
  const claimTypeArg = args.find((a) => a === "--claim-type") ? args[args.indexOf("--claim-type") + 1] : null;
  const defaultExpiry = args.find((a) => a === "--expiry") ? parseInt(args[args.indexOf("--expiry") + 1], 10) : 365;

  if (fileIdx === -1 || !args[fileIdx + 1]) {
    console.error("Usage: issuer-cli bulk-create --file subjects.csv --claim-type <type> [--expiry <days>]");
    console.error("CSV columns: address (required), claim_type, expiry_days, metadata");
    process.exit(1);
  }

  const csvPath = args[fileIdx + 1];
  let csvContent;
  try {
    csvContent = fs.readFileSync(path.resolve(csvPath), "utf8");
  } catch (err) {
    console.error(`Cannot read file ${csvPath}: ${err.message}`);
    process.exit(1);
  }

  const rows = parseCsv(csvContent);
  if (rows.length === 0) {
    console.error("No data rows found in CSV.");
    process.exit(1);
  }

  console.log(`\nBulk-creating ${rows.length} attestation(s) from ${csvPath}…\n`);

  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);
  const issuer = Keypair.fromSecret(config.issuerSecret);

  let succeeded = 0;
  let failed = 0;

  for (const row of rows) {
    const ct = row.claim_type || claimTypeArg;
    if (!ct) {
      console.error(`  [row ${row.lineNum}] SKIP  ${row.address} — no claim_type; pass --claim-type or add a claim_type column`);
      failed++;
      continue;
    }

    const expiryDays = row.expiry_days ?? defaultExpiry;
    const expiration = Math.floor(Date.now() / 1000) + expiryDays * 24 * 60 * 60;

    const createOp = contract.call(
      "create_attestation",
      nativeToScVal(Address.fromString(issuer.publicKey()), { type: "address" }),
      nativeToScVal(Address.fromString(row.address), { type: "address" }),
      nativeToScVal(ct, { type: "string" }),
      nativeToScVal(expiration, { type: "u64" }),
      row.metadata ? nativeToScVal(row.metadata, { type: "string" }) : nativeToScVal(null, { type: "void" })
    );

    try {
      const res = await submitWrite(server, issuer, createOp, config.networkPassphrase);
      const attestationId = res.returnValue ? scValToNative(res.returnValue) : "—";
      console.log(`  [row ${row.lineNum}] OK    ${row.address}  claim=${ct}  id=${attestationId}`);
      succeeded++;
    } catch (err) {
      console.error(`  [row ${row.lineNum}] FAIL  ${row.address}  claim=${ct}  error=${err.message}`);
      failed++;
    }
  }

  console.log(`\nDone: ${succeeded} succeeded, ${failed} failed out of ${rows.length} rows.`);
  if (failed > 0) process.exit(1);
}

// ---------------------------------------------------------------------------
// template management
// ---------------------------------------------------------------------------

async function templateCreate() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const idIdx = args.indexOf("--id");
  const ctIdx = args.indexOf("--claim-type");
  const metaIdx = args.indexOf("--metadata");

  const templateId = idIdx >= 0 ? args[idIdx + 1] : null;
  const claimType = ctIdx >= 0 ? args[ctIdx + 1] : null;
  const metadata = metaIdx >= 0 ? args[metaIdx + 1] : null;

  if (!templateId || !claimType) {
    console.error("Usage: issuer-cli template create --id <template_id> --claim-type <type> [--metadata <json>]");
    process.exit(1);
  }

  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);
  const issuer = Keypair.fromSecret(config.issuerSecret);

  console.log(`\nCreating template "${templateId}" (${claimType})…`);

  const op = contract.call(
    "create_template",
    nativeToScVal(Address.fromString(issuer.publicKey()), { type: "address" }),
    nativeToScVal(templateId, { type: "string" }),
    nativeToScVal(claimType, { type: "string" }),
    metadata ? nativeToScVal(metadata, { type: "string" }) : nativeToScVal(null, { type: "void" })
  );

  await submitWrite(server, issuer, op, config.networkPassphrase);
  console.log(`✓ Template "${templateId}" created.`);
}

async function templateList() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);
  const issuer = Keypair.fromSecret(config.issuerSecret);

  console.log(`\nListing templates for ${issuer.publicKey()}…`);

  const op = contract.call(
    "list_templates",
    nativeToScVal(Address.fromString(issuer.publicKey()), { type: "address" })
  );

  const retval = await simulateRead(server, issuer.publicKey(), op, config.networkPassphrase);
  const templates = retval ? scValToNative(retval) : [];

  if (templates.length === 0) {
    console.log("  (no templates)");
    return;
  }

  console.log(`\n  Found ${templates.length} template(s):`);
  templates.forEach((tmpl, i) => {
    console.log(`  ${i + 1}. ID: ${tmpl.template_id}`);
    console.log(`     Claim Type: ${tmpl.claim_type}`);
    if (tmpl.metadata) console.log(`     Metadata: ${tmpl.metadata}`);
  });
}

async function templateDelete() {
  required(config.contractId, "TRUSTLINK_CONTRACT_ID");
  required(config.issuerSecret, "ISSUER_SECRET");

  const templateId = args[2];
  if (!templateId) {
    console.error("Usage: issuer-cli template delete <template_id>");
    process.exit(1);
  }

  const server = new SorobanRpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);
  const issuer = Keypair.fromSecret(config.issuerSecret);

  console.log(`\nDeleting template "${templateId}"…`);

  const op = contract.call(
    "delete_template",
    nativeToScVal(Address.fromString(issuer.publicKey()), { type: "address" }),
    nativeToScVal(templateId, { type: "string" })
  );

  await submitWrite(server, issuer, op, config.networkPassphrase);
  console.log(`✓ Template "${templateId}" deleted.`);
}

async function handleTemplate() {
  const subcommand = args[1];
  switch (subcommand) {
    case "create":
      await templateCreate();
      break;
    case "list":
      await templateList();
      break;
    case "delete":
      await templateDelete();
      break;
    default:
      console.error(`Unknown template subcommand: ${subcommand}`);
      console.error("Usage: issuer-cli template <create|list|delete> [options]");
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

  propose-multisig <subject> <claim_type> --signers <addr1,addr2,...> --threshold <n>
    Propose a multisig attestation requiring co-signatures

  cosign-proposal <proposal_id>
    Co-sign an existing multisig proposal

  list-proposals
    List all multisig proposals for this contract

  import <subject> <claim_type> --source-ref <ref> [--expiry <days>] [--metadata <json>]
    Import an off-chain attestation (admin only)

  export-audit-trail --issuer <address> --from <date> --to <date> [--format csv|json] [--output <file>]
    Export a regulator-ready audit trail for an issuer over a date range.
    Queries attestations from the indexer and fetches audit log entries from
    the contract. Output is JSON (default) or CSV.

  bulk-create --file subjects.csv --claim-type <type> [--expiry <days>]
    Batch-create attestations from a CSV file. The CSV must have an 'address'
    column; optional per-row columns: claim_type, expiry_days, metadata.
    Per-row claim_type overrides --claim-type when present.
    Reports success/failure for every row.

  template create --id <template_id> --claim-type <type> [--metadata <json>]
    Create an attestation template for this issuer.

  template list
    List all templates for this issuer.

  template delete <template_id>
    Delete an existing template.

Environment Variables:
  RPC_URL                 Stellar RPC endpoint (default: testnet)
  NETWORK_PASSPHRASE      Stellar network (default: testnet)
  TRUSTLINK_CONTRACT_ID   TrustLink contract address
  ISSUER_SECRET           Issuer's secret key
  INDEXER_URL             TrustLink indexer GraphQL endpoint (required for export-audit-trail)

Examples:
  issuer-cli issue GBRPYHIL... KYC_PASSED --expiry 365
  issuer-cli revoke att_abc123 --reason "User requested"
  issuer-cli list-issued --page 0 --limit 10
  issuer-cli check GBRPYHIL... KYC_PASSED
  issuer-cli propose-multisig GBRPYHIL... KYC_PASSED --signers GABC...,GDEF... --threshold 2
  issuer-cli cosign-proposal proposal_abc123
  issuer-cli list-proposals
  issuer-cli import GBRPYHIL... KYC_PASSED --source-ref "external-id-123" --expiry 365
  issuer-cli export-audit-trail --issuer GABC... --from 2024-01-01 --to 2024-12-31 --format csv --output audit.csv
  issuer-cli bulk-create --file subjects.csv --claim-type KYC_PASSED --expiry 365
  issuer-cli template create --id kyc-standard --claim-type KYC_PASSED
  issuer-cli template list
  issuer-cli template delete kyc-standard
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
      case "propose-multisig":
        await proposeMultisig();
        break;
      case "cosign-proposal":
        await cosignProposal();
        break;
      case "list-proposals":
        await listProposals();
        break;
      case "import":
        await importAttestation();
        break;
      case "export-audit-trail":
        await exportAuditTrail();
        break;
      case "bulk-create":
        await bulkCreate();
        break;
      case "template":
        await handleTemplate();
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
