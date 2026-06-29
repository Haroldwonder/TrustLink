import { PrismaClient } from "@prisma/client";
import { rpc as SorobanRpc, scValToNative } from "@stellar/stellar-sdk";
import type { Redis } from "ioredis";
import { pubsub, ATTESTATION_CREATED, cacheInvalidate } from "./graphql";
import {
  attestationsTotal,
  revocationsTotal,
  eventsProcessedTotal,
  indexerLagLedgers,
} from "./metrics";
import { dispatchWebhooks } from "./webhooks";

const CONTRACT_ID = process.env.CONTRACT_ID!;
const RPC_URL = process.env.RPC_URL ?? "https://soroban-testnet.stellar.org";
const PAGE_LIMIT = 200;
const POLL_MS = 5_000;

const WATCHED = new Set([
  "created",
  "revoked",
  "imported",
  "bridged",
  "ms_prop",
  "ms_sign",
  "ms_actv",
  "iss_reg",
  "rate_limit_set", // #775
]);

let lastLedger = 0;

export function getLastLedger(): number {
  return lastLedger;
}

export async function startIndexer(db: PrismaClient, redis: Redis | null = null): Promise<void> {
  const rpc = new SorobanRpc.Server(RPC_URL, { allowHttp: true });

  // ── Backfill ───────────────────────────────────────────────────────────────
  const checkpoint = await db.checkpoint.findUnique({ where: { id: 1 } });
  let cursor = checkpoint ? checkpoint.ledger + 1 : GENESIS_LEDGER;

  const { sequence: tip } = await rpc.getLatestLedger();
  if (cursor <= tip) {
    console.log(`Backfilling ledgers ${cursor}–${tip}…`);
    try {
      cursor = await processRange(db, rpc, cursor, tip, redis);
    } catch (err) {
      console.error("Error during backfill:", err);
    }
  }

  // ── Live polling ───────────────────────────────────────────────────────────
  console.log("Live polling for new events…");
  while (true) {
    await sleep(POLL_MS);
    const { sequence: latest } = await rpc.getLatestLedger();
    if (cursor <= latest) {
      cursor = await processRange(db, rpc, cursor, latest, redis);
      indexerLagLedgers.set(latest - cursor);
    }
  }
}

// ── Core processing ──────────────────────────────────────────────────────────

async function processRange(
  db: PrismaClient,
  rpc: SorobanRpc.Server,
  from: number,
  to: number,
  redis: Redis | null
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
          await handleEvent(db, ev, redis);
          processedCount++;
        } catch (err) {
          console.error(`Error processing event at ledger ${ev.ledger}:`, err);
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

      if (processedCount % 100 === 0 && processedCount > 0) {
        console.log(`Processed ${processedCount} events, checkpoint: ${lastProcessed}`);
      }
    } catch (err) {
      console.error(`Error fetching events from ledger ${startLedger} to ${endLedger}:`, err);
      await sleep(1000);
      continue;
    }

    lastLedger = Math.min(startLedger - 1, to);
  }

  console.log(`Completed processing range ${from}–${to}, total events: ${processedCount}`);
  return to + 1;
}

// ── Event handler ─────────────────────────────────────────────────────────────

async function handleEvent(
  db: PrismaClient,
  ev: SorobanRpc.Api.EventResponse,
  redis: Redis | null
): Promise<void> {
  if (!ev.topic.length) return;

  const topicStr = scValToNative(ev.topic[0]) as string;
  if (!WATCHED.has(topicStr)) return;

  eventsProcessedTotal.inc();
  const data = scValToNative(ev.value) as unknown[];

  // ── Multi-sig events ───────────────────────────────────────────────────────

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

  // ── #775: rate_limit_set ───────────────────────────────────────────────────

  if (topicStr === "rate_limit_set") {
    // expected data: [issuer_address, rate_limit_value]
    const issuerAddr = String(data[0]);
    const rateLimit = Number(data[1]);
    await db.issuer.upsert({
      where: { address: issuerAddr },
      update: { rateLimit },
      create: { address: issuerAddr, rateLimit },
    });
    // Invalidate issuerStats cache for this issuer
    await cacheInvalidate(redis, `issuerStats:${issuerAddr}`);
    return;
  }

  // ── Issuer registration (for cache invalidation) ───────────────────────────

  if (topicStr === "iss_reg") {
    const issuerAddr = ev.topic[1] ? String(scValToNative(ev.topic[1])) : String(data[0]);
    await cacheInvalidate(redis, `issuerStats:${issuerAddr}`);
    return;
  }

  // ── #776: revoked ──────────────────────────────────────────────────────────

  if (topicStr === "revoked") {
    // contract event data: attestation_id (and optionally reason)
    const attestationId = String(data[0]);
    const revocationReason = data[1] != null ? String(data[1]) : null;

    const attestation = await db.attestation.findUnique({ where: { id: attestationId } });

    await db.attestation.updateMany({
      where: { id: attestationId },
      data: { isRevoked: true, revocationReason },
    });

    // #774: append audit entry for revocation
    if (attestation) {
      const actor = ev.topic[1] ? String(scValToNative(ev.topic[1])) : attestation.issuer;
      await db.auditEntry.create({
        data: {
          attestationId,
          action: "Revoked",
          actor,
          details: revocationReason,
          ledger: ev.ledger,
          timestamp: BigInt(ev.ledgerClosedAt
            ? Math.floor(new Date(ev.ledgerClosedAt).getTime() / 1000)
            : Date.now() / 1000),
        },
      });
      // Invalidate issuerStats cache for this issuer
      await cacheInvalidate(redis, `issuerStats:${attestation.issuer}`);
    }

    revocationsTotal.inc();
    dispatchWebhooks(db, "attestation.revoked", { id: attestationId }).catch(() => {});
    return;
  }

  // ── created | imported | bridged ───────────────────────────────────────────

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

  // #774: append audit entry for creation
  await db.auditEntry.create({
    data: {
      attestationId: id,
      action: topicStr === "imported" ? "Imported" : topicStr === "bridged" ? "Bridged" : "Created",
      actor: issuer,
      details: null,
      ledger: ev.ledger,
      timestamp,
    },
  });

  // Invalidate issuerStats cache
  await cacheInvalidate(redis, `issuerStats:${issuer}`);

  attestationsTotal.inc();

  dispatchWebhooks(db, `attestation.${topicStr}`, {
    ...attestation,
    timestamp: String(attestation.timestamp),
    expiration: attestation.expiration != null ? String(attestation.expiration) : null,
  }).catch(() => {});

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
