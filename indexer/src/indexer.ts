import { PrismaClient } from "@prisma/client";
import { rpc as SorobanRpc, scValToNative } from "@stellar/stellar-sdk";
import type { Redis } from "ioredis";
import { pubsub, ATTESTATION_CREATED, ATTESTATION_REVOKED, ISSUER_REGISTERED, cacheInvalidate } from "./graphql";
import {
  attestationsTotal,
  revocationsTotal,
  eventsProcessedTotal,
  indexerLagLedgers,
  incrementEventProcessed,
  incrementEventFailed,
  incrementIssuerAttestation,
  incrementIssuerRevocation,
  setIssuerRateLimitRatio,
  issuersTotal,
  EventTypes,
} from "./metrics";
import { dispatchWebhooks } from "./webhooks";
import { scheduleArchivalJob } from "./archival";
import { getTracer } from "./tracing";
import { logger, logProcessedEvent } from "./logger";

const CONTRACT_ID = process.env.CONTRACT_ID!;
const RPC_URL = process.env.RPC_URL ?? "https://soroban-testnet.stellar.org";
const START_LEDGER = process.env.START_LEDGER
  ? parseInt(process.env.START_LEDGER, 10)
  : undefined;
const PAGE_LIMIT = 200;
const POLL_MS = 5_000;
// Maximum number of events processed concurrently within a single page batch.
// Independently-shaped events (different topics, different subjects) can be
// handled in parallel up to this limit without losing ordering guarantees for
// the checkpoint — all events in a page complete before the checkpoint is
// advanced. Defaults to 8; set EVENT_CONCURRENCY env var to override.
const EVENT_CONCURRENCY = process.env.EVENT_CONCURRENCY
  ? Math.max(1, parseInt(process.env.EVENT_CONCURRENCY, 10))
  : 8;

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

import { getLastLedger, setLastLedger } from "./indexer-state";

export async function startIndexer(db: PrismaClient, redis: Redis | null = null): Promise<void> {
  const rpc = new SorobanRpc.Server(RPC_URL, { allowHttp: true });

  // Initialize archival scheduler (runs every 6 hours, configurable)
  const ARCHIVAL_INTERVAL_HOURS = parseInt(
    process.env.ARCHIVAL_INTERVAL_HOURS ?? "6",
    10,
  );
  scheduleArchivalJob(db, ARCHIVAL_INTERVAL_HOURS);

  // ── Backfill ───────────────────────────────────────────────────────────────
  const checkpoint = await db.checkpoint.findUnique({ where: { id: 1 } });
  // START_LEDGER env var overrides stored checkpoint
  let cursor =
    START_LEDGER ?? (checkpoint ? checkpoint.ledger + 1 : GENESIS_LEDGER);

  const { sequence: tip } = await rpc.getLatestLedger();
  if (cursor <= tip) {
    logger.info({ cursor, tip }, "Backfilling ledgers");
    try {
      cursor = await processRange(db, rpc, cursor, tip, redis);
    } catch (err) {
      console.error("Error during backfill:", err);
    }
  }

  // ── Live polling ───────────────────────────────────────────────────────────
  logger.info("Live polling for new events");
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
  const span = getTracer().startSpan("indexer.processRange", {
    attributes: { "ledger.from": from, "ledger.to": to },
  });
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

      // ── Bounded-concurrency pool ────────────────────────────────────────
      // Process up to EVENT_CONCURRENCY events in parallel within each page.
      // All tasks complete before the checkpoint is advanced, preserving
      // exactly-once-per-checkpoint semantics. Failed events are written to
      // the dead-letter table rather than silently dropped.
      const tasks = response.events.map((ev) => async () => {
        const topicStr = ev.topic[0]
          ? (scValToNative(ev.topic[0]) as string)
          : "unknown";
        try {
          await handleEvent(db, ev, redis);
          processedCount++;
          const eventType = normalizeEventType(topicStr);
          if (eventType) {
            incrementEventProcessed(eventType);
          }
          logProcessedEvent({
            eventType: topicStr,
            ledger: ev.ledger,
          });
        } catch (err) {
          logger.error({ err, ledger: ev.ledger, eventType: topicStr }, "failed to process contract event");
          const eventType = normalizeEventType(topicStr);
          if (eventType) {
            incrementEventFailed(eventType);
          }
          await routeToDeadLetter(db, ev, err);
        }
      });

      await runWithConcurrency(tasks, EVENT_CONCURRENCY);

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

    setLastLedger(Math.min(startLedger - 1, to));
  }

  logger.info({ from, to, processedCount }, "Completed processing ledger range");
  span.end();
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

  const span = getTracer().startSpan("indexer.handleEvent", {
    attributes: { "event.topic": topicStr, "ledger.sequence": ev.ledger },
  });
  try {
  eventsProcessedTotal.inc();
  const data = scValToNative(ev.value) as unknown[];

  // ── Multi-sig events ───────────────────────────────────────────────────────

  if (topicStr === "ms_prop") {
    const proposalId = String(data[0]);
    const proposer = String(data[1]);
    const threshold = Number(data[2]);
    const subject = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
    // claimType is not in the event; default to empty string — updated via ms_sign if needed
    const expiresAt = BigInt(Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60);

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
    const signer = String(data[2]);
    const signatureCount = Number(data[1]);
    const existing = await db.multisigProposal.findUnique({
      where: { id: proposalId },
      select: { signers: true },
    });
    if (!existing) return;
    const updatedSigners = existing.signers.includes(signer)
      ? existing.signers
      : [...existing.signers, signer];
    await db.multisigProposal.update({
      where: { id: proposalId },
      data: { signatureCount, signers: updatedSigners },
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

    // Calculate rate limit ratio (attestations / rateLimit)
    const attestationCount = await db.attestation.count({
      where: { issuer: issuerAddr, isRevoked: false },
    });
    const ratio = rateLimit > 0 ? attestationCount / rateLimit : 0;
    setIssuerRateLimitRatio(issuerAddr, ratio);
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
    if (attestation) {
      incrementIssuerRevocation(attestation.issuer);
    }
    dispatchWebhooks(db, "attestation.revoked", { id: attestationId }).catch(
      () => {},
    );

    // Publish to GraphQL subscription
    pubsub.publish(ATTESTATION_REVOKED, {
      onAttestationRevoked: {
        id: attestationId,
        issuer: attestation?.issuer ?? "",
        subject: attestation?.subject ?? "",
        claimType: attestation?.claimType ?? "",
        revokedAt: new Date().toISOString(),
      },
    });
    return;
  }

  // Handle issuer registration events
  if (topicStr === "iss_reg") {
    // data: [issuer_address, name, url, description]
    const issuerAddress = String(data[0]);
    const name = String(data[1]);
    const url = data[2] != null ? String(data[2]) : null;
    const description = data[3] != null ? String(data[3]) : null;

    await db.issuer.upsert({
      where: { address: issuerAddress },
      update: { name, url, description },
      create: {
        address: issuerAddress,
        name,
        url,
        description,
        tier: "basic",
      },
    });

    // Update issuers total count
    const totalIssuers = await db.issuer.count();
    issuersTotal.set(totalIssuers);

    // Publish to GraphQL subscription
    pubsub.publish(ISSUER_REGISTERED, {
      onIssuerRegistered: {
        issuer: issuerAddress,
        registeredAt: new Date().toISOString(),
      },
    });
    return;
  }

  // Handle issuer tier update events
  if (topicStr === "issuer_tier_updated") {
    // data: [issuer_address, new_tier]
    const issuerAddress = String(data[0]);
    const tier = String(data[1]);

    await db.issuer.update({
      where: { address: issuerAddress },
      data: { tier },
    });
    return;
  }

  // ── created | imported | bridged ───────────────────────────────────────────

  const subject = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
  const [id, issuer, claimType, rawTs] = data as [
    string,
    string,
    string,
    bigint | number,
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
  incrementIssuerAttestation(issuer);

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
  } finally {
    span.end();
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

// ── Bounded-concurrency helpers ───────────────────────────────────────────────

/**
 * Runs an array of async task functions with at most `concurrency` running
 * simultaneously. Resolves when all tasks have settled (never rejects —
 * individual task failures must be handled inside each task function).
 */
async function runWithConcurrency(
  tasks: Array<() => Promise<void>>,
  concurrency: number
): Promise<void> {
  const queue = [...tasks];
  const workers: Promise<void>[] = [];

  async function worker(): Promise<void> {
    while (queue.length > 0) {
      const task = queue.shift();
      if (task) await task();
    }
  }

  const workerCount = Math.min(concurrency, tasks.length);
  for (let i = 0; i < workerCount; i++) {
    workers.push(worker());
  }
  await Promise.all(workers);
}

/**
 * Persists a failed event to the event_dead_letters table so it can be
 * inspected and reprocessed by operators rather than silently lost.
 * Errors from this write are swallowed to avoid masking the original failure.
 */
async function routeToDeadLetter(
  db: PrismaClient,
  ev: SorobanRpc.Api.EventResponse,
  err: unknown
): Promise<void> {
  try {
    const error = err instanceof Error ? err : new Error(String(err));
    const topicStr = ev.topic[0]
      ? (scValToNative(ev.topic[0]) as string)
      : "unknown";

    // Serialise the raw event value safely
    let eventDataJson = "{}";
    try {
      eventDataJson = JSON.stringify(scValToNative(ev.value));
    } catch {
      eventDataJson = JSON.stringify({ raw: String(ev.value) });
    }

    await (db as unknown as {
      eventDeadLetter: {
        create: (args: { data: Record<string, unknown> }) => Promise<void>;
      };
    }).eventDeadLetter.create({
      data: {
        ledger: ev.ledger,
        eventType: topicStr,
        contractId: ev.contractId ?? CONTRACT_ID,
        topic0: ev.topic[0] ? String(scValToNative(ev.topic[0])) : null,
        topic1: ev.topic[1] ? String(scValToNative(ev.topic[1])) : null,
        topic2: ev.topic[2] ? String(scValToNative(ev.topic[2])) : null,
        eventDataJson,
        errorMessage: error.message,
        errorStack: error.stack ?? null,
        attemptCount: 1,
        status: "PENDING",
        updatedAt: new Date(),
      },
    });
  } catch (writeErr) {
    console.error("Failed to write event to dead-letter table:", writeErr);
  }
}

// Map raw event topics to normalized event type labels
function normalizeEventType(topic: string): string | null {
  const mapping: Record<string, string> = {
    created: EventTypes.CREATED,
    imported: EventTypes.IMPORTED,
    bridged: EventTypes.BRIDGED,
    revoked: EventTypes.REVOKED,
    renewed: EventTypes.RENEWED,
    updated: EventTypes.UPDATED,
    expired: EventTypes.EXPIRED,
    endorsed: EventTypes.ENDORSED,
    iss_reg: EventTypes.ISSUER_REGISTERED,
    iss_tier: EventTypes.ISSUER_TIER,
    iss_rem: EventTypes.ISSUER_REMOVED,
    clmtype: EventTypes.CLAIM_TYPE,
    ms_prop: EventTypes.MULTISIG_PROPOSED,
    ms_sign: EventTypes.MULTISIG_COSIGNED,
    ms_actv: EventTypes.MULTISIG_ACTIVATED,
    adm_init: EventTypes.ADMIN_INIT,
    adm_xfer: EventTypes.ADMIN_TRANSFER,
  };
  return mapping[topic] ?? null;
}
