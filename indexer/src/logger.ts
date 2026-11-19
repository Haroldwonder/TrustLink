import pino from "pino";

export const logger = pino({
  level: process.env.LOG_LEVEL ?? "info",
  base: { service: "trustlink-indexer" },
});

/** Fraction of info-level per-event logs to emit (0–1). Default 1 = 100%. */
export function getInfoSampleRate(): number {
  const raw = process.env.LOG_INFO_SAMPLE_RATE ?? "1";
  const parsed = Number.parseFloat(raw);
  if (!Number.isFinite(parsed)) {
    return 1;
  }
  return Math.min(1, Math.max(0, parsed));
}

let randomFn = Math.random;

/** Test hook to make sampling deterministic in unit tests. */
export function setRandomFn(fn: () => number): void {
  randomFn = fn;
}

export function resetRandomFn(): void {
  randomFn = Math.random;
}

export function shouldSampleInfo(rate = getInfoSampleRate()): boolean {
  if (rate >= 1) {
    return true;
  }
  if (rate <= 0) {
    return false;
  }
  return randomFn() < rate;
}

/** Create a child logger with a correlation ID for a single request. */
export function requestLogger(correlationId: string) {
  return logger.child({ correlationId });
}

export interface ProcessedEventLogFields {
  eventType: string;
  ledger: number;
}

/**
 * Info-level per-event log with configurable sampling.
 * Warning/error paths should call `logger.warn` / `logger.error` directly.
 */
export function logProcessedEvent(fields: ProcessedEventLogFields): void {
  if (shouldSampleInfo()) {
    logger.info(fields, "processed contract event");
  }
}
