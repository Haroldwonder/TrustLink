-- Add indexes for common query patterns
CREATE INDEX "Attestation_claimType_idx" ON "Attestation"("claimType");
CREATE INDEX "Attestation_subject_claimType_idx" ON "Attestation"("subject", "claimType");
CREATE INDEX "Attestation_timestamp_idx" ON "Attestation"("timestamp");
