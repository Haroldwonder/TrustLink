import { createHmac, timingSafeEqual } from "crypto";

/**
 * Default HTTP header name used by TrustLink to deliver the HMAC-SHA256 signature.
 */
export const TRUSTLINK_SIGNATURE_HEADER = "x-trustlink-signature";

export type WebhookHeaders =
  | Record<string, string | string[] | undefined>
  | { get(name: string): string | null }
  | string;

export interface VerifyWebhookSignatureOptions {
  /**
   * Custom header name to extract signature from. Defaults to "x-trustlink-signature".
   */
  headerName?: string;
}

/**
 * Extracts the signature string from a headers object, map, or direct signature string.
 */
function extractSignature(
  headers: WebhookHeaders,
  headerName: string
): string | null {
  if (typeof headers === "string") {
    return headers.trim();
  }

  if (!headers || typeof headers !== "object") {
    return null;
  }

  // Handle Fetch API / standard Headers object with .get()
  if (typeof (headers as { get?: unknown }).get === "function") {
    const val = (headers as { get(name: string): string | null }).get(headerName);
    return val ? val.trim() : null;
  }

  // Handle plain Record / IncomingHttpHeaders with case-insensitive search
  const target = headerName.toLowerCase();
  for (const [key, value] of Object.entries(headers)) {
    if (key.toLowerCase() === target) {
      if (Array.isArray(value)) {
        return value[0]?.trim() ?? null;
      }
      return typeof value === "string" ? value.trim() : null;
    }
  }

  return null;
}

/**
 * Verifies that an incoming webhook request was signed by TrustLink using HMAC-SHA256.
 *
 * Uses timing-safe comparison (`timingSafeEqual`) to prevent timing attacks.
 *
 * @param secret - The shared webhook secret key.
 * @param rawBody - The exact raw request body (string, Buffer, or Uint8Array) received in the HTTP request.
 * @param headers - HTTP request headers object (e.g. from Express, Fastify, Next.js, or Fetch API)
 *                  or the raw signature string itself.
 * @param options - Optional configuration parameters.
 * @returns `true` if the signature is valid and authentic, `false` otherwise.
 *
 * @example
 * ```ts
 * import { verifyWebhookSignature } from "@trustlink/sdk";
 *
 * // In an Express endpoint (with express.raw({ type: 'application/json' })):
 * app.post("/webhook", (req, res) => {
 *   const isValid = verifyWebhookSignature(
 *     process.env.WEBHOOK_SECRET!,
 *     req.body,
 *     req.headers
 *   );
 *
 *   if (!isValid) {
 *     return res.status(401).send("Invalid signature");
 *   }
 *
 *   const event = JSON.parse(req.body.toString());
 *   // process event...
 *   res.sendStatus(200);
 * });
 * ```
 */
export function verifyWebhookSignature(
  secret: string | Buffer | Uint8Array,
  rawBody: string | Buffer | Uint8Array,
  headers: WebhookHeaders,
  options: VerifyWebhookSignatureOptions = {}
): boolean {
  if (!secret) {
    return false;
  }

  if (rawBody === undefined || rawBody === null) {
    return false;
  }

  const headerName = options.headerName ?? TRUSTLINK_SIGNATURE_HEADER;
  const signature = extractSignature(headers, headerName);

  if (!signature) {
    return false;
  }

  // SHA-256 in hex must be exactly 64 characters
  const cleanSig = signature.toLowerCase().trim();
  if (cleanSig.length !== 64 || !/^[0-9a-f]{64}$/.test(cleanSig)) {
    return false;
  }

  try {
    const hmac = createHmac("sha256", secret);
    if (typeof rawBody === "string") {
      hmac.update(Buffer.from(rawBody, "utf8"));
    } else {
      hmac.update(rawBody);
    }
    const expectedHex = hmac.digest("hex");

    const expectedBuf = Buffer.from(expectedHex, "hex");
    const sigBuf = Buffer.from(cleanSig, "hex");

    if (expectedBuf.length !== sigBuf.length) {
      return false;
    }

    return timingSafeEqual(expectedBuf, sigBuf);
  } catch {
    return false;
  }
}
