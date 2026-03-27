import { PrismaClient } from "@prisma/client";
import { rpc as SorobanRpc, scValToNative } from "@stellar/stellar-sdk";
import { pubsub, ATTESTATION_CREATED } from "./graphql";

const CONTRACT_ID = process.env.CONTRACT_ID!;
const RPC_URL = process.env.RPC_URL ?? "https://soroban-testnet.stellar.org";
const GENESIS_LEDGER = Number(process.env.GENESIS_LEDGER ?? 0);
const PAGE_LIMIT = 200;
const POLL_MS = 5_000;

const WATCHED = new Set(["created", "revoked", "imported", "bridged"]);

export async function startIndexer(db: PrismaClient): Promise<void> {
  const rpc = new SorobanRpc.Server(RPC_URL, { allowHttp: true });

  // ── Backfill ───────────────────────────────────────────────────────────────
  const checkpoint = await db.checkpoint.findUnique({ where: { id: 1 } });
  let cursor = checkpoint ? checkpoint.ledger + 1 : GENESIS_LEDGER;

  const { sequence: tip } = await rpc.getLatestLedger();
  if (cursor <= tip) {
    console.log(`Backfilling ledgers ${cursor}–${tip}…`);
    cursor = await processRange(db, rpc, cursor, tip);
  }

  // ── Live polling ───────────────────────────────────────────────────────────
  console.log("Live polling for new events…");
  while (true) {
    await sleep(POLL_MS);
    const { sequence: latest } = await rpc.getLatestLedger();
    if (cursor <= latest) {
      cursor = await processRange(db, rpc, cursor, latest);
    }
  }
}

// ── Core processing ──────────────────────────────────────────────────────────

async function processRange(
  db: PrismaClient,
  rpc: SorobanRpc.Server,
  from: number,
  to: number
): Promise<number> {
  let startLedger = from;

  while (startLedger <= to) {
    const response = await rpc.getEvents({
      startLedger,
      endLedger: Math.min(startLedger + PAGE_LIMIT - 1, to),
      filters: [{ type: "contract", contractIds: [CONTRACT_ID] }],
      limit: PAGE_LIMIT,
    });

    for (const ev of response.events) {
      await handleEvent(db, ev);
    }

    const lastProcessed =
      response.events.length > 0
        ? response.events[response.events.length - 1].ledger
        : Math.min(startLedger + PAGE_LIMIT - 1, to);

    startLedger = lastProcessed + 1;

    await db.checkpoint.upsert({
      where: { id: 1 },
      update: { ledger: lastProcessed },
      create: { id: 1, ledger: lastProcessed },
    });
  }

  return to + 1;
}

// ── Event handler ─────────────────────────────────────────────────────────────

async function handleEvent(
  db: PrismaClient,
  ev: SorobanRpc.Api.EventResponse
): Promise<void> {
  if (!ev.topic.length) return;

  // topic[0] is the event name symbol; topic[1] (when present) is subject/issuer address.
  const topicStr = scValToNative(ev.topic[0]) as string;
  if (!WATCHED.has(topicStr)) return;

  const data = scValToNative(ev.value) as unknown[];

  if (topicStr === "revoked") {
    // data: [attestation_id, reason?]
    const attestationId = String(data[0]);
    await db.attestation.updateMany({
      where: { id: attestationId },
      data: { isRevoked: true },
    });
    return;
  }

  // "created" | "imported" | "bridged"
  // topic[1] = subject address
  const subject = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";

  // data: [id, issuer, claimType, timestamp, ...extras]
  const [id, issuer, claimType, rawTs] = data as [string, string, string, bigint | number];
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
    update: { subject, ...extra },
    create: {
      id,
      issuer,
      subject,
      claimType,
      timestamp,
      imported: topicStr === "imported",
      bridged: topicStr === "bridged",
      ...extra,
    },
  });

  // Publish to GraphQL subscriptions
  pubsub.publish(ATTESTATION_CREATED, {
    onAttestationCreated: {
      ...attestation,
      timestamp: String(attestation.timestamp),
      expiration: attestation.expiration != null ? String(attestation.expiration) : null,
      createdAt: attestation.createdAt.toISOString(),
      updatedAt: attestation.updatedAt.toISOString(),
    },
  });
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}
