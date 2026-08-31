import { describe, it, expect } from "vitest";
import {
  verifyAttestationsInSameBundle,
  verifyBundleClaimTypes,
  verifyBundleSize,
} from "../bundleHelpers";
import { AttestationOrigin } from "../types";
import type { Attestation, AttestationBundle } from "../types";

function makeAttestation(overrides: Partial<Attestation> = {}): Attestation {
  return {
    id: "att-1",
    issuer: "issuer-1",
    subject: "subject-1",
    claim_type: "KYC_PASSED",
    timestamp: 0n,
    expiration: null,
    revoked: false,
    metadata: null,
    valid_from: null,
    origin: AttestationOrigin.Native,
    source_chain: null,
    source_tx: null,
    tags: null,
    revocation_reason: null,
    deleted: false,
    bundle_id: "bundle-1",
    ...overrides,
  } as Attestation;
}

describe("verifyAttestationsInSameBundle", () => {
  it("returns null for an empty array", () => {
    expect(verifyAttestationsInSameBundle([])).toBeNull();
  });

  it("returns null when the first attestation has no bundle_id", () => {
    expect(
      verifyAttestationsInSameBundle([makeAttestation({ bundle_id: null })])
    ).toBeNull();
  });

  it("returns the shared bundle id when all attestations match", () => {
    const attestations = [
      makeAttestation({ id: "a", bundle_id: "bundle-1" }),
      makeAttestation({ id: "b", bundle_id: "bundle-1" }),
    ];
    expect(verifyAttestationsInSameBundle(attestations)).toBe("bundle-1");
  });

  it("returns null when attestations belong to different bundles", () => {
    const attestations = [
      makeAttestation({ id: "a", bundle_id: "bundle-1" }),
      makeAttestation({ id: "b", bundle_id: "bundle-2" }),
    ];
    expect(verifyAttestationsInSameBundle(attestations)).toBeNull();
  });
});

function makeBundle(overrides: Partial<AttestationBundle> = {}): AttestationBundle {
  return {
    id: "bundle-1",
    issuer: "issuer-1",
    subject: "subject-1",
    claim_types: ["KYC_PASSED", "AGE_VERIFIED"],
    timestamp: 0n,
    attestation_ids: ["a", "b"],
    all_valid: true,
    ...overrides,
  };
}

describe("verifyBundleClaimTypes", () => {
  it("returns true when claim types match exactly in order", () => {
    const bundle = makeBundle();
    expect(verifyBundleClaimTypes(bundle, ["KYC_PASSED", "AGE_VERIFIED"])).toBe(true);
  });

  it("returns false when lengths differ", () => {
    const bundle = makeBundle();
    expect(verifyBundleClaimTypes(bundle, ["KYC_PASSED"])).toBe(false);
  });

  it("returns false when order differs", () => {
    const bundle = makeBundle();
    expect(verifyBundleClaimTypes(bundle, ["AGE_VERIFIED", "KYC_PASSED"])).toBe(false);
  });
});

describe("verifyBundleSize", () => {
  it("returns true when the bundle has the expected count", () => {
    expect(verifyBundleSize(makeBundle(), 2)).toBe(true);
  });

  it("returns false when the bundle does not have the expected count", () => {
    expect(verifyBundleSize(makeBundle(), 3)).toBe(false);
  });
});
