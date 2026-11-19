-- Migration: 0008_add_event_dead_letter
-- Adds the event_dead_letters table for persisting per-event processing failures.
-- Events that throw during handleEvent are routed here instead of being silently lost.

CREATE TYPE "DeadLetterStatus" AS ENUM ('PENDING', 'RETRYING', 'RESOLVED', 'ABANDONED');

CREATE TABLE "event_dead_letters" (
    "id"             TEXT         NOT NULL,
    "ledger"         INTEGER      NOT NULL,
    "eventType"      TEXT         NOT NULL,
    "contractId"     TEXT         NOT NULL,
    "topic0"         TEXT,
    "topic1"         TEXT,
    "topic2"         TEXT,
    "eventDataJson"  TEXT         NOT NULL,
    "errorMessage"   TEXT         NOT NULL,
    "errorStack"     TEXT,
    "attemptCount"   INTEGER      NOT NULL DEFAULT 1,
    "status"         "DeadLetterStatus" NOT NULL DEFAULT 'PENDING',
    "failedAt"       TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "resolvedAt"     TIMESTAMP(3),
    "updatedAt"      TIMESTAMP(3) NOT NULL,

    CONSTRAINT "event_dead_letters_pkey" PRIMARY KEY ("id")
);

CREATE INDEX "event_dead_letters_status_idx"     ON "event_dead_letters"("status");
CREATE INDEX "event_dead_letters_ledger_idx"     ON "event_dead_letters"("ledger");
CREATE INDEX "event_dead_letters_eventType_idx"  ON "event_dead_letters"("eventType");
CREATE INDEX "event_dead_letters_contractId_idx" ON "event_dead_letters"("contractId");
CREATE INDEX "event_dead_letters_failedAt_idx"   ON "event_dead_letters"("failedAt");
