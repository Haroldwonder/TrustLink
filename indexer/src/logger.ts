import pino from "pino";

export const logger = pino({
  level: process.env.LOG_LEVEL ?? "info",
  base: { service: "trustlink-indexer" },
});

/** Create a child logger with a correlation ID for a single request. */
export function requestLogger(correlationId: string) {
  return logger.child({ correlationId });
}
