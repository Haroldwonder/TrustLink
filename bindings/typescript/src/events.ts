/**
 * TrustLink Event Topics and Event Subscription Utilities
 *
 * Provides canonical event topic definitions, type-safe event subscriptions,
 * and helpers for filtering contract events via the indexer or direct ledger watching.
 */

/**
 * Canonical event topic constants matching TrustLink contract.
 * All topics are ≤9 characters as required by Soroban symbol_short!().
 */
export const EventTopics = {
  // ─── Attestation Lifecycle ────────────────────────────────────────────────────
  CREATED: "created",
  IMPORTED: "imported",
  BRIDGED: "bridged",
  REVOKED: "revoked",
  RENEWED: "renewed",
  UPDATED: "updated",
  EXPIRED: "expired",
  ENDORSED: "endorsed",
  AMENDED: "amended",
  DEL_REQ: "del_req",
  XFER: "xfer",
  ATT_XFER: "att_xfer",

  // ─── Issuer Lifecycle ──────────────────────────────────────────────────────────
  ISS_REG: "iss_reg",
  ISS_TIER: "iss_tier",
  ISS_REM: "iss_rem",

  // ─── Admin & Governance ────────────────────────────────────────────────────────
  ADM_INIT: "adm_init",
  ADM_XFER: "adm_xfer",
  ADM_ADD: "adm_add",
  ADM_REM: "adm_rem",
  ADM_PROP: "adm_prop",

  // ─── Compliance & Requests ────────────────────────────────────────────────────
  CLM_TYPE: "clm_type",
  ATT_REQ: "att_req",
  REQ_OK: "req_ok",
  REQ_NO: "req_no",
  REQ_CNCL: "req_cncl",
  DEL_CRTD: "del_crtd",
  DEL_RVKD: "del_rvkd",
  WL_ON: "wl_on",
  WL_ADD: "wl_add",
  WL_REM: "wl_rem",

  // ─── Multi-Sig & Proposals ────────────────────────────────────────────────────
  MS_PROP: "ms_prop",
  MS_SIGN: "ms_sign",
  MS_ACTV: "ms_actv",
  MS_CANCEL: "ms_cancel",

  // ─── Dispute & Amendment ──────────────────────────────────────────────────────
  DISPUTED: "disputed",
  DSP_RES: "dsp_res",

  // ─── Pause & Lifecycle ────────────────────────────────────────────────────────
  PAUSED: "paused",
  UNPAUSED: "unpaused",

  // ─── Templates ────────────────────────────────────────────────────────────────
  TMPL_CRT: "tmpl_crt",
  TPL_DEL: "tpl_del",

  // ─── Council Governance ───────────────────────────────────────────────────────
  CNCL_INI: "cncl_ini",
  PROP_NEW: "prop_new",
  PROP_OK: "prop_ok",
  PROP_EXE: "prop_exe",
  TL_START: "tl_start",
  EXP_HOOK: "exp_hook",
} as const;

/** Union type of all valid event topics */
export type EventTopic = (typeof EventTopics)[keyof typeof EventTopics];

/**
 * Event topic categories for convenient subscription grouping.
 * Use these to subscribe to related events by category.
 */
export const EventCategories = {
  /** Attestation lifecycle: created, imported, bridged, revoked, renewed, updated, expired, endorsed, amended, xfer */
  ATTESTATION_LIFECYCLE: [
    EventTopics.CREATED,
    EventTopics.IMPORTED,
    EventTopics.BRIDGED,
    EventTopics.REVOKED,
    EventTopics.RENEWED,
    EventTopics.UPDATED,
    EventTopics.EXPIRED,
    EventTopics.ENDORSED,
    EventTopics.AMENDED,
    EventTopics.XFER,
  ] as const,

  /** Issuer compliance: iss_reg, iss_tier, iss_rem, wl_on, wl_add, wl_rem, del_crtd, del_rvkd */
  ISSUER_COMPLIANCE: [
    EventTopics.ISS_REG,
    EventTopics.ISS_TIER,
    EventTopics.ISS_REM,
    EventTopics.WL_ON,
    EventTopics.WL_ADD,
    EventTopics.WL_REM,
    EventTopics.DEL_CRTD,
    EventTopics.DEL_RVKD,
  ] as const,

  /** Request processing: att_req, req_ok, req_no, req_cncl */
  REQUEST_LIFECYCLE: [
    EventTopics.ATT_REQ,
    EventTopics.REQ_OK,
    EventTopics.REQ_NO,
    EventTopics.REQ_CNCL,
  ] as const,

  /** Multi-sig operations: ms_prop, ms_sign, ms_actv, ms_cancel */
  MULTISIG: [
    EventTopics.MS_PROP,
    EventTopics.MS_SIGN,
    EventTopics.MS_ACTV,
    EventTopics.MS_CANCEL,
  ] as const,

  /** Disputes and amendments: disputed, dsp_res, amended */
  DISPUTE_AMENDMENT: [EventTopics.DISPUTED, EventTopics.DSP_RES, EventTopics.AMENDED] as const,

  /** Administrative actions: adm_init, adm_xfer, adm_add, adm_rem, adm_prop, paused, unpaused */
  ADMIN_ACTIONS: [
    EventTopics.ADM_INIT,
    EventTopics.ADM_XFER,
    EventTopics.ADM_ADD,
    EventTopics.ADM_REM,
    EventTopics.ADM_PROP,
    EventTopics.PAUSED,
    EventTopics.UNPAUSED,
  ] as const,

  /** Council governance: cncl_ini, prop_new, prop_ok, prop_exe, tl_start */
  COUNCIL_GOVERNANCE: [
    EventTopics.CNCL_INI,
    EventTopics.PROP_NEW,
    EventTopics.PROP_OK,
    EventTopics.PROP_EXE,
    EventTopics.TL_START,
  ] as const,

  /** Indexed events (real-time in indexer): created, revoked, imported, bridged, ms_prop, ms_sign, ms_actv, iss_reg */
  INDEXED_PRIORITY: [
    EventTopics.CREATED,
    EventTopics.REVOKED,
    EventTopics.IMPORTED,
    EventTopics.BRIDGED,
    EventTopics.MS_PROP,
    EventTopics.MS_SIGN,
    EventTopics.MS_ACTV,
    EventTopics.ISS_REG,
  ] as const,

  /** All topics */
  ALL: Object.values(EventTopics),
} as const;

/**
 * Options for event subscriptions.
 * Allows filtering by topic and entity identifiers.
 */
export interface EventSubscriptionOptions {
  /** Event topics to subscribe to. If empty, subscribes to all topics. */
  topics?: EventTopic[];

  /** Filter by subject address (for events with subject as indexed topic) */
  subject?: string;

  /** Filter by issuer address (for events with issuer as indexed topic) */
  issuer?: string;

  /** Polling interval in milliseconds for direct ledger watching (default: 5000) */
  pollingIntervalMs?: number;

  /** Maximum number of events to fetch per poll (default: 100) */
  pageSize?: number;

  /** Starting ledger for direct ledger watching (optional) */
  startLedger?: number;
}

/**
 * Validates that all provided topics are valid event topics.
 *
 * @param topics - Array of topics to validate
 * @throws Error if any topic is invalid
 * @returns The validated topics array
 */
export function validateEventTopics(topics: unknown[]): EventTopic[] {
  const validTopics = new Set(Object.values(EventTopics));

  for (const topic of topics) {
    if (typeof topic !== "string" || !validTopics.has(topic as EventTopic)) {
      throw new Error(`Invalid event topic: "${topic}". See EventTopics for valid values.`);
    }
  }

  return topics as EventTopic[];
}

/**
 * Normalizes event topic constants to their string representation.
 * Useful for working with raw ledger event data where topics are strings.
 *
 * @param raw - Raw topic string from ledger event
 * @returns Normalized topic, or null if not recognized
 */
export function normalizeEventTopic(raw: string): EventTopic | null {
  const normalized = raw.toLowerCase().trim();
  const validTopics = Object.values(EventTopics);
  return validTopics.includes(normalized as EventTopic) ? (normalized as EventTopic) : null;
}

/**
 * Returns all topics in a given category.
 *
 * @param category - The category key from EventCategories
 * @returns Array of event topics in that category
 */
export function getTopicsInCategory(category: keyof typeof EventCategories): EventTopic[] {
  const topics = EventCategories[category];
  return Array.isArray(topics) ? (topics as EventTopic[]) : [topics as EventTopic];
}

/**
 * Returns a human-readable description of an event topic.
 * Useful for logging, debugging, and UI display.
 *
 * @param topic - The event topic
 * @returns Description of what this topic represents
 */
export function getTopicDescription(topic: EventTopic): string {
  const descriptions: Record<EventTopic, string> = {
    [EventTopics.CREATED]: "Attestation created",
    [EventTopics.IMPORTED]: "Attestation imported from external source",
    [EventTopics.BRIDGED]: "Attestation bridged from another blockchain",
    [EventTopics.REVOKED]: "Attestation revoked by issuer",
    [EventTopics.RENEWED]: "Attestation expiration renewed",
    [EventTopics.UPDATED]: "Attestation updated",
    [EventTopics.EXPIRED]: "Attestation expired",
    [EventTopics.ENDORSED]: "Attestation endorsed",
    [EventTopics.AMENDED]: "Attestation metadata amended",
    [EventTopics.DEL_REQ]: "Deletion requested",
    [EventTopics.XFER]: "Attestation issuer transferred",
    [EventTopics.ATT_XFER]: "Attestation transferred",
    [EventTopics.ISS_REG]: "Issuer registered",
    [EventTopics.ISS_TIER]: "Issuer tier updated",
    [EventTopics.ISS_REM]: "Issuer removed",
    [EventTopics.ADM_INIT]: "Admin initialized",
    [EventTopics.ADM_XFER]: "Admin transferred",
    [EventTopics.ADM_ADD]: "Admin added",
    [EventTopics.ADM_REM]: "Admin removed",
    [EventTopics.ADM_PROP]: "Admin transfer proposed",
    [EventTopics.CLM_TYPE]: "Claim type registered",
    [EventTopics.ATT_REQ]: "Attestation requested",
    [EventTopics.REQ_OK]: "Request fulfilled",
    [EventTopics.REQ_NO]: "Request rejected",
    [EventTopics.REQ_CNCL]: "Request cancelled",
    [EventTopics.DEL_CRTD]: "Delegation created",
    [EventTopics.DEL_RVKD]: "Delegation revoked",
    [EventTopics.WL_ON]: "Whitelist mode enabled",
    [EventTopics.WL_ADD]: "Subject added to whitelist",
    [EventTopics.WL_REM]: "Subject removed from whitelist",
    [EventTopics.MS_PROP]: "Multi-sig proposal created",
    [EventTopics.MS_SIGN]: "Multi-sig proposal cosigned",
    [EventTopics.MS_ACTV]: "Multi-sig proposal activated",
    [EventTopics.MS_CANCEL]: "Multi-sig proposal cancelled",
    [EventTopics.DISPUTED]: "Dispute raised",
    [EventTopics.DSP_RES]: "Dispute resolved",
    [EventTopics.PAUSED]: "Contract paused",
    [EventTopics.UNPAUSED]: "Contract unpaused",
    [EventTopics.TMPL_CRT]: "Template created",
    [EventTopics.TPL_DEL]: "Template deleted",
    [EventTopics.CNCL_INI]: "Council initialized",
    [EventTopics.PROP_NEW]: "Council proposal created",
    [EventTopics.PROP_OK]: "Council proposal approved",
    [EventTopics.PROP_EXE]: "Council proposal executed",
    [EventTopics.TL_START]: "Timelock started",
    [EventTopics.EXP_HOOK]: "Expiration hook triggered",
  };

  return descriptions[topic] || "Unknown event";
}

/**
 * Type guard for event topics - narrows type from string to EventTopic.
 *
 * @param value - Value to check
 * @returns True if value is a valid EventTopic
 */
export function isEventTopic(value: unknown): value is EventTopic {
  if (typeof value !== "string") return false;
  return Object.values(EventTopics).includes(value as EventTopic);
}
