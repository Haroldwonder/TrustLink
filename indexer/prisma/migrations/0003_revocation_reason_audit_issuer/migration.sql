-- #776: add revocationReason to Attestation
ALTER TABLE "Attestation" ADD COLUMN "revocationReason" TEXT;

-- #774: AuditEntry table
CREATE TABLE "AuditEntry" (
    "id"            SERIAL PRIMARY KEY,
    "attestationId" TEXT NOT NULL,
    "action"        TEXT NOT NULL,
    "actor"         TEXT NOT NULL,
    "details"       TEXT,
    "ledger"        INTEGER NOT NULL,
    "timestamp"     BIGINT NOT NULL,
    "createdAt"     TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "AuditEntry_attestationId_fkey"
        FOREIGN KEY ("attestationId") REFERENCES "Attestation"("id")
        ON DELETE RESTRICT ON UPDATE CASCADE
);
CREATE INDEX "AuditEntry_attestationId_idx" ON "AuditEntry"("attestationId");

-- #775: Issuer table
CREATE TABLE "Issuer" (
    "address"   TEXT NOT NULL PRIMARY KEY,
    "rateLimit" INTEGER,
    "updatedAt" TIMESTAMP(3) NOT NULL
);
