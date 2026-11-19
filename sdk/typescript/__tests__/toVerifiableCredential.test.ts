/**
 * Unit tests for toVerifiableCredential (Issue #1025).
 *
 * Verifies W3C VC Data Model 1.1 mapping from TrustLink attestations,
 * including issuer / credentialSubject / expirationDate / credentialStatus
 * and documentation of non-equivalent fields under credentialSubject.trustlink.
 */

import type { Attestation } from "../src/types";
import {
  toVerifiableCredential,
  unixSecondsToIso,
  VC_CONTEXT_V1,
  TRUSTLINK_VC_CONTEXT,
  TRUSTLINK_REVOCATION_STATUS_TYPE,
} from "../src/verifiableCredential";

function makeAttestation(overrides: Partial<Attestation> = {}): Attestation {
  return {
    id: "att-001",
    issuer: "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN",
    subject: "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
    claim_type: "KYC_PASSED",
    timestamp: 1_700_000_000n,
    expiration: 1_800_000_000n,
    revoked: false,
    metadata: null,
    valid_from: null,
    origin: "Native",
    source_chain: null,
    source_tx: null,
    tags: null,
    jurisdiction: null,
    revocation_reason: null,
    deleted: false,
    ...overrides,
  };
}

describe("unixSecondsToIso", () => {
  test("converts bigint Unix seconds to ISO-8601 UTC", () => {
    expect(unixSecondsToIso(0n)).toBe("1970-01-01T00:00:00.000Z");
    expect(unixSecondsToIso(1_700_000_000n)).toBe("2023-11-14T22:13:20.000Z");
  });

  test("accepts number input", () => {
    expect(unixSecondsToIso(1_700_000_000)).toBe("2023-11-14T22:13:20.000Z");
  });

  test("throws on non-finite values", () => {
    expect(() => unixSecondsToIso(Number.NaN)).toThrow(RangeError);
  });
});

describe("toVerifiableCredential", () => {
  test("maps core VC fields from a typical attestation", () => {
    const attestation = makeAttestation();
    const vc = toVerifiableCredential(attestation);

    expect(vc["@context"]).toEqual([VC_CONTEXT_V1, TRUSTLINK_VC_CONTEXT]);
    expect(vc.id).toBe("urn:trustlink:attestation:att-001");
    expect(vc.type).toEqual([
      "VerifiableCredential",
      "TrustLinkAttestation",
      "KYC_PASSED",
    ]);
    expect(vc.issuer).toBe(attestation.issuer);
    expect(vc.issuanceDate).toBe("2023-11-14T22:13:20.000Z");
    expect(vc.expirationDate).toBe("2027-01-15T08:00:00.000Z");
    expect(vc.credentialSubject).toEqual({
      id: attestation.subject,
      claimType: "KYC_PASSED",
      trustlink: {
        origin: "Native",
        sourceChain: null,
        sourceTx: null,
        tags: null,
        jurisdiction: null,
        deleted: false,
        attestationId: "att-001",
      },
    });
    expect(vc.credentialStatus).toEqual({
      id: "urn:trustlink:attestation:att-001#status",
      type: TRUSTLINK_REVOCATION_STATUS_TYPE,
      revoked: false,
    });
  });

  test("omits expirationDate when attestation.expiration is null", () => {
    const vc = toVerifiableCredential(makeAttestation({ expiration: null }));
    expect(vc.expirationDate).toBeUndefined();
    expect("expirationDate" in vc).toBe(false);
  });

  test("includes credentialStatus.revoked and revocationReason when revoked", () => {
    const vc = toVerifiableCredential(
      makeAttestation({
        revoked: true,
        revocation_reason: "Subject requested erasure",
      })
    );

    expect(vc.credentialStatus.type).toBe(TRUSTLINK_REVOCATION_STATUS_TYPE);
    expect(vc.credentialStatus.revoked).toBe(true);
    expect(vc.credentialStatus.revocationReason).toBe("Subject requested erasure");
  });

  test("omits revocationReason when not set", () => {
    const vc = toVerifiableCredential(makeAttestation({ revoked: true }));
    expect(vc.credentialStatus.revoked).toBe(true);
    expect(vc.credentialStatus.revocationReason).toBeUndefined();
    expect("revocationReason" in vc.credentialStatus).toBe(false);
  });

  test("maps valid_from onto credentialSubject.validFrom (VC 1.1 has no top-level validFrom)", () => {
    const vc = toVerifiableCredential(
      makeAttestation({ valid_from: 1_700_000_100n })
    );
    expect(vc.credentialSubject.validFrom).toBe("2023-11-14T22:15:00.000Z");
    expect((vc as unknown as Record<string, unknown>).validFrom).toBeUndefined();
  });

  test("includes metadata on credentialSubject when present", () => {
    const vc = toVerifiableCredential(
      makeAttestation({ metadata: '{"level":"enhanced"}' })
    );
    expect(vc.credentialSubject.metadata).toBe('{"level":"enhanced"}');
  });

  test("preserves non-equivalent TrustLink fields under credentialSubject.trustlink", () => {
    const vc = toVerifiableCredential(
      makeAttestation({
        origin: "Bridged",
        source_chain: "ethereum",
        source_tx: "0xabc",
        tags: ["retail", "eu"],
        jurisdiction: "EU",
        deleted: true,
      })
    );

    expect(vc.credentialSubject.trustlink).toEqual({
      origin: "Bridged",
      sourceChain: "ethereum",
      sourceTx: "0xabc",
      tags: ["retail", "eu"],
      jurisdiction: "EU",
      deleted: true,
      attestationId: "att-001",
    });
  });

  test("does not include a proof property (unsigned export)", () => {
    const vc = toVerifiableCredential(makeAttestation());
    expect((vc as unknown as Record<string, unknown>).proof).toBeUndefined();
    expect("proof" in vc).toBe(false);
  });

  test("respects custom context and idPrefix options", () => {
    const vc = toVerifiableCredential(makeAttestation({ id: "xyz" }), {
      context: ["https://example.com/ctx"],
      idPrefix: "https://example.com/credentials/",
    });

    expect(vc["@context"]).toEqual(["https://example.com/ctx"]);
    expect(vc.id).toBe("https://example.com/credentials/xyz");
    expect(vc.credentialStatus.id).toBe(
      "https://example.com/credentials/xyz#status"
    );
  });

  test("produces JSON-serializable output (bigints converted to ISO strings)", () => {
    const vc = toVerifiableCredential(makeAttestation());
    expect(() => JSON.stringify(vc)).not.toThrow();
    const parsed = JSON.parse(JSON.stringify(vc));
    expect(parsed.issuanceDate).toBe(vc.issuanceDate);
    expect(parsed.expirationDate).toBe(vc.expirationDate);
    expect(parsed.credentialSubject.claimType).toBe("KYC_PASSED");
  });

  test("uses claim_type as a VC type entry", () => {
    const vc = toVerifiableCredential(
      makeAttestation({ claim_type: "ACCREDITED_INVESTOR" })
    );
    expect(vc.type).toContain("ACCREDITED_INVESTOR");
    expect(vc.credentialSubject.claimType).toBe("ACCREDITED_INVESTOR");
  });

  test.each([
    ["id", { id: "" }],
    ["issuer", { issuer: "" }],
    ["subject", { subject: "" }],
    ["claim_type", { claim_type: "" }],
  ] as const)("throws when %s is missing", (_field, overrides) => {
    expect(() =>
      toVerifiableCredential(makeAttestation(overrides as Partial<Attestation>))
    ).toThrow(/required/i);
  });

  test("round-trips attestation id through VC id and trustlink extension", () => {
    const attestation = makeAttestation({ id: "round-trip-42" });
    const vc = toVerifiableCredential(attestation);
    expect(vc.id.endsWith(attestation.id)).toBe(true);
    expect(vc.credentialSubject.trustlink.attestationId).toBe(attestation.id);
  });
});
