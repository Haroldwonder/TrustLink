"""TrustLink Webhook Signature Verification."""

from __future__ import annotations

import hashlib
import hmac
from typing import Any, Mapping, Union

TRUSTLINK_SIGNATURE_HEADER = "x-trustlink-signature"


def verify_webhook_signature(
    secret: str | bytes,
    raw_body: str | bytes,
    headers: Union[Mapping[str, Any], str],
    header_name: str = TRUSTLINK_SIGNATURE_HEADER,
) -> bool:
    """Verifies that an incoming webhook request was signed by TrustLink using HMAC-SHA256.

    Uses constant-time comparison (hmac.compare_digest) to prevent timing attacks.

    Args:
        secret: The shared webhook secret key.
        raw_body: The exact raw request body (str or bytes).
        headers: Request headers mapping or the direct signature hex string.
        header_name: Custom signature header name (defaults to 'x-trustlink-signature').

    Returns:
        True if the signature is valid, False otherwise.
    """
    if not secret or raw_body is None:
        return False

    if isinstance(headers, str):
        signature = headers.strip()
    elif isinstance(headers, Mapping):
        target = header_name.lower()
        signature = None
        for k, v in headers.items():
            if str(k).lower() == target:
                if isinstance(v, (list, tuple)) and v:
                    signature = str(v[0]).strip()
                elif v is not None:
                    signature = str(v).strip()
                break
        if not signature:
            return False
    else:
        return False

    clean_sig = signature.lower().strip()
    if len(clean_sig) != 64 or not all(c in "0123456789abcdef" for c in clean_sig):
        return False

    secret_bytes = secret.encode("utf-8") if isinstance(secret, str) else bytes(secret)
    body_bytes = raw_body.encode("utf-8") if isinstance(raw_body, str) else bytes(raw_body)

    expected = hmac.new(secret_bytes, body_bytes, hashlib.sha256).hexdigest()
    return hmac.compare_digest(expected.lower(), clean_sig)
