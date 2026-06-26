-- CreateEnum
CREATE TYPE "WebhookFailureStatus" AS ENUM ('FAILED', 'RETRYING', 'RECOVERED');

-- CreateTable
CREATE TABLE "webhook_failures" (
    "id" TEXT NOT NULL,
    "webhook_id" TEXT,
    "url" TEXT NOT NULL,
    "event_type" TEXT NOT NULL,
    "payload" TEXT NOT NULL,
    "status_code" INTEGER,
    "error_message" TEXT,
    "attempt_count" INTEGER NOT NULL DEFAULT 5,
    "status" "WebhookFailureStatus" NOT NULL DEFAULT 'FAILED',
    "failed_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "resolved_at" TIMESTAMP(3),
    "updated_at" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "webhook_failures_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "webhook_failures_status_idx" ON "webhook_failures"("status");

-- CreateIndex
CREATE INDEX "webhook_failures_webhook_id_idx" ON "webhook_failures"("webhook_id");

-- CreateIndex
CREATE INDEX "webhook_failures_event_type_idx" ON "webhook_failures"("event_type");

-- CreateIndex
CREATE INDEX "webhook_failures_failed_at_idx" ON "webhook_failures"("failed_at");
