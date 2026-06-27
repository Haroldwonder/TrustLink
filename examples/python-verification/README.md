# Python Server-Side Verification Example

This example demonstrates how to verify TrustLink attestations from a Python backend service using the Stellar RPC API.

## Use Cases

- **Backend KYC Checks**: Verify user credentials before processing sensitive operations
- **API Gateway Middleware**: Check attestations before allowing access to protected endpoints
- **Batch Processing**: Verify multiple users' claims in background jobs
- **Compliance Auditing**: Query attestation status for regulatory reporting

## Prerequisites

- Python 3.8+
- Stellar testnet account with funds
- TrustLink contract deployed and initialized
- Subject and issuer addresses registered

## Setup

```bash
cd examples/python-verification
pip install -r requirements.txt
cp .env.example .env
```

Set environment variables:

```bash
export RPC_URL="https://soroban-testnet.stellar.org"
export NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
export TRUSTLINK_CONTRACT_ID="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8"
export SUBJECT_ADDRESS="GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNHERX3UNFOK2MAGNTQEFUProtocol"
export ISSUER_ADDRESS="GCZST3XVCDTUJ76ZAV2HA72KYQJM3O5OF7MANXVZUOTSBZUJUJVOY7XL"
```

## Run

```bash
python verify.py
```

## API Reference

### TrustLinkVerifier

Main class for verifying attestations.

#### `verify_claim(subject_address, claim_type) -> bool`

Check if a subject has a valid claim of any issuer.

```python
verifier = TrustLinkVerifier(rpc_url, network_passphrase, contract_id)
has_kyc = verifier.verify_claim("GBRPYHIL...", "KYC_PASSED")
if has_kyc:
    # Proceed with operation
    pass
```

#### `verify_claim_from_issuer(subject_address, claim_type, issuer_address) -> bool`

Check if a subject has a valid claim from a specific issuer.

```python
has_issuer_kyc = verifier.verify_claim_from_issuer(
    "GBRPYHIL...",
    "KYC_PASSED",
    "GCZST3XV..."
)
```

#### `get_attestation_status(attestation_id) -> Optional[str]`

Get the status of an attestation ("Valid", "Expired", "Revoked").

```python
status = verifier.get_attestation_status("att_abc123...")
if status == "Valid":
    # Attestation is current
    pass
elif status == "Expired":
    # Attestation needs renewal
    pass
```

## Integration Examples

### Flask API Middleware

```python
from flask import Flask, request, jsonify
from functools import wraps

app = Flask(__name__)
verifier = TrustLinkVerifier(rpc_url, network_passphrase, contract_id)

def require_kyc(f):
    @wraps(f)
    def decorated_function(*args, **kwargs):
        user_address = request.headers.get("X-User-Address")
        if not user_address:
            return jsonify({"error": "Missing user address"}), 400
        
        if not verifier.verify_claim(user_address, "KYC_PASSED"):
            return jsonify({"error": "KYC verification failed"}), 403
        
        return f(*args, **kwargs)
    return decorated_function

@app.route("/api/deposit", methods=["POST"])
@require_kyc
def deposit():
    return jsonify({"status": "deposit accepted"})
```

### Batch Verification

```python
users = ["GBRPYHIL...", "GCZST3XV...", "GXYZ..."]
verified_users = [
    user for user in users
    if verifier.verify_claim(user, "KYC_PASSED")
]
print(f"Verified {len(verified_users)}/{len(users)} users")
```

### Compliance Report

```python
attestation_ids = ["att_001", "att_002", "att_003"]
for att_id in attestation_ids:
    status = verifier.get_attestation_status(att_id)
    print(f"{att_id}: {status}")
```

## Error Handling

The verifier returns `False` or `None` on errors rather than raising exceptions. Check logs for details:

```python
import logging
logging.basicConfig(level=logging.DEBUG)

# Errors will be logged to stderr
has_kyc = verifier.verify_claim(address, "KYC_PASSED")
```

## Performance Notes

- Each verification makes an RPC call to simulate a contract invocation
- For high-volume verification, consider caching results with a TTL
- Batch multiple verifications in parallel using `asyncio` or `concurrent.futures`

## Production Considerations

1. **Error Handling**: Implement retry logic for transient RPC failures
2. **Caching**: Cache verification results with appropriate TTL
3. **Monitoring**: Log all verification attempts for audit trails
4. **Rate Limiting**: Implement rate limits on verification endpoints
5. **Security**: Never expose contract IDs or addresses in error messages
6. **Timeouts**: Set appropriate timeouts for RPC calls
