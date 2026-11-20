-- Issue #1706: Let webhook subscribers choose which event types they receive
ALTER TABLE "Webhook" ADD COLUMN "eventTypes" TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[];
