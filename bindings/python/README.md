# TrustLink Python Bindings

Python SDK for the TrustLink on-chain attestation contract on Stellar.

## Installation

```bash
pip install trustlink-sdk
```

Requires Python 3.8+. For async support (FastAPI, aiohttp):

```bash
pip install "trustlink-sdk[async]"
```

## Quick Start

```python
from trustlink import TrustLinkClient

# Initialize client
client = TrustLinkClient(
    contract_id="C...",
    rpc_url="https://soroban-testnet.stellar.org",
    network_passphrase="Test SDF Network ; September 2015"
)

# Query attestations
attestations = client.get_subject_attestations("GXXXXXX", offset=0, limit=50)

# Check if user has valid claim
has_kyc = client.has_valid_claim("GXXXXXX", "KYC_PASSED")

# Create attestation (requires issuer auth)
client.create_attestation(
    issuer_secret="SXXXXXX",
    subject="GXXXXXX",
    claim_type="KYC_PASSED",
    expiration=None,
    metadata=None
)

# Revoke attestation
client.revoke_attestation(
    issuer_secret="SXXXXXX",
    attestation_id="att_...",
    reason="User requested"
)
```

## Async Usage (asyncio / FastAPI / aiohttp)

Install with the async extra to pull in `httpx`:

```bash
pip install "trustlink-contract[async]"
```

Use `AsyncTrustLinkClient` as an async context manager so the underlying HTTP
session is closed automatically:

```python
import asyncio
from trustlink import AsyncTrustLinkClient

async def main():
    async with AsyncTrustLinkClient(
        contract_id="C...",
        rpc_url="https://soroban-testnet.stellar.org",
    ) as client:
        # All read methods are awaitable
        has_kyc = await client.has_valid_claim("GXXXXXX", "KYC_PASSED")
        print("KYC valid:", has_kyc)

        attestation = await client.get_attestation("att_...")
        print("Attestation:", attestation)

        stats = await client.get_global_stats()
        print("Total attestations:", stats["total_attestations"])

asyncio.run(main())
```

### FastAPI example

```python
from fastapi import FastAPI
from trustlink import AsyncTrustLinkClient

app = FastAPI()
client = AsyncTrustLinkClient(
    contract_id="C...",
    rpc_url="https://soroban-testnet.stellar.org",
)

@app.on_event("shutdown")
async def shutdown():
    await client.close()

@app.get("/kyc/{address}")
async def check_kyc(address: str) -> dict:
    has_kyc = await client.has_valid_claim(address, "KYC_PASSED")
    return {"address": address, "kyc_valid": has_kyc}
```

## API Reference

### Read Operations

- `get_subject_attestations(subject, offset, limit)` - Get attestations for a subject
- `has_valid_claim(subject, claim_type)` - Check if subject has valid claim
- `has_valid_claim_from_issuer(subject, claim_type, issuer)` - Check claim from specific issuer
- `has_any_claim(subject, claim_types)` - Check if subject has any of the claim types
- `has_all_claims(subject, claim_types)` - Check if subject has all claim types
- `get_attestation(attestation_id)` - Get specific attestation
- `get_attestation_status(attestation_id)` - Get attestation status
- `get_issuer_attestations(issuer, offset, limit)` - Get attestations issued by issuer
- `list_claim_types(offset, limit)` - List registered claim types
- `get_global_stats()` - Get contract-wide statistics
- `is_issuer(address)` - Check if address is registered issuer

### Write Operations

- `create_attestation(issuer_secret, subject, claim_type, expiration, metadata)` - Create attestation
- `revoke_attestation(issuer_secret, attestation_id, reason)` - Revoke attestation
- `register_issuer(admin_secret, issuer)` - Register issuer (admin only)
- `remove_issuer(admin_secret, issuer)` - Remove issuer (admin only)
- `propose_attestation(issuer_secret, subject, claim_type, required_signers, threshold)` - Propose multi-sig attestation
- `cosign_attestation(issuer_secret, proposal_id)` - Co-sign multi-sig proposal

## Type Hints

All functions include full type hints for IDE support and static type checking.

```python
from trustlink import Attestation, AttestationStatus

def process_attestation(att: Attestation) -> None:
    print(f"ID: {att['id']}")
    print(f"Issuer: {att['issuer']}")
    print(f"Subject: {att['subject']}")
    print(f"Claim Type: {att['claim_type']}")
    print(f"Status: {att['status']}")
```

## Error Handling

```python
from trustlink import TrustLinkError, ContractError

try:
    client.create_attestation(...)
except ContractError as e:
    print(f"Contract error: {e.code} - {e.message}")
except TrustLinkError as e:
    print(f"SDK error: {e}")
```

## License

MIT
