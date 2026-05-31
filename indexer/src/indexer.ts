import { PrismaClient } from "@prisma/client";
import { rpc as SorobanRpc, scValToNative } from "@stellar/stellar-sdk";
import { pubsub, ATTESTATION_CREATED, ATTESTATION_REVOKED, ISSUER_REGISTERED } from "./graphql";
import {
  attestationsTotal,
  revocationsTotal,
  eventsProcessedTotal,
  indexerLagLedgers,
  incrementEventProcessed,
  incrementEventFailed,
  EventTypes,
} from "./metrics";
import { dispatchWebhooks } from "./webhooks";

const CONTRACT_ID = process.env.CONTRACT_ID!;
const RPC_URL = process.env.RPC_URL ?? "https://soroban-testnet.stellar.org";
const START_LEDGER = process.env.START_LEDGER ? parseInt(process.env.START_LEDGER, 10) : undefined;
const PAGE_LIMIT = 200;
const POLL_MS = 5_000;

const WATCHED = new Set(["created", "revoked", "imported", "bridged", "ms_prop", "ms_sign", "ms_actv", "iss_reg", "issuer_tier_updated", "att_req", "req_ful", "req_rej"]);

let lastLedger = 0;

export function getLastLedger(): number {
  return lastLedger;
}

export async function reindex(db: PrismaClient, fromLedger: number): Promise<void> {
  const rpc = new SorobanRpc.Server(RPC_URL, { allowHttp: true });
  const { sequence: tip } = await rpc.getLatestLedger();
  
  console.log(`Reindexing from ledger ${fromLedger} to ${tip}…`);
  await processRange(db, rpc, fromLedger, tip);
  console.log(`Reindex complete`);
}

export async function startIndexer(db: PrismaClient): Promise<void> {
  const rpc = new SorobanRpc.Server(RPC_URL, { allowHttp: true });

  // ── Backfill ───────────────────────────────────────────────────────────────
  const checkpoint = await db.checkpoint.findUnique({ where: { id: 1 } });
  // START_LEDGER env var overrides stored checkpoint
  let cursor = START_LEDGER ?? (checkpoint ? checkpoint.ledger + 1 : GENESIS_LEDGER);

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
    const { sequence: latest } = await rpc.getLatestLedger();
    if (cursor <= latest) {
      cursor = await processRange(db, rpc, cursor, latest);
      indexerLagLedgers.set(latest - cursor);
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
        const topicStr = ev.topic[0] ? scValToNative(ev.topic[0]) as string : "unknown";
        try {
          await handleEvent(db, ev);
          processedCount++;
          // Track by event type
          const eventType = normalizeEventType(topicStr);
          if (eventType) {
            incrementEventProcessed(eventType);
          }
        } catch (err) {
          console.error(`Error processing event at ledger ${ev.ledger}:`, err);
          // Continue processing other events
          const eventType = normalizeEventType(topicStr);
          if (eventType) {
            incrementEventFailed(eventType);
          }
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

    const lastProcessed =
      response.events.length > 0
        ? response.events[response.events.length - 1].ledger
        : Math.min(startLedger + PAGE_LIMIT - 1, to);

    lastLedger = lastProcessed;
    startLedger = lastProcessed + 1;

    await db.checkpoint.upsert({
      where: { id: 1 },
      update: { ledger: lastProcessed },
      create: { id: 1, ledger: lastProcessed },
    });
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

  eventsProcessedTotal.inc();
  const data = scValToNative(ev.value) as unknown[];

  // Handle multi-sig events
  if (topicStr === "ms_prop") {
    // topics: ["ms_prop", subject_address]  data: (proposal_id, proposer, threshold)
    const proposalId = String(data[0]);
    const proposer = String(data[1]);
    const threshold = Number(data[2]);
    const subject = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
    // claimType is not in the event; default to empty string — updated via ms_sign if needed
    const expiresAt = BigInt(Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60);

    // Idempotent: upsert with no-op on conflict so replays are safe
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
        finalized: false,
        expiresAt,
      },
    });
    return;
  }

  if (topicStr === "ms_sign") {
    // topics: ["ms_sign", signer_address]  data: (proposal_id, signatures_so_far, threshold)
    const proposalId = String(data[0]);
    const signatureCount = Number(data[1]);
    const signer = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";

    // Fetch current signers to append idempotently
    const existing = await db.multisigProposal.findUnique({
      where: { id: proposalId },
      select: { signers: true },
    });
    if (!existing) return; // proposal not yet indexed; skip

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
    // topics: ["ms_actv"]  data: (proposal_id, attestation_id)
    const proposalId = String(data[0]);

    await db.multisigProposal.updateMany({
      where: { id: proposalId, finalized: false },
      data: { finalized: true },
    });
    attestationsTotal.inc();
    return;
  }

  if (topicStr === "att_req") {
    // topics: ["att_req", subject_address]  data: (request_id, issuer, claim_type, requested_at, expires_at)
    const subject = ev.topic[1] ? String(scValToNative(ev.topic[1])) : "";
    const [requestId, issuer, claimType, rawRequestedAt, rawExpiresAt] = data as [string, string, string, bigint | number, bigint | number];
    await db.attestationRequest.upsert({
      where: { id: String(requestId) },
      update: {},
      create: {
        id: String(requestId),
        subject,
        issuer: String(issuer),
        claimType: String(claimType),
        requestedAt: BigInt(rawRequestedAt),
        expiresAt: BigInt(rawExpiresAt),
        status: "PENDING",
      },
    });
    return;
  }

  if (topicStr === "req_ful") {
    // topics: ["req_ful", issuer_address]  data: (request_id, attestation_id)
    const [requestId, attestationId] = data as [string, string];
    await db.attestationRequest.updateMany({
      where: { id: String(requestId), status: "PENDING" },
      data: { status: "FULFILLED", fulfillmentId: String(attestationId) },
    });
    return;
  }

  if (topicStr === "req_rej") {
    // topics: ["req_rej", issuer_address]  data: (request_id, rejection_reason?)
    const [requestId, rawReason] = data as [string, string | null | undefined];
    const rejectionReason = rawReason != null ? String(rawReason) : null;
    await db.attestationRequest.updateMany({
      where: { id: String(requestId), status: "PENDING" },
      data: { status: "REJECTED", rejectionReason },
    });
    return;
  }

  if (topicStr === "revoked") {
    const attestationId = String(data[0]);
    const attestation = await db.attestation.findUnique({
      where: { id: attestationId },
    });
    
    await db.attestation.updateMany({
      where: { id: attestationId },
      data: { isRevoked: true },
    });
    revocationsTotal.inc();
    dispatchWebhooks(db, "attestation.revoked", { id: attestationId }).catch(() => {});

    // Publish to GraphQL subscription
    pubsub.publish(ATTESTATION_REVOKED, {
      onAttestationRevoked: {
        id: attestationId,
        issuer: attestation?.issuer ?? "",
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

  attestationsTotal.inc();

  // Dispatch webhooks for new attestation events
  dispatchWebhooks(db, `attestation.${topicStr}`, {
    ...attestation,
    timestamp: String(attestation.timestamp),
    expiration: attestation.expiration != null ? String(attestation.expiration) : null,
  }).catch(() => {});

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

// Map raw event topics to normalized event type labels
function normalizeEventType(topic: string): string | null {
  const mapping: Record<string, string> = {
    "created": EventTypes.CREATED,
    "imported": EventTypes.IMPORTED,
    "bridged": EventTypes.BRIDGED,
    "revoked": EventTypes.REVOKED,
    "renewed": EventTypes.RENEWED,
    "updated": EventTypes.UPDATED,
    "expired": EventTypes.EXPIRED,
    "endorsed": EventTypes.ENDORSED,
    "iss_reg": EventTypes.ISSUER_REGISTERED,
    "iss_tier": EventTypes.ISSUER_TIER,
    "iss_rem": EventTypes.ISSUER_REMOVED,
    "clmtype": EventTypes.CLAIM_TYPE,
    "ms_prop": EventTypes.MULTISIG_PROPOSED,
    "ms_sign": EventTypes.MULTISIG_COSIGNED,
    "ms_actv": EventTypes.MULTISIG_ACTIVATED,
    "adm_init": EventTypes.ADMIN_INIT,
    "adm_xfer": EventTypes.ADMIN_TRANSFER,
  };
  return mapping[topic] ?? null;
}
