import { PrismaClient } from "@prisma/client";
import { rpc as SorobanRpc, scValToNative } from "@stellar/stellar-sdk";
import { pubsub, ATTESTATION_CREATED, ATTESTATION_REVOKED, ISSUER_REGISTERED } from "./graphql";

const CONTRACT_ID = process.env.CONTRACT_ID!;
const RPC_URL = process.env.RPC_URL ?? "https://soroban-testnet.stellar.org";
const PAGE_LIMIT = 200;
const POLL_MS = 5_000;

// Parse --from-ledger CLI flag or use environment variable
let GENESIS_LEDGER = Number(process.env.GENESIS_LEDGER ?? 0);
const args = process.argv.slice(2);
const fromLedgerIdx = args.indexOf("--from-ledger");
if (fromLedgerIdx !== -1 && args[fromLedgerIdx + 1]) {
  GENESIS_LEDGER = Number(args[fromLedgerIdx + 1]);
  console.log(`Starting from ledger: ${GENESIS_LEDGER}`);
}

const WATCHED = new Set(["created", "revoked", "imported", "bridged", "iss_reg"]);

export async function startIndexer(db: PrismaClient): Promise<void> {
  const rpc = new SorobanRpc.Server(RPC_URL, { allowHttp: true });

  // ── Backfill ───────────────────────────────────────────────────────────────
  const checkpoint = await db.checkpoint.findUnique({ where: { id: 1 } });
  let cursor = checkpoint ? checkpoint.ledger + 1 : GENESIS_LEDGER;

  const { sequence: tip } = await rpc.getLatestLedger();
  if (cursor <= tip) {
    console.log(`Backfilling ledgers ${cursor}–${tip}…`);
    try {
      cursor = await processRange(db, rpc, cursor, tip);
    } catch (err) {
      console.error("Error during backfill:", err);
      // Continue with live polling even if backfill fails
    }
  }

  // ── Live polling ───────────────────────────────────────────────────────────
  console.log("Live polling for new events…");
  while (true) {
    await sleep(POLL_MS);
    try {
      const { sequence: latest } = await rpc.getLatestLedger();
      if (cursor <= latest) {
        cursor = await processRange(db, rpc, cursor, latest);
      }
    } catch (err) {
      console.error("Error during live polling:", err);
      // Continue polling on error
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
  let processedCount = 0;

  while (startLedger <= to) {
    const endLedger = Math.min(startLedger + PAGE_LIMIT - 1, to);
    
    try {
      const response = await rpc.getEvents({
        startLedger,
        endLedger,
        filters: [{ type: "contract", contractIds: [CONTRACT_ID] }],
        limit: PAGE_LIMIT,
      });

      for (const ev of response.events) {
        try {
          await handleEvent(db, ev);
          processedCount++;
        } catch (err) {
          console.error(`Error processing event at ledger ${ev.ledger}:`, err);
          // Continue processing other events
        }
      }

      const lastProcessed =
        response.events.length > 0
          ? response.events[response.events.length - 1].ledger
          : endLedger;

      startLedger = lastProcessed + 1;

      await db.checkpoint.upsert({
        where: { id: 1 },
        update: { ledger: lastProcessed },
        create: { id: 1, ledger: lastProcessed },
      });

      if (processedCount % 100 === 0) {
        console.log(`Processed ${processedCount} events, checkpoint: ${lastProcessed}`);
      }
    } catch (err) {
      console.error(`Error fetching events from ledger ${startLedger} to ${endLedger}:`, err);
      // Retry with exponential backoff
      await sleep(1000);
      continue;
    }
  }

  console.log(`Completed processing range ${from}–${to}, total events: ${processedCount}`);
  return to + 1;
}

// ── Event handler ─────────────────────────────────────────────────────────────

async function handleEvent(
  db: PrismaClient,
  ev: SorobanRpc.Api.EventResponse
): Promise<void> {
  if (!ev.topic.length) return;

  const topicStr = scValToNative(ev.topic[0]) as string;
  if (!WATCHED.has(topicStr)) return;

  const data = scValToNative(ev.value) as unknown[];

  if (topicStr === "revoked") {
    const attestationId = String(data[0]);
    const attestation = await db.attestation.findUnique({
      where: { id: attestationId },
    });
    
    await db.attestation.updateMany({
      where: { id: attestationId },
      data: { isRevoked: true },
    });

    if (attestation) {
      pubsub.publish(ATTESTATION_REVOKED, {
        onAttestationRevoked: {
          id: attestationId,
          issuer: attestation.issuer,
          revokedAt: new Date().toISOString(),
        },
      });
    }
    return;
  }

  if (topicStr === "iss_reg") {
    const issuer = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
    pubsub.publish(ISSUER_REGISTERED, {
      onIssuerRegistered: {
        issuer,
        registeredAt: new Date().toISOString(),
      },
    });
    return;
  }

  // "created" | "imported" | "bridged"
  const subject = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
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
