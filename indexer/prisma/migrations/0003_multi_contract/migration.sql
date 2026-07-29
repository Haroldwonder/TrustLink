-- Migration: 0003_multi_contract
-- Extend schema to support multiple contract IDs.
--
-- All attestation and proposal records are now keyed by contractId so a single
-- indexer deployment can track several TrustLink instances simultaneously.

-- 1. Add contractId column with a temporary default so existing rows get a
--    value.  The default is cleared after backfill to enforce NOT NULL.
ALTER TABLE "Attestation" ADD COLUMN "contractId" TEXT;

-- Backfill existing rows using the CONTRACT_ID env var that was hard-coded
-- before this migration. Operators must set LEGACY_CONTRACT_ID to the value
-- that was in use before upgrading.  If unset we fall back to an empty string
-- so the migration succeeds and operators can UPDATE manually.
UPDATE "Attestation"
  SET "contractId" = COALESCE(current_setting('app.legacy_contract_id', true), '');

-- Make the column required once backfilled.
ALTER TABLE "Attestation" ALTER COLUMN "contractId" SET NOT NULL;

ALTER TABLE "MultisigProposal" ADD COLUMN "contractId" TEXT;

UPDATE "MultisigProposal"
  SET "contractId" = COALESCE(current_setting('app.legacy_contract_id', true), '');

ALTER TABLE "MultisigProposal" ALTER COLUMN "contractId" SET NOT NULL;

-- 2. Drop the old singleton Checkpoint row (id=1) and replace with a per-
--    contract table keyed by contractId.
DROP TABLE IF EXISTS "Checkpoint";

CREATE TABLE "Checkpoint" (
    "contractId" TEXT NOT NULL,
    "ledger"     INTEGER NOT NULL,
    CONSTRAINT "Checkpoint_pkey" PRIMARY KEY ("contractId")
);

-- Seed a checkpoint for the legacy contract if one was in use.
INSERT INTO "Checkpoint" ("contractId", "ledger")
  SELECT COALESCE(current_setting('app.legacy_contract_id', true), ''), 0
  WHERE COALESCE(current_setting('app.legacy_contract_id', true), '') <> ''
  ON CONFLICT DO NOTHING;

-- 3. Add indexes for the new contractId columns.
CREATE INDEX "Attestation_contractId_idx"         ON "Attestation"("contractId");
CREATE INDEX "Attestation_contractId_subject_idx"  ON "Attestation"("contractId", "subject");
CREATE INDEX "Attestation_contractId_issuer_idx"   ON "Attestation"("contractId", "issuer");
CREATE INDEX "Attestation_contractId_claimType_idx" ON "Attestation"("contractId", "claimType");

CREATE INDEX "MultisigProposal_contractId_idx" ON "MultisigProposal"("contractId");
