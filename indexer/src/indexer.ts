import { PrismaClient } from "@prisma/client";
import { rpc as SorobanRpc, scValToNative } from "@stellar/stellar-sdk";
import { pubsub, ATTESTATION_CREATED } from "./graphql";
import {
  attestationsTotal,
  revocationsTotal,
  eventsProcessedTotal,
  indexerLagLedgers,
} from "./metrics";
import { dispatchWebhooks } from "./webhooks";

// ── Configuration ─────────────────────────────────────────────────────────────

/**
 * Accept one or more contract IDs to track.
 *
 * Set a single contract:
 *   CONTRACT_ID=CXXX...
 *
 * Set multiple contracts (comma-separated):
 *   CONTRACT_IDS=CXXX...,CYYY...,CZZZ...
 *
 * CONTRACT_IDS takes precedence over CONTRACT_ID when both are set.
 */
function resolveContractIds(): string[] {
  const multi = process.env.CONTRACT_IDS;
  if (multi) {
    const ids = multi
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    if (ids.length > 0) return ids;
  }
  const single = process.env.CONTRACT_ID;
  if (single) return [single];
  throw new Error(
    "No contract IDs configured. Set CONTRACT_ID or CONTRACT_IDS environment variable."
  );
}

const CONTRACT_IDS = resolveContractIds();
const RPC_URL = process.env.RPC_URL ?? "https://soroban-testnet.stellar.org";
const PAGE_LIMIT = 200;
const POLL_MS = 5_000;

// Ledger sequence to start from when no checkpoint exists.
// Callers can override via GENESIS_LEDGER env var.
const GENESIS_LEDGER = Number(process.env.GENESIS_LEDGER ?? 0);

const WATCHED = new Set([
  "created",
  "revoked",
  "imported",
  "bridged",
  "ms_prop",
  "ms_sign",
  "ms_actv",
]);

// Per-contract last-ledger tracker (in-memory, for metrics).
const lastLedgerByContract = new Map<string, number>();

export function getLastLedger(): number {
  return Math.min(...Array.from(lastLedgerByContract.values()), 0);
}

// ── Entry point ───────────────────────────────────────────────────────────────

export async function startIndexer(db: PrismaClient): Promise<void> {
  const rpc = new SorobanRpc.Server(RPC_URL, { allowHttp: true });

  console.log(`Tracking ${CONTRACT_IDS.length} contract(s): ${CONTRACT_IDS.join(", ")}`);

  // Start an independent indexer loop for each contract ID.
  await Promise.all(
    CONTRACT_IDS.map((contractId) => runContractIndexer(db, rpc, contractId))
  );
}

// ── Per-contract indexer loop ─────────────────────────────────────────────────

async function runContractIndexer(
  db: PrismaClient,
  rpc: SorobanRpc.Server,
  contractId: string
): Promise<void> {
  // Load per-contract checkpoint (ledger keyed by contractId).
  const checkpoint = await db.checkpoint.findUnique({
    where: { contractId },
  });
  let cursor = checkpoint ? checkpoint.ledger + 1 : GENESIS_LEDGER;

  const { sequence: tip } = await rpc.getLatestLedger();
  if (cursor <= tip) {
    console.log(`[${contractId}] Backfilling ledgers ${cursor}–${tip}…`);
    try {
      cursor = await processRange(db, rpc, contractId, cursor, tip);
    } catch (err) {
      console.error(`[${contractId}] Error during backfill:`, err);
    }
  }

  console.log(`[${contractId}] Live polling for new events…`);
  while (true) {
    await sleep(POLL_MS);
    const { sequence: latest } = await rpc.getLatestLedger();
    if (cursor <= latest) {
      cursor = await processRange(db, rpc, contractId, cursor, latest);
      indexerLagLedgers.set(latest - cursor);
    }
  }
}

// ── Core processing ───────────────────────────────────────────────────────────

async function processRange(
  db: PrismaClient,
  rpc: SorobanRpc.Server,
  contractId: string,
  from: number,
  to: number
): Promise<number> {
  let startLedger = from;
  let processedCount = 0;

  while (startLedger <= to) {
    const endLedger = Math.min(startLedger + PAGE_LIMIT - 1, to);
    let lastProcessed = endLedger;

    try {
      const response = await rpc.getEvents({
        startLedger,
        endLedger,
        filters: [{ type: "contract", contractIds: [contractId] }],
        limit: PAGE_LIMIT,
      });

      for (const ev of response.events) {
        try {
          await handleEvent(db, contractId, ev);
          processedCount++;
        } catch (err) {
          console.error(
            `[${contractId}] Error processing event at ledger ${ev.ledger}:`,
            err
          );
        }
      }

      if (response.events.length > 0) {
        lastProcessed = response.events[response.events.length - 1].ledger;
      }
    } catch (err) {
      console.error(
        `[${contractId}] Error fetching events ${startLedger}–${endLedger}:`,
        err
      );
      await sleep(1_000);
      continue;
    }

    lastLedgerByContract.set(contractId, lastProcessed);
    startLedger = lastProcessed + 1;

    await db.checkpoint.upsert({
      where: { contractId },
      update: { ledger: lastProcessed },
      create: { contractId, ledger: lastProcessed },
    });

    if (processedCount > 0 && processedCount % 100 === 0) {
      console.log(
        `[${contractId}] Processed ${processedCount} events, checkpoint: ${lastProcessed}`
      );
    }
  }

  if (processedCount > 0) {
    console.log(
      `[${contractId}] Completed range ${from}–${to}, total events: ${processedCount}`
    );
  }

  return to + 1;
}

// ── Event handler ─────────────────────────────────────────────────────────────

async function handleEvent(
  db: PrismaClient,
  contractId: string,
  ev: SorobanRpc.Api.EventResponse
): Promise<void> {
  if (!ev.topic.length) return;

  const topicStr = scValToNative(ev.topic[0]) as string;
  if (!WATCHED.has(topicStr)) return;

  eventsProcessedTotal.inc();
  const data = scValToNative(ev.value) as unknown[];

  // ── multi-sig events ────────────────────────────────────────────────────────
  if (topicStr === "ms_prop") {
    const proposalId = String(data[0]);
    const proposer = String(data[1]);
    const threshold = Number(data[2]);
    const subject = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";

    await db.multisigProposal.upsert({
      where: { id: proposalId },
      update: {},
      create: {
        id: proposalId,
        contractId,
        subject,
        proposer,
        claimType: "",
        threshold,
        signers: [proposer],
        signatureCount: 1,
        expiresAt: BigInt(Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60),
      },
    });
    return;
  }

  if (topicStr === "ms_sign") {
    const proposalId = String(data[0]);
    const signatureCount = Number(data[1]);
    await db.multisigProposal.update({
      where: { id: proposalId },
      data: { signatureCount },
    });
    return;
  }

  if (topicStr === "ms_actv") {
    const proposalId = String(data[0]);
    await db.multisigProposal.update({
      where: { id: proposalId },
      data: { finalized: true },
    });
    attestationsTotal.inc();
    return;
  }

  if (topicStr === "revoked") {
    const attestationId = String(data[0]);
    await db.attestation.updateMany({
      where: { id: attestationId, contractId },
      data: { isRevoked: true },
    });
    revocationsTotal.inc();
    dispatchWebhooks(db, "attestation.revoked", {
      id: attestationId,
      contractId,
    }).catch(() => {});
    return;
  }

  // ── "created" | "imported" | "bridged" ─────────────────────────────────────
  const subject = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
  const [id, issuer, claimType, rawTs] = data as [
    string,
    string,
    string,
    bigint | number
  ];
  const timestamp = BigInt(rawTs);

  let extra: Record<string, unknown> = {};
  if (topicStr === "created") {
    extra = { metadata: data[4] != null ? String(data[4]) : null };
  } else if (topicStr === "imported") {
    extra = { expiration: data[4] != null ? BigInt(data[4] as number) : null };
  } else if (topicStr === "bridged") {
    extra = {
      sourceChain: data[4] != null ? String(data[4]) : null,
      sourceTx: data[5] != null ? String(data[5]) : null,
    };
  }

  const attestation = await db.attestation.upsert({
    where: { id },
    update: { subject, contractId, ...extra },
    create: {
      id,
      contractId,
      issuer,
      subject,
      claimType,
      timestamp,
      imported: topicStr === "imported",
      bridged: topicStr === "bridged",
      ...extra,
    },
  });

  attestationsTotal.inc();

  dispatchWebhooks(db, `attestation.${topicStr}`, {
    ...attestation,
    timestamp: String(attestation.timestamp),
    expiration:
      attestation.expiration != null ? String(attestation.expiration) : null,
  }).catch(() => {});

  pubsub.publish(ATTESTATION_CREATED, {
    onAttestationCreated: {
      ...attestation,
      timestamp: String(attestation.timestamp),
      expiration:
        attestation.expiration != null ? String(attestation.expiration) : null,
      createdAt: attestation.createdAt.toISOString(),
      updatedAt: attestation.updatedAt.toISOString(),
    },
  });
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}
