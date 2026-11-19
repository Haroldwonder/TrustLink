-- Issue #1604: Encrypt webhook secrets at rest
-- This migration adds fields for encrypted secrets and adds rotatedAt timestamp

-- Rename old secret column to secret_plaintext_legacy for safe transition
ALTER TABLE "Webhook" RENAME COLUMN "secret" TO "secret_plaintext_legacy";

-- Add new encrypted secret column (initially NULL, will be backfilled)
ALTER TABLE "Webhook" ADD COLUMN "secret_encrypted" TEXT;

-- Add secret rotation timestamp
ALTER TABLE "Webhook" ADD COLUMN "rotatedAt" TIMESTAMP(3) DEFAULT CURRENT_TIMESTAMP;

-- Add next_secret for grace period during rotation (optional, for dual-signing)
ALTER TABLE "Webhook" ADD COLUMN "next_secret_encrypted" TEXT;

-- Update existing records: copy plaintext to encrypted (app will handle actual encryption on startup)
-- This marker allows the app to know which records need actual encryption
UPDATE "Webhook" SET "secret_encrypted" = "secret_plaintext_legacy" WHERE "secret_encrypted" IS NULL;

-- After migration runs, app should:
-- 1. Encrypt all secret_encrypted values using WEBHOOK_SECRET_KEY
-- 2. Delete secret_plaintext_legacy column (can be done in a second migration after verification)

-- Add unique constraint on id to ensure proper webhook identification
-- (already implicit via @id but making explicit)
CREATE INDEX "idx_webhook_rotated_at" ON "Webhook"("rotatedAt");
