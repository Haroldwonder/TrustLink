# Webhook Security (Issue #1604)

## Overview

Webhook secrets are now encrypted at rest using AES-256-GCM. This prevents database administrators, backup operators, or attackers with read access to the database from forging webhook signatures.

## Secret Generation and Storage

### Server-Side Generation (Recommended)

When creating a webhook, **do not** provide a secret. The server will generate a cryptographically secure 32-byte (256-bit) random secret and return it **exactly once**:

```bash
curl -X POST http://localhost:3000/webhooks \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/webhook"}'

# Response:
{
  "id": "webhook_123",
  "url": "https://example.com/webhook",
  "secret": "a1b2c3d4e5f6...", # 64 hex characters = 32 bytes
  "active": true,
  "rotatedAt": "2026-11-18T12:00:00Z"
}
```

Store this secret securely on the client side (e.g., in a secrets manager). It will not be returned again.

### Client-Provided Secret

Alternatively, provide your own secret as a 32+ byte hex-encoded string:

```bash
curl -X POST http://localhost:3000/webhooks \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/webhook",
    "secret": "your_32_byte_hex_encoded_secret_here_64_chars_long"
  }'
```

## Environment Configuration

### Setting the Encryption Key

The server uses `WEBHOOK_SECRET_KEY` environment variable (a 32-byte hex-encoded AES key):

```bash
# Generate a new key (run once, store securely):
node -e "console.log(require('crypto').randomBytes(32).toString('hex'))"
# Output: a1b2c3d4e5f6...  (64 hex characters)

# Set it in your environment:
export WEBHOOK_SECRET_KEY="a1b2c3d4e5f6..."
```

If `WEBHOOK_SECRET_KEY` is not set, the server will fail to start with a clear error message.

## Secret Rotation

### Why Rotate?

- After accidental exposure
- Periodically as a security best practice (e.g., every 90 days)
- When changing webhook endpoints

### How to Rotate

**POST /webhooks/:id/rotate-secret**

```bash
curl -X POST http://localhost:3000/webhooks/webhook_123/rotate-secret

# Response:
{
  "id": "webhook_123",
  "url": "https://example.com/webhook",
  "secret": "new_secret_a1b2c3d4...",  # New secret returned once
  "rotatedAt": "2026-11-18T13:00:00Z",
  "graceMessage": "Old secret will be accepted for 7 days via X-TrustLink-Signature-Next header"
}
```

### Grace Period (Dual-Signing)

During rotation, the server signs all webhook payloads with **both** the old and new secrets for 7 days:

- `X-TrustLink-Signature`: Signed with the **new** secret
- `X-TrustLink-Signature-Next`: Signed with the **old** secret (for compatibility)

This allows clients to:

1. Update their secret immediately after rotation
2. Gradually migrate verification to the new secret
3. Fall back to the old secret if they missed the update

After the 7-day grace period, the server will only sign with the new secret. Old secrets are not retained (they cannot be recovered).

## Implementation Details

### Database Schema

```sql
CREATE TABLE "Webhook" (
  id                    String   @id
  url                   String
  secret_encrypted      String   -- AES-256-GCM encrypted, base64-encoded
  next_secret_encrypted String?  -- Old secret during grace period
  rotatedAt             DateTime -- Tracks rotation time
  active                Boolean
  createdAt             DateTime
  updatedAt             DateTime
)
```

### Encryption Format

Each encrypted secret is formatted as: `base64(IV || ciphertext || authTag)`

- **IV** (12 bytes): Random nonce
- **ciphertext**: Encrypted 32-byte secret
- **authTag** (16 bytes): AEAD authentication tag

### Signing Webhook Payloads

Webhooks are signed using HMAC-SHA256 with the decrypted secret:

```
signature = HMAC-SHA256(secret, payload)
```

## API Endpoints

### Create Webhook
**POST /webhooks**

Request (server-generated secret):
```json
{"url": "https://example.com/webhook"}
```

Request (client-provided secret):
```json
{
  "url": "https://example.com/webhook",
  "secret": "your_32_byte_hex_secret"
}
```

Response (201 Created):
```json
{
  "id": "webhook_123",
  "url": "https://example.com/webhook",
  "secret": "...",  # Only returned on creation
  "active": true,
  "rotatedAt": "2026-11-18T12:00:00Z"
}
```

### List Webhooks
**GET /webhooks**

Response:
```json
[
  {
    "id": "webhook_123",
    "url": "https://example.com/webhook",
    "active": true,
    "rotatedAt": "2026-11-18T12:00:00Z",
    "createdAt": "2026-11-10T08:00:00Z"
  }
]
```

Note: Secrets are never returned in list responses.

### Rotate Webhook Secret
**POST /webhooks/:id/rotate-secret**

Response (200 OK):
```json
{
  "id": "webhook_123",
  "url": "https://example.com/webhook",
  "secret": "new_secret_here",
  "rotatedAt": "2026-11-18T13:00:00Z",
  "graceMessage": "Old secret will be accepted for 7 days..."
}
```

### Delete Webhook
**DELETE /webhooks/:id**

Response (204 No Content)

## Testing Webhook Signatures

### With Server-Generated Secret

```bash
# 1. Create webhook
SECRET=$(curl -X POST http://localhost:3000/webhooks \
  -H "Content-Type: application/json" \
  -d '{"url": "https://webhook.site/unique-id"}' \
  | jq -r '.secret')

# 2. Sign test payload
PAYLOAD='{"event":"test","data":{},"ts":1234567890}'
SIGNATURE=$(echo -n "$PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" -hex | cut -d' ' -f2)

# 3. Verify by sending to webhook.site and checking signature
```

### With Known Secret

```bash
SECRET="a1b2c3d4e5f6..." # Your 32-byte hex secret
PAYLOAD='{"event":"test","data":{}}'
SIGNATURE=$(echo -n "$PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" -hex | cut -d' ' -f2)

# Validate: HMAC-SHA256(secret, payload) == signature header
```

## Migration from Plaintext Secrets

When upgrading from plaintext secrets:

1. A database migration copies existing plaintext secrets to encrypted storage
2. Secrets are encrypted on server startup (or via manual script)
3. The `secret_plaintext_legacy` column can be safely dropped after verification
4. Existing webhooks continue to work without interruption

To verify migration:

```bash
# Check that all webhooks have encrypted secrets
SELECT COUNT(*) FROM "Webhook" WHERE secret_encrypted IS NOT NULL;

# Check for any remaining plaintext
SELECT COUNT(*) FROM "Webhook" WHERE secret_plaintext_legacy IS NOT NULL;
```

## Security Considerations

- **Keys**: `WEBHOOK_SECRET_KEY` must be protected like a database password
- **Backups**: Encrypted secrets in backups are useless without the key
- **Rotation**: Rotate `WEBHOOK_SECRET_KEY` separately from webhook secrets (requires re-encryption)
- **Expiry**: Old secrets during grace period are retained only temporarily
- **Audit**: All rotation events are logged via `rotatedAt` timestamp

## Troubleshooting

**Error: "WEBHOOK_SECRET_KEY environment variable is not set"**
- Set `WEBHOOK_SECRET_KEY` before starting the server
- Generate a new key: `node -e "console.log(require('crypto').randomBytes(32).toString('hex'))"`

**Error: "Webhook secret must be at least 32 bytes"**
- Provide a 32-byte (64 hex character) secret
- Or omit the secret to let the server generate one

**Error: "Webhook secret decryption failed"**
- The `WEBHOOK_SECRET_KEY` may have changed without re-encrypting secrets
- Use the same key as when secrets were encrypted

## References

- [OWASP: Secrets Management](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)
- [AES-GCM](https://en.wikipedia.org/wiki/Galois/Counter_Mode)
- [HMAC-SHA256](https://en.wikipedia.org/wiki/HMAC)
