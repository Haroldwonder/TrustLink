import { TrustLinkClient } from "@trustlink/sdk";
import type { ConformanceContext, ConformanceObservation } from "../types";
import { normalizeAttestationIds, parseErrorObservation } from "../normalize";

export async function collectSdkObservations(
  ctx: ConformanceContext,
): Promise<ConformanceObservation[]> {
  const client = new TrustLinkClient({
    contractId: ctx.contractId,
    network: "local",
    rpcUrl: ctx.rpcUrl,
  });

  const observations: ConformanceObservation[] = [
    {
      step: "has_valid_claim_existing",
      kind: "boolean",
      value: await client.hasValidClaim(ctx.subject, "KYC_PASSED"),
    },
    {
      step: "has_valid_claim_missing",
      kind: "boolean",
      value: await client.hasValidClaim(ctx.subject, "ACCREDITED_INVESTOR"),
    },
    {
      step: "get_subject_attestations",
      kind: "ids",
      value: normalizeAttestationIds(await client.getSubjectAttestations(ctx.subject, 0, 10)),
    },
  ];

  try {
    await client.getAttestation("nonexistent-conformance-id");
    throw new Error("expected get_attestation to fail");
  } catch (error) {
    observations.push(parseErrorObservation("get_attestation_not_found", error));
  }

  return observations;
}
