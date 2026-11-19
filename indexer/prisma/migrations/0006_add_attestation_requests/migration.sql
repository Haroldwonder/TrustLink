-- CreateEnum
CREATE TYPE "RequestStatus" AS ENUM ('PENDING', 'FULFILLED', 'REJECTED');

-- CreateTable
CREATE TABLE "attestation_requests" (
    "id" TEXT NOT NULL,
    "subject" TEXT NOT NULL,
    "issuer" TEXT NOT NULL,
    "claim_type" TEXT NOT NULL,
    "requested_at" BIGINT NOT NULL,
    "expires_at" BIGINT NOT NULL,
    "status" "RequestStatus" NOT NULL DEFAULT 'PENDING',
    "fulfillment_id" TEXT,
    "rejection_reason" TEXT,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "attestation_requests_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "attestation_requests_issuer_idx" ON "attestation_requests"("issuer");

-- CreateIndex
CREATE INDEX "attestation_requests_subject_idx" ON "attestation_requests"("subject");

-- CreateIndex
CREATE INDEX "attestation_requests_status_idx" ON "attestation_requests"("status");

-- CreateIndex
CREATE INDEX "attestation_requests_issuer_status_idx" ON "attestation_requests"("issuer", "status");
