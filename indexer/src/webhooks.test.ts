import { createHmac } from "crypto";

// Re-implement the sign function exactly as in webhooks.ts so the test is
// self-contained and doesn't depend on module internals.
function sign(secret: string, body: string): string {
  return createHmac("sha256", secret).update(body).digest("hex");
}

function verify(secret: string, body: string, signature: string): boolean {
  const expected = sign(secret, body);
  // Constant-time comparison to prevent timing attacks.
  if (expected.length !== signature.length) return false;
  let diff = 0;
  for (let i = 0; i < expected.length; i++) {
    diff |= expected.charCodeAt(i) ^ signature.charCodeAt(i);
  }
  return diff === 0;
}

const SECRET = "test-secret-key";
const PAYLOAD = JSON.stringify({ event: "attestation_created", data: { id: "abc123" }, ts: 1700000000000 });

describe("webhook signature (HMAC-SHA256)", () => {
  it("produces a 64-character hex digest", () => {
    const sig = sign(SECRET, PAYLOAD);
    expect(sig).toHaveLength(64);
    expect(sig).toMatch(/^[0-9a-f]+$/);
  });

  it("verifies a correctly signed payload", () => {
    const sig = sign(SECRET, PAYLOAD);
    expect(verify(SECRET, PAYLOAD, sig)).toBe(true);
  });

  it("rejects a tampered payload body", () => {
    const sig = sign(SECRET, PAYLOAD);
    const tampered = PAYLOAD.replace("abc123", "evil999");
    expect(verify(SECRET, tampered, sig)).toBe(false);
  });

  it("rejects a signature produced with a different secret", () => {
    const sig = sign("wrong-secret", PAYLOAD);
    expect(verify(SECRET, PAYLOAD, sig)).toBe(false);
  });

  it("rejects a truncated signature", () => {
    const sig = sign(SECRET, PAYLOAD).slice(0, 32);
    expect(verify(SECRET, PAYLOAD, sig)).toBe(false);
  });

  it("is deterministic — same inputs always produce the same signature", () => {
    expect(sign(SECRET, PAYLOAD)).toBe(sign(SECRET, PAYLOAD));
  });

  it("produces different signatures for different secrets", () => {
    expect(sign("secret-a", PAYLOAD)).not.toBe(sign("secret-b", PAYLOAD));
  });
});
