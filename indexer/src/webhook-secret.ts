/**
 * Webhook secret encryption utilities.
 * 
 * Issue #1604: Secrets are encrypted at rest using AES-256-GCM with a
 * server-side application key (WEBHOOK_SECRET_KEY). Encryption keys are
 * never exposed to the client; instead, secrets are generated server-side
 * and returned only once on creation.
 */

import { createCipheriv, createDecipheriv, randomBytes } from "crypto";

// Webhook secret constraints
export const MIN_SECRET_LENGTH = 32; // 32 bytes = 256 bits
export const MAX_SECRET_LENGTH = 256; // Reasonable upper bound
export const SECRET_ENCODING = "hex";
export const ALGORITHM = "aes-256-gcm";

/**
 * Get the encryption key from environment variable WEBHOOK_SECRET_KEY.
 * The key must be exactly 32 bytes (256 bits) for AES-256.
 */
function getEncryptionKey(): Buffer {
  const keyHex = process.env.WEBHOOK_SECRET_KEY;
  if (!keyHex) {
    throw new Error(
      "WEBHOOK_SECRET_KEY environment variable is not set. " +
      "Generate a 32-byte key with: node -e \"console.log(require('crypto').randomBytes(32).toString('hex'))\""
    );
  }
  
  const key = Buffer.from(keyHex, "hex");
  if (key.length !== 32) {
    throw new Error(`WEBHOOK_SECRET_KEY must be exactly 32 bytes (64 hex chars), got ${key.length} bytes`);
  }
  
  return key;
}

/**
 * Validate webhook secret strength and format.
 * 
 * @param secret Raw secret bytes (before encryption)
 * @throws Error if secret is too short or too long
 */
export function validateWebhookSecret(secret: Buffer): void {
  if (secret.length < MIN_SECRET_LENGTH) {
    throw new Error(
      `Webhook secret must be at least ${MIN_SECRET_LENGTH} bytes. ` +
      `Generate a strong secret with: node -e "console.log(require('crypto').randomBytes(32).toString('hex'))"`
    );
  }
  
  if (secret.length > MAX_SECRET_LENGTH) {
    throw new Error(`Webhook secret must not exceed ${MAX_SECRET_LENGTH} bytes`);
  }
}

/**
 * Generate a cryptographically secure random webhook secret.
 * Returns a hex-encoded string of MIN_SECRET_LENGTH bytes.
 */
export function generateWebhookSecret(): string {
  const secret = randomBytes(MIN_SECRET_LENGTH);
  return secret.toString("hex");
}

/**
 * Encrypt a webhook secret for storage at rest.
 * 
 * @param plaintext Hex-encoded secret string (from generateWebhookSecret or client submission)
 * @returns Encrypted secret as `base64(iv + ciphertext + authTag)`
 */
export function encryptSecret(plaintext: string): string {
  const secretBuffer = Buffer.from(plaintext, "hex");
  validateWebhookSecret(secretBuffer);
  
  const key = getEncryptionKey();
  const iv = randomBytes(12); // 96-bit IV for GCM
  const cipher = createCipheriv(ALGORITHM, key, iv);
  
  const ciphertext = Buffer.concat([
    cipher.update(secretBuffer),
    cipher.final(),
  ]);
  
  const authTag = cipher.getAuthTag();
  
  // Prepend IV and auth tag so decryption can extract them
  const encrypted = Buffer.concat([iv, ciphertext, authTag]);
  return encrypted.toString("base64");
}

/**
 * Decrypt a webhook secret from storage.
 * 
 * @param encrypted Base64-encoded encrypted secret (from encryptSecret)
 * @returns Hex-encoded plaintext secret
 * @throws Error if decryption fails (tampering or wrong key)
 */
export function decryptSecret(encrypted: string): string {
  const key = getEncryptionKey();
  const encryptedBuffer = Buffer.from(encrypted, "base64");
  
  // Extract components: IV (12 bytes) + ciphertext + authTag (16 bytes)
  if (encryptedBuffer.length < 12 + 16) {
    throw new Error("Invalid encrypted secret format");
  }
  
  const iv = encryptedBuffer.slice(0, 12);
  const authTag = encryptedBuffer.slice(-16);
  const ciphertext = encryptedBuffer.slice(12, -16);
  
  const decipher = createDecipheriv(ALGORITHM, key, iv);
  decipher.setAuthTag(authTag);
  
  try {
    const plaintext = Buffer.concat([
      decipher.update(ciphertext),
      decipher.final(),
    ]);
    return plaintext.toString("hex");
  } catch (err) {
    throw new Error("Webhook secret decryption failed: likely wrong key or corrupted data");
  }
}

/**
 * Sign webhook payload with a secret (same as before).
 * Called with the decrypted secret.
 */
export function signWebhookPayload(secret: string, body: string): string {
  const { createHmac } = require("crypto");
  return createHmac("sha256", Buffer.from(secret, "hex")).update(body).digest("hex");
}
