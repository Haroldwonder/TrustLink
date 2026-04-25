# TrustLink Issuer CLI

A simple command-line tool for issuers to manage attestations without writing code.

## Features

- **Issue attestations**: Create new claims for users with optional expiration and metadata
- **Revoke attestations**: Revoke existing attestations with optional reason
- **List issued**: View all attestations issued by this issuer with pagination
- **Check claims**: Verify if a subject has a valid claim from this issuer

## Prerequisites

- Node.js 18+
- Stellar testnet account with funds
- TrustLink contract deployed and initialized
- Issuer registered with the admin

## Setup

```bash
cd examples/issuer-cli
npm install
cp .env.example .env
```

Set environment variables:

```bash
export RPC_URL="https://soroban-testnet.stellar.org"
export NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
export TRUSTLINK_CONTRACT_ID="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8"
export ISSUER_SECRET="S..."
```

## Usage

### Issue an Attestation

```bash
node issuer-cli.mjs issue <subject_address> <claim_type> [--expiry <days>] [--metadata <json>]
```

Examples:

```bash
# Issue KYC with 365-day expiration
node issuer-cli.mjs issue GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNHERX3UNFOK2MAGNTQEFUPROTOCOL KYC_PASSED --expiry 365

# Issue with metadata
node issuer-cli.mjs issue GBRPYHIL... ACCREDITED_INVESTOR --expiry 730 --metadata '{"level":"accredited","verified_at":"2026-04-25"}'

# Issue without expiration (default 365 days)
node issuer-cli.mjs issue GBRPYHIL... MERCHANT_VERIFIED
```

Output:
```
📝 Issuing attestation...
   Subject: GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNHERX3UNFOK2MAGNTQEFUPROTOCOL
   Claim Type: KYC_PASSED
   Expires in: 365 days
✓ Attestation created: att_abc123def456...
✓ Expires: 2027-04-25T18:44:40.983Z
```

### Revoke an Attestation

```bash
node issuer-cli.mjs revoke <attestation_id> [--reason <text>]
```

Examples:

```bash
# Revoke without reason
node issuer-cli.mjs revoke att_abc123def456

# Revoke with reason
node issuer-cli.mjs revoke att_abc123def456 --reason "User requested removal"
```

Output:
```
🗑️  Revoking attestation...
   ID: att_abc123def456
   Reason: User requested removal
✓ Attestation revoked
```

### List Issued Attestations

```bash
node issuer-cli.mjs list-issued [--page <n>] [--limit <n>]
```

Examples:

```bash
# List first 10 attestations
node issuer-cli.mjs list-issued

# List page 2 with 20 per page
node issuer-cli.mjs list-issued --page 1 --limit 20
```

Output:
```
📋 Listing issued attestations...
   Page: 0, Limit: 10

   Found 3 attestation(s):
   1. ID: att_abc123
      Subject: GBRPYHIL...
      Claim: KYC_PASSED
      Status: Active
   2. ID: att_def456
      Subject: GCZST3XV...
      Claim: ACCREDITED_INVESTOR
      Status: Active
   3. ID: att_ghi789
      Subject: GXYZ...
      Claim: KYC_PASSED
      Status: Revoked
```

### Check a Claim

```bash
node issuer-cli.mjs check <subject_address> <claim_type>
```

Examples:

```bash
# Check if subject has valid KYC from this issuer
node issuer-cli.mjs check GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNHERX3UNFOK2MAGNTQEFUPROTOCOL KYC_PASSED
```

Output:
```
🔍 Checking claim...
   Subject: GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNHERX3UNFOK2MAGNTQEFUPROTOCOL
   Claim Type: KYC_PASSED
✓ Subject has valid KYC_PASSED claim from this issuer
```

## Help

```bash
node issuer-cli.mjs --help
```

## Common Claim Types

- `KYC_PASSED` - User has passed KYC verification
- `ACCREDITED_INVESTOR` - User qualifies as accredited investor
- `MERCHANT_VERIFIED` - User is a verified merchant
- `AML_CLEARED` - User has passed AML screening
- `SANCTIONS_CHECKED` - User has been checked against sanctions lists

## Error Handling

The CLI provides clear error messages:

```bash
# Missing required environment variable
$ node issuer-cli.mjs issue GBRPYHIL... KYC_PASSED
Error: Missing TRUSTLINK_CONTRACT_ID. Set it in environment variables.

# Invalid subject address
$ node issuer-cli.mjs issue invalid KYC_PASSED
✗ Failed to create attestation: Invalid address

# Attestation already exists
$ node issuer-cli.mjs issue GBRPYHIL... KYC_PASSED
✗ Failed to create attestation: DuplicateAttestation
```

## Scripting

Use the CLI in shell scripts for batch operations:

```bash
#!/bin/bash

# Issue KYC to multiple users
for user in $(cat users.txt); do
  echo "Issuing KYC to $user..."
  node issuer-cli.mjs issue "$user" KYC_PASSED --expiry 365
done
```

## Performance Notes

- Each command makes one or more RPC calls to the Stellar network
- Network latency affects command execution time (typically 2-5 seconds)
- For batch operations, consider adding delays between commands

## Production Considerations

1. **Security**: Never commit `.env` files with real secret keys
2. **Backups**: Keep records of issued attestation IDs for auditing
3. **Monitoring**: Log all CLI operations for compliance
4. **Rate Limiting**: Stellar network has rate limits; space out bulk operations
5. **Error Recovery**: Implement retry logic for transient failures
