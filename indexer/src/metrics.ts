import { register, Counter, Gauge } from "prom-client";

export const attestationsTotal = new Counter({
  name: "trustlink_attestations_total",
  help: "Total number of attestations created",
});

export const revocationsTotal = new Counter({
  name: "trustlink_revocations_total",
  help: "Total number of attestations revoked",
});

export const issuersTotal = new Gauge({
  name: "trustlink_issuers_total",
  help: "Current number of registered issuers",
});

export const eventsProcessedTotal = new Counter({
  name: "trustlink_events_processed_total",
  help: "Total number of events processed",
});

export const eventsProcessedByType = new Counter({
  name: "trustlink_events_processed_total",
  help: "Total number of events processed by type",
  labelNames: ["type"],
});

export const eventsFailedTotal = new Counter({
  name: "trustlink_events_failed_total",
  help: "Total number of events that failed to process",
  labelNames: ["type"],
});

export const indexerLagLedgers = new Gauge({
  name: "trustlink_indexer_lag_ledgers",
  help: "Number of ledgers behind the tip",
});

// Event type constants matching TrustLink contract topics
export const EventTypes = {
  CREATED: "created",
  IMPORTED: "imported",
  BRIDGED: "bridged",
  REVOKED: "revoked",
  RENEWED: "renewed",
  UPDATED: "updated",
  EXPIRED: "expired",
  ENDORSED: "endorsed",
  ISSUER_REGISTERED: "iss_reg",
  ISSUER_TIER: "iss_tier",
  ISSUER_REMOVED: "iss_rem",
  CLAIM_TYPE: "clmtype",
  MULTISIG_PROPOSED: "ms_prop",
  MULTISIG_COSIGNED: "ms_sign",
  MULTISIG_ACTIVATED: "ms_actv",
  ADMIN_INIT: "adm_init",
  ADMIN_TRANSFER: "adm_xfer",
} as const;

export type EventType = (typeof EventTypes)[keyof typeof EventTypes];

/**
 * Increment the event processed counter for a specific event type.
 */
export function incrementEventProcessed(type: EventType): void {
  eventsProcessedByType.inc({ type });
}

/**
 * Increment the event failed counter for a specific event type.
 */
export function incrementEventFailed(type: EventType): void {
  eventsFailedTotal.inc({ type });
}
// Per-issuer counters used by the issuer activity dashboard (#823) and the
// TrustLinkIssuerRevocationSpike alert (#824).
export const issuerAttestationsTotal = new Counter({
  name: "trustlink_issuer_attestations_total",
  help: "Total attestations created per issuer",
  labelNames: ["issuer"],
});

export const issuerRevocationsTotal = new Counter({
  name: "trustlink_issuer_revocations_total",
  help: "Total attestations revoked per issuer",
  labelNames: ["issuer"],
});

export const issuerRateLimitRatio = new Gauge({
  name: "trustlink_issuer_rate_limit_ratio",
  help: "Fraction of per-issuer attestation capacity consumed (0–1)",
  labelNames: ["issuer"],
});

export function incrementIssuerAttestation(issuer: string): void {
  issuerAttestationsTotal.inc({ issuer });
}

export function incrementIssuerRevocation(issuer: string): void {
  issuerRevocationsTotal.inc({ issuer });
}

export function setIssuerRateLimitRatio(issuer: string, ratio: number): void {
  issuerRateLimitRatio.set({ issuer }, ratio);
}

export async function getMetrics(): Promise<string> {
  return register.metrics();
}