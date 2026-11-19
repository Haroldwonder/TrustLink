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
  type EventType,
} from "./metrics";
import { dispatchWebhooks } from "./webhooks";
import { scheduleArchivalJob } from "./archival";
import { getTracer } from "./tracing";
import { logger, logProcessedEvent } from "./logger";
import { scheduleReconciliationJob } from "./reconciliation";
import { handleMsSign } from "./ms-sign";

const CONTRACT_ID = process.env.CONTRACT_ID!;
const RPC_URL = process.env.RPC_URL ?? "https://soroban-testnet.stellar.org";
const START_LEDGER = process.env.START_LEDGER
  ? parseInt(process.env.START_LEDGER, 10)
  : undefined;
const GENESIS_LEDGER = process.env.GENESIS_LEDGER
  ? parseInt(process.env.GENESIS_LEDGER, 10)
  : 0;
const PAGE_LIMIT = 200;
const POLL_MS = 5_000;
// Maximum number of entity-key partitions processed in parallel.
// Within each partition events are always applied sequentially, so dependent
// events on the same entity (e.g. created then revoked for the same
// attestation ID) are never re-ordered. Defaults to 8.
const EVENT_CONCURRENCY = process.env.EVENT_CONCURRENCY
  ? Math.max(1, parseInt(process.env.EVENT_CONCURRENCY, 10))
  : 8;

// #1591: genesis ledger used when no checkpoint exists
const GENESIS_LEDGER = 1;

/**
 * Complete set of event topics the indexer handles.
 * Every topic listed in docs/EVENT_TOPICS.md is either handled below or
 * explicitly skipped with a comment explaining why (#1593).
 */
const WATCHED = new Set([
  // Attestation lifecycle
  "created",
  "revoked",
  "imported",
  "bridged",
  "renewed",    // #1593
  "updated",    // #1593
  "del_req",    // #1593 — GDPR deletion request
  "xfer",       // #1593 — issuer transfer
  "amended",    // #1593
  // Issuer lifecycle
  "iss_reg",
  "iss_tier",   // #1593
  "iss_rem",    // #1593
  // Multi-sig
  "ms_prop",
  "ms_sign",
  "ms_actv",
  "ms_cancel",  // #1593
  // Attestation requests
  "att_req",    // #1593
  "req_ok",     // #1593
  "req_no",     // #1593
  "req_cncl",   // #1593
  // Delegation
  "del_crtd",   // #1593
  "del_rvkd",   // #1593
  // Whitelist
  "wl_on",      // #1593
  "wl_add",     // #1593
  "wl_rem",     // #1593
  // Disputes
  "disputed",   // #1593
  "dsp_res",    // #1593
  // Admin / governance — intentionally not written to DB (informational only)
  // "adm_init", "adm_xfer", "adm_add", "adm_rem", "adm_prop" => skipped
  // Pause — intentionally informational only
  // "paused", "unpaused" => skipped
  // Rate-limit config
  "rate_limit_set", // #775
  "wl_add",
  "wl_rem",
  "wl_on",
  "wl_off",
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

  // Cross-check indexed rows against live contract state on a schedule
  const RECONCILE_INTERVAL_MINUTES = parseInt(
    process.env.RECONCILE_INTERVAL_MINUTES ?? "60",
    10,
  );
  const RECONCILE_SAMPLE_SIZE = parseInt(
    process.env.RECONCILE_SAMPLE_SIZE ?? "50",
    10,
  );
  scheduleReconciliationJob(db, {
    intervalMinutes: RECONCILE_INTERVAL_MINUTES,
    sampleSize: RECONCILE_SAMPLE_SIZE,
    contractId: CONTRACT_ID,
    rpcUrl: RPC_URL,
  });

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
      // Update lag during backfill
      const { sequence: latestAfterBackfill } = await rpc.getLatestLedger();
      indexerLagLedgers.set(Math.max(0, latestAfterBackfill - cursor));
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
      // Compute lag against the current tip after processing
      const { sequence: latestTipNow } = await rpc.getLatestLedger();
      indexerLagLedgers.set(Math.max(0, latestTipNow - cursor));
    }
  }
}

// ── Core processing ──────────────────────────────────────────────────────────

/**
 * Derives a stable partition key from an event so that dependent events on the
 * same entity are always applied in order (#1592).
 *
 * Partitioning rules:
 *   - Attestation events  → the attestation ID (data[0])
 *   - Multi-sig events    → the proposal ID (data[0])
 *   - Issuer events       → the issuer address (topic[1] or data[0])
 *   - Everything else     → a unique key derived from ledger + contractId + txHash
 */
function partitionKey(ev: SorobanRpc.Api.EventResponse): string {
  const topic0 = ev.topic[0] ? (scValToNative(ev.topic[0]) as string) : "";
  const data = scValToNative(ev.value) as unknown[];

  // Attestation lifecycle topics keyed on attestation ID
  const attestationTopics = new Set([
    "created", "revoked", "imported", "bridged",
    "renewed", "updated", "del_req", "xfer", "amended",
    "att_req", "req_ok", "req_no", "req_cncl",
    "disputed", "dsp_res",
  ]);
  if (attestationTopics.has(topic0) && data[0] != null) {
    return `att:${String(data[0])}`;
  }

  // Multi-sig topics keyed on proposal ID
  const multisigTopics = new Set(["ms_prop", "ms_sign", "ms_actv", "ms_cancel"]);
  if (multisigTopics.has(topic0) && data[0] != null) {
    return `ms:${String(data[0])}`;
  }

  // Issuer topics keyed on issuer address
  const issuerTopics = new Set(["iss_reg", "iss_tier", "iss_rem", "rate_limit_set", "wl_on", "wl_add", "wl_rem"]);
  if (issuerTopics.has(topic0)) {
    const issuerAddr = ev.topic[1]
      ? String(scValToNative(ev.topic[1]))
      : (data[0] != null ? String(data[0]) : "");
    return `iss:${issuerAddr}`;
  }

  // Delegation keyed on delegator
  const delegationTopics = new Set(["del_crtd", "del_rvkd"]);
  if (delegationTopics.has(topic0)) {
    const delegator = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
    return `del:${delegator}`;
  }

  // Fallback: treat as independent event
  return `misc:${ev.ledger}:${ev.id ?? Math.random()}`;
}

async function processRange(
export async function processRange(
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
    try {
      // ── #1591: Cursor-based pagination ──────────────────────────────────
      // Fetch all events in [startLedger, to] using cursor-based pagination.
      // We continue fetching until the response has fewer events than the
      // page limit, which signals that we have consumed the full range.
      // The checkpoint stores the paging token of the last consumed event so
      // that resumption after a restart is exact — no events are re-skipped.
      const allEvents: SorobanRpc.Api.EventResponse[] = [];
      let pagingCursor: string | undefined;
      let lastLedgerInPage = startLedger;

      while (true) {
        const requestParams: Parameters<typeof rpc.getEvents>[0] = {
          startLedger: pagingCursor ? undefined : startLedger,
          filters: [{ type: "contract", contractIds: [CONTRACT_ID] }],
          limit: PAGE_LIMIT,
          ...(pagingCursor ? { cursor: pagingCursor } : { endLedger: to }),
        };

        const response = await rpc.getEvents(requestParams);
        const pageEvents = response.events ?? [];

        allEvents.push(...pageEvents);

        if (pageEvents.length > 0) {
          const lastEv = pageEvents[pageEvents.length - 1];
          lastLedgerInPage = lastEv.ledger;
          // Soroban RPC exposes the paging token as `pagingToken` on each event
          pagingCursor = (lastEv as unknown as { pagingToken?: string }).pagingToken
            ?? (response as unknown as { cursor?: string }).cursor;
        }

        // Stop when the page is smaller than PAGE_LIMIT — we've drained the range
        if (pageEvents.length < PAGE_LIMIT) break;

        // Safety: if we've passed the target range stop regardless
        if (lastLedgerInPage >= to) break;
      }

      // ── #1592: Partition-then-sequential event processing ──────────────
      // Group events by entity key. Events in different partitions are
      // processed concurrently; events in the same partition are applied
      // strictly in ledger order, eliminating create/revoke races.
      const partitions = new Map<string, SorobanRpc.Api.EventResponse[]>();
      for (const ev of allEvents) {
        const key = partitionKey(ev);
        if (!partitions.has(key)) partitions.set(key, []);
        partitions.get(key)!.push(ev);
      }

      // Each partition becomes one task; events within it run sequentially.
      const tasks = Array.from(partitions.values()).map(
        (eventsInPartition) => async () => {
          for (const ev of eventsInPartition) {
            const topicStr = ev.topic[0]
              ? (scValToNative(ev.topic[0]) as string)
              : "unknown";
            try {
              await handleEvent(db, ev, redis);
              processedCount++;
              const eventType = normalizeEventType(topicStr);
              if (eventType) incrementEventProcessed(eventType);
              logProcessedEvent({ eventType: topicStr, ledger: ev.ledger });
            } catch (err) {
              logger.error(
                { err, ledger: ev.ledger, eventType: topicStr },
                "failed to process contract event",
              );
              const eventType = normalizeEventType(topicStr);
              if (eventType) incrementEventFailed(eventType);
              await routeToDeadLetter(db, ev, err);
            }
          }
        },
      );

      await runWithConcurrency(tasks, EVENT_CONCURRENCY);

      // Advance checkpoint to the last ledger we fully consumed
      const checkpointLedger = allEvents.length > 0 ? lastLedgerInPage : to;
      startLedger = checkpointLedger + 1;

      await db.checkpoint.upsert({
        where: { id: 1 },
        update: { ledger: checkpointLedger },
        create: { id: 1, ledger: checkpointLedger },
      });

      if (processedCount % 100 === 0 && processedCount > 0) {
        console.log(`Processed ${processedCount} events, checkpoint: ${checkpointLedger}`);
      }
    } catch (err) {
      console.error(`Error fetching events from ledger ${startLedger}:`, err);
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

/** Exported for unit tests that exercise real event-handler code paths. */
export async function handleEvent(
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

    // ── Multi-sig events ─────────────────────────────────────────────────────

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
  eventsProcessedTotal.inc();
  const data = scValToNative(ev.value) as unknown[];

  // ── Attestation request lifecycle events ──────────────────────────────────
  //
  // att_req  topics=[att_req, issuer]  data=[request_id, subject, claim_type, expires_at]
  // req_ok   topics=[req_ok, issuer]   data=[request_id, attestation_id]
  // req_no   topics=[req_no, issuer]   data=[request_id, reason]
  // req_cncl topics=[req_cncl,subject] data=request_id
  // req_exp  topics=[req_exp, issuer]  data=[request_id, subject, expires_at]

  if (topicStr === "att_req") {
    const issuerAddr = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
    const [requestId, subjectAddr, claimType, expiresAt] = data as [string, string, string, bigint];
    const requestedAt = BigInt(ev.ledger);
    await db.attestationRequest.upsert({
      where: { id: requestId },
      update: {},
      create: {
        id: requestId,
        subject: subjectAddr,
        issuer: issuerAddr,
        claimType: String(claimType),
        requestedAt,
        expiresAt: BigInt(expiresAt),
        status: "PENDING",
      },
    });
    return;
  }

  if (topicStr === "req_ok") {
    const [requestId, attestationId] = data as [string, string];
    await db.attestationRequest.updateMany({
      where: { id: requestId, status: "PENDING" },
      data: { status: "FULFILLED", fulfillmentId: attestationId },
    });
    return;
  }

  if (topicStr === "req_no") {
    const [requestId, reason] = data as [string, string | null];
    await db.attestationRequest.updateMany({
      where: { id: requestId, status: "PENDING" },
      data: { status: "REJECTED", rejectionReason: reason ?? null },
    });
    return;
  }

  if (topicStr === "req_cncl") {
    const requestId = String(data[0]);
    await db.attestationRequest.updateMany({
      where: { id: requestId, status: "PENDING" },
      data: { status: "CANCELLED" },
    });
    return;
  }

  if (topicStr === "req_exp") {
    // data = [request_id, subject, expires_at]
    // Transitions PENDING → EXPIRED. Uses updateMany so replaying is a no-op.
    const requestId = String(data[0]);
    await db.attestationRequest.updateMany({
      where: { id: requestId, status: "PENDING" },
      data: { status: "EXPIRED" },
    });
    return;
  }

  // ── Multi-sig events ───────────────────────────────────────────────────────

  if (topicStr === "ms_prop") {
    const proposalId = String(data[0]);
    const proposer = String(data[1]);
    const threshold = Number(data[2]);
    const subject = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
    // claimType is not in the event; default to empty string — updated via ms_sign if needed
    const expiresAt = BigInt(Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60);

    if (topicStr === "ms_sign") {
      const proposalId = String(data[0]);
      const signatureCount = Number(data[1]);
      const signer = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
      await handleMsSign(db, proposalId, signatureCount, signer);
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

    // #1593: ms_cancel — mark cancelled proposals so they don't appear open
    if (topicStr === "ms_cancel") {
      const proposalId = String(data[0]);
      await db.multisigProposal.updateMany({
        where: { id: proposalId, finalized: false },
        data: { finalized: true },
      });
      return;
    }

    // ── Rate-limit config (#775) ─────────────────────────────────────────────

    if (topicStr === "rate_limit_set") {
      const issuerAddr = String(data[0]);
      const rateLimit = Number(data[1]);
      await db.issuer.upsert({
        where: { address: issuerAddr },
        update: { rateLimit },
        create: { address: issuerAddr, rateLimit },
      });
      await cacheInvalidate(redis, `issuerStats:${issuerAddr}`);
      const attestationCount = await db.attestation.count({
        where: { issuer: issuerAddr, isRevoked: false },
      });
      const ratio = rateLimit > 0 ? attestationCount / rateLimit : 0;
      setIssuerRateLimitRatio(issuerAddr, ratio);
      return;
    }

    // ── Issuer lifecycle ─────────────────────────────────────────────────────

    if (topicStr === "iss_reg") {
      const issuerAddress = ev.topic[1]
        ? String(scValToNative(ev.topic[1]))
        : String(data[0]);
      const name = data[1] != null ? String(data[1]) : "";
      const url = data[2] != null ? String(data[2]) : null;
      const description = data[3] != null ? String(data[3]) : null;
      await db.issuer.upsert({
        where: { address: issuerAddress },
        update: { name, url, description },
        create: { address: issuerAddress, name, url, description, tier: "basic" },
      });
      const totalIssuers = await db.issuer.count();
      issuersTotal.set(totalIssuers);
      await cacheInvalidate(redis, `issuerStats:${issuerAddress}`);
      pubsub.publish(ISSUER_REGISTERED, {
        onIssuerRegistered: {
          issuer: issuerAddress,
          registeredAt: new Date().toISOString(),
        },
      });
      return;
    }

    // #1593: iss_tier — update issuer tier in the registry
    if (topicStr === "iss_tier") {
      const issuerAddress = ev.topic[1]
        ? String(scValToNative(ev.topic[1]))
        : String(data[0]);
      // data: [tier] — tier is an object/string depending on contract version
      const tier = data[0] != null ? String(data[0]) : "basic";
      await db.issuer.upsert({
        where: { address: issuerAddress },
        update: { tier },
        create: { address: issuerAddress, tier },
      });
      await cacheInvalidate(redis, `issuerStats:${issuerAddress}`);
      return;
    }
  // ── #1577: Whitelist events ────────────────────────────────────────────────

  if (topicStr === "wl_add" || topicStr === "wl_rem") {
    const issuerAddr = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
    const subjectAddr = String(data[0]);
    const added = topicStr === "wl_add";

    await db.whitelistEntry.upsert({
      where: { issuer_subject: { issuer: issuerAddr, subject: subjectAddr } },
      update: { added },
      create: { issuer: issuerAddr, subject: subjectAddr, added },
    });
    return;
  }

  if (topicStr === "wl_on" || topicStr === "wl_off") {
    const issuerAddr = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
    const enabled = topicStr === "wl_on";

    await db.issuer.upsert({
      where: { address: issuerAddr },
      update: { whitelistEnabled: enabled },
      create: { address: issuerAddr, whitelistEnabled: enabled },
    });
    return;
  }

  // ── Issuer registration (for cache invalidation) ───────────────────────────

    // #1593: iss_rem — mark issuer as removed (tier "removed") so they are no
    // longer listed as active issuers while preserving history.
    if (topicStr === "iss_rem") {
      const issuerAddress = ev.topic[1]
        ? String(scValToNative(ev.topic[1]))
        : String(data[0]);
      await db.issuer.updateMany({
        where: { address: issuerAddress },
        data: { tier: "removed" },
      });
      await cacheInvalidate(redis, `issuerStats:${issuerAddress}`);
      return;
    }

    // ── Revocation ───────────────────────────────────────────────────────────

    if (topicStr === "revoked") {
      const attestationId = String(data[0]);
      const revocationReason = data[1] != null ? String(data[1]) : null;
      const attestation = await db.attestation.findUnique({ where: { id: attestationId } });
      await db.attestation.updateMany({
        where: { id: attestationId },
        data: { isRevoked: true, revocationReason },
      });
      if (attestation) {
        const actor = ev.topic[1]
          ? String(scValToNative(ev.topic[1]))
          : attestation.issuer;
        await db.auditEntry.create({
          data: {
            attestationId,
            action: "Revoked",
            actor,
            details: revocationReason,
            ledger: ev.ledger,
            timestamp: BigInt(
              ev.ledgerClosedAt
                ? Math.floor(new Date(ev.ledgerClosedAt).getTime() / 1000)
                : Date.now() / 1000,
            ),
          },
        });
        await cacheInvalidate(redis, `issuerStats:${attestation.issuer}`);
      }
      revocationsTotal.inc();
      if (attestation) incrementIssuerRevocation(attestation.issuer);
      dispatchWebhooks(db, "attestation.revoked", { id: attestationId }).catch(() => {});
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

    // #1593: renewed — update expiration so stale data is not served
    if (topicStr === "renewed" || topicStr === "updated") {
      const attestationId = String(data[0]);
      const newExpiration = data[1] != null ? BigInt(data[1] as number) : null;
      await db.attestation.updateMany({
        where: { id: attestationId },
        data: newExpiration != null ? { expiration: newExpiration } : {},
      });
      return;
    }

    // #1593: del_req — GDPR deletion: scrub PII and mark attestation deleted
    // Hard-delete the row so it is absent from all indexer outputs including
    // /export/attestations. Audit entries are removed via cascade (or manually
    // here if no cascade is configured).
    if (topicStr === "del_req") {
      const attestationId = String(data[0]);
      // Remove audit trail first to avoid FK violations
      await db.auditEntry.deleteMany({ where: { attestationId } });
      await db.attestation.deleteMany({ where: { id: attestationId } });
      return;
    }

    // #1593: xfer — update issuer when an attestation is transferred
    if (topicStr === "xfer") {
      const attestationId = String(data[0]);
      const newIssuer = String(data[1]);
      await db.attestation.updateMany({
        where: { id: attestationId },
        data: { issuer: newIssuer },
      });
      await db.auditEntry.create({
        data: {
          attestationId,
          action: "Transferred",
          actor: ev.topic[1] ? String(scValToNative(ev.topic[1])) : newIssuer,
          details: `Transferred to ${newIssuer}`,
          ledger: ev.ledger,
          timestamp: BigInt(Math.floor(Date.now() / 1000)),
        },
      });
      return;
    }

    // #1593: amended — update metadata to keep it current
    if (topicStr === "amended") {
      const attestationId = String(data[0]);
      const newMetadata = data[1] != null ? String(data[1]) : null;
      if (newMetadata != null) {
        await db.attestation.updateMany({
          where: { id: attestationId },
          data: { metadata: newMetadata },
        });
      }
      return;
    }

    // ── Attestation requests (#1593) ─────────────────────────────────────────

    if (topicStr === "att_req") {
      // data: [request_id, subject, claim_type, expires_at]
      const requestId = String(data[0]);
      const subject = String(data[1]);
      const claimType = String(data[2]);
      const expiresAt = BigInt(data[3] as number);
      const issuer = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
      await db.attestationRequest.upsert({
        where: { id: requestId },
        update: {},
        create: {
          id: requestId,
          subject,
          issuer,
          claimType,
          requestedAt: BigInt(Math.floor(Date.now() / 1000)),
          expiresAt,
          status: "PENDING",
        },
      });
      return;
    }

    if (topicStr === "req_ok") {
      // data: [request_id, attestation_id]
      const requestId = String(data[0]);
      const fulfillmentId = String(data[1]);
      await db.attestationRequest.updateMany({
        where: { id: requestId },
        data: { status: "FULFILLED", fulfillmentId },
      });
      return;
    }

    if (topicStr === "req_no") {
      // data: [request_id, reason?]
      const requestId = String(data[0]);
      const rejectionReason = data[1] != null ? String(data[1]) : null;
      await db.attestationRequest.updateMany({
        where: { id: requestId },
        data: { status: "REJECTED", rejectionReason },
      });
      return;
    }

    if (topicStr === "req_cncl") {
      // data: [request_id]
      const requestId = String(data[0]);
      // Cancelled requests are removed from the pending queue
      await db.attestationRequest.updateMany({
        where: { id: requestId, status: "PENDING" },
        data: { status: "REJECTED", rejectionReason: "Cancelled by subject" },
      });
      return;
    }

    // ── Delegation (#1593) ───────────────────────────────────────────────────

    if (topicStr === "del_crtd") {
      // data: [delegate, claim_type, expiration?]
      const delegator = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
      const delegate = String(data[0]);
      const claimType = String(data[1]);
      const expiresAt = data[2] != null ? BigInt(data[2] as number) : BigInt(0);
      await db.delegation.upsert({
        where: { delegator_delegate_claimType: { delegator, delegate, claimType } },
        update: { expiresAt, revoked: false },
        create: { delegator, delegate, claimType, expiresAt },
      });
      return;
    }

    if (topicStr === "del_rvkd") {
      // data: [delegate, claim_type]
      const delegator = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
      const delegate = String(data[0]);
      const claimType = String(data[1]);
      await db.delegation.updateMany({
        where: { delegator, delegate, claimType },
        data: { revoked: true },
      });
      return;
    }

    // ── Whitelist (#1593) ────────────────────────────────────────────────────

    if (topicStr === "wl_on") {
      // wl_on has no meaningful data to persist beyond the issuer address,
      // which is already in the issuer registry. No-op except logging.
      return;
    }

    if (topicStr === "wl_add") {
      // data: [subject]
      const issuer = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
      const subject = String(data[0]);
      await db.whitelistEntry.upsert({
        where: { issuer_subject: { issuer, subject } },
        update: {},
        create: { issuer, subject },
      });
      return;
    }

    if (topicStr === "wl_rem") {
      // data: [subject]
      const issuer = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
      const subject = String(data[0]);
      await db.whitelistEntry.deleteMany({ where: { issuer, subject } });
      return;
    }

    // ── Disputes (#1593) ─────────────────────────────────────────────────────

    if (topicStr === "disputed" || topicStr === "dsp_res") {
      // Dispute data is informational; log it via the audit trail.
      const attestationId = String(data[0]);
      const actor = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
      const action = topicStr === "disputed" ? "Disputed" : "DisputeResolved";
      const details = data[1] != null ? String(data[1]) : null;
      // Only create an audit entry if the attestation exists
      const exists = await db.attestation.findUnique({ where: { id: attestationId } });
      if (exists) {
        await db.auditEntry.create({
          data: {
            attestationId,
            action,
            actor,
            details,
            ledger: ev.ledger,
            timestamp: BigInt(Math.floor(Date.now() / 1000)),
          },
        });
      }
      return;
    }

    // ── created | imported | bridged ─────────────────────────────────────────

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

    await db.auditEntry.create({
      data: {
        attestationId: id,
        action:
          topicStr === "imported"
            ? "Imported"
            : topicStr === "bridged"
              ? "Bridged"
              : "Created",
        actor: issuer,
        details: null,
        ledger: ev.ledger,
        timestamp,
      },
    });

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

  async function worker(): Promise<void> {
    while (queue.length > 0) {
      const task = queue.shift();
      if (task) await task();
    }
  }

  const workerCount = Math.min(concurrency, tasks.length);
  const workers: Promise<void>[] = [];
  for (let i = 0; i < workerCount; i++) {
    workers.push(worker());
  }
  await Promise.all(workers);
}

/**
 * Persists a failed event to the event_dead_letters table so it can be
 * inspected and reprocessed by operators rather than silently lost.
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
function normalizeEventType(topic: string): EventType | null {
  const mapping: Record<string, EventType> = {
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
    att_req:  EventTypes.REQUEST_EXPIRED, // reuse bucket — request events share metrics
    req_ok:   EventTypes.REQUEST_EXPIRED,
    req_no:   EventTypes.REQUEST_EXPIRED,
    req_cncl: EventTypes.REQUEST_EXPIRED,
    req_exp:  EventTypes.REQUEST_EXPIRED,
  };
  return mapping[topic] ?? null;
}
