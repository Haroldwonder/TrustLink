import { createHmac } from "crypto";
import {
  verifyWebhookSignature,
  TRUSTLINK_SIGNATURE_HEADER,
} from "../src/webhooks";

describe("verifyWebhookSignature", () => {
  const SECRET = "whsec_test_secret_key_1234567890abcdef";
  const PAYLOAD = JSON.stringify({
    event: "attestation_created",
    data: {
      attestationId: "att_1234567890",
      subject: "GDD4AYCIMALJPW2DPPM45Z2EEIS5X4PPH5ZPWQZY7EISHUZVDOFKZ2UH",
      claimType: "KYC_PASSED",
    },
    ts: 1700000000000,
  });

  function computeValidSignature(secret: string, body: string): string {
    return createHmac("sha256", secret).update(body).digest("hex");
  }

  const VALID_SIGNATURE = computeValidSignature(SECRET, PAYLOAD);

  test("verifies valid signature when passed directly as signature string", () => {
    expect(verifyWebhookSignature(SECRET, PAYLOAD, VALID_SIGNATURE)).toBe(true);
  });

  test("verifies valid signature with Buffer payload", () => {
    const bufferPayload = Buffer.from(PAYLOAD, "utf8");
    expect(verifyWebhookSignature(SECRET, bufferPayload, VALID_SIGNATURE)).toBe(true);
  });

  test("verifies valid signature with Uint8Array payload", () => {
    const uint8Payload = new TextEncoder().encode(PAYLOAD);
    expect(verifyWebhookSignature(SECRET, uint8Payload, VALID_SIGNATURE)).toBe(true);
  });

  test("extracts signature case-insensitively from standard headers object", () => {
    // lowercase
    expect(
      verifyWebhookSignature(SECRET, PAYLOAD, {
        "x-trustlink-signature": VALID_SIGNATURE,
      })
    ).toBe(true);

    // standard canonical casing
    expect(
      verifyWebhookSignature(SECRET, PAYLOAD, {
        "X-TrustLink-Signature": VALID_SIGNATURE,
      })
    ).toBe(true);

    // uppercase
    expect(
      verifyWebhookSignature(SECRET, PAYLOAD, {
        "X-TRUSTLINK-SIGNATURE": VALID_SIGNATURE,
      })
    ).toBe(true);
  });

  test("handles array header values from Node http", () => {
    expect(
      verifyWebhookSignature(SECRET, PAYLOAD, {
        "x-trustlink-signature": [VALID_SIGNATURE],
      })
    ).toBe(true);
  });

  test("handles Fetch API Headers object with .get()", () => {
    const mockHeaders = {
      get: (name: string) => {
        if (name.toLowerCase() === TRUSTLINK_SIGNATURE_HEADER) {
          return VALID_SIGNATURE;
        }
        return null;
      },
    };

    expect(verifyWebhookSignature(SECRET, PAYLOAD, mockHeaders)).toBe(true);
  });

  test("supports custom header name via options", () => {
    const customHeader = "x-custom-signature";
    expect(
      verifyWebhookSignature(
        SECRET,
        PAYLOAD,
        { [customHeader]: VALID_SIGNATURE },
        { headerName: customHeader }
      )
    ).toBe(true);
  });

  test("rejects tampered payload", () => {
    const tamperedPayload = JSON.stringify({
      ...JSON.parse(PAYLOAD),
      event: "attestation_revoked",
    });

    expect(verifyWebhookSignature(SECRET, tamperedPayload, VALID_SIGNATURE)).toBe(false);
  });

  test("rejects incorrect secret", () => {
    const wrongSecret = "whsec_wrong_secret_key_9999999999";
    expect(verifyWebhookSignature(wrongSecret, PAYLOAD, VALID_SIGNATURE)).toBe(false);
  });

  test("rejects missing signature header", () => {
    expect(verifyWebhookSignature(SECRET, PAYLOAD, {})).toBe(false);
    expect(verifyWebhookSignature(SECRET, PAYLOAD, { "content-type": "application/json" })).toBe(false);
  });

  test("rejects empty or null signature", () => {
    expect(verifyWebhookSignature(SECRET, PAYLOAD, "")).toBe(false);
    expect(verifyWebhookSignature(SECRET, PAYLOAD, "   ")).toBe(false);
    expect(verifyWebhookSignature(SECRET, PAYLOAD, { "x-trustlink-signature": "" })).toBe(false);
  });

  test("rejects invalid hex characters", () => {
    // Replace last characters with non-hex
    const invalidHex = VALID_SIGNATURE.slice(0, 60) + "zzzz";
    expect(verifyWebhookSignature(SECRET, PAYLOAD, invalidHex)).toBe(false);
  });

  test("rejects truncated or overlong signatures", () => {
    expect(verifyWebhookSignature(SECRET, PAYLOAD, VALID_SIGNATURE.slice(0, 32))).toBe(false);
    expect(verifyWebhookSignature(SECRET, PAYLOAD, VALID_SIGNATURE + "aa")).toBe(false);
  });

  test("rejects empty secret or missing payload", () => {
    expect(verifyWebhookSignature("", PAYLOAD, VALID_SIGNATURE)).toBe(false);
    expect(verifyWebhookSignature(null as any, PAYLOAD, VALID_SIGNATURE)).toBe(false);
    expect(verifyWebhookSignature(SECRET, null as any, VALID_SIGNATURE)).toBe(false);
    expect(verifyWebhookSignature(SECRET, undefined as any, VALID_SIGNATURE)).toBe(false);
  });
});
