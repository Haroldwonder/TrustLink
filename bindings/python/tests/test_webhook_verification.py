"""Tests for trustlink.webhooks."""

import hashlib
import hmac
import sys
from unittest.mock import MagicMock

# Ensure stellar_sdk is mockable if not installed in local environment
if "stellar_sdk" not in sys.modules:
    sys.modules["stellar_sdk"] = MagicMock()
    sys.modules["stellar_sdk.soroban_types"] = MagicMock()
    sys.modules["stellar_sdk.xdr"] = MagicMock()

import pytest
from trustlink.webhooks import TRUSTLINK_SIGNATURE_HEADER, verify_webhook_signature


def _compute_sig(secret: str | bytes, body: str | bytes) -> str:
    secret_bytes = secret.encode("utf-8") if isinstance(secret, str) else bytes(secret)
    body_bytes = body.encode("utf-8") if isinstance(body, str) else bytes(body)
    return hmac.new(secret_bytes, body_bytes, hashlib.sha256).hexdigest()


class TestWebhookSignatureVerification:
    def test_valid_signature_with_string_body(self):
        secret = "whsec_supersecretkey123"
        body = '{"event":"attestation.created","id":"att-123","status":"active"}'
        sig = _compute_sig(secret, body)

        assert verify_webhook_signature(secret, body, {TRUSTLINK_SIGNATURE_HEADER: sig}) is True

    def test_valid_signature_with_bytes_body_and_secret(self):
        secret = b"whsec_binarysecret456"
        body = b'{"event":"attestation.revoked","id":"att-456"}'
        sig = _compute_sig(secret, body)

        assert verify_webhook_signature(secret, body, {TRUSTLINK_SIGNATURE_HEADER: sig}) is True

    def test_direct_signature_string(self):
        secret = "my-secret"
        body = "raw body text"
        sig = _compute_sig(secret, body)

        assert verify_webhook_signature(secret, body, sig) is True

    def test_case_insensitive_header_lookup(self):
        secret = "my-secret"
        body = "test payload"
        sig = _compute_sig(secret, body)

        assert verify_webhook_signature(secret, body, {"X-TrustLink-Signature": sig}) is True
        assert verify_webhook_signature(secret, body, {"X-TRUSTLINK-SIGNATURE": sig}) is True
        assert verify_webhook_signature(secret, body, {"x-trustlink-signature": sig}) is True

    def test_header_as_list_value(self):
        secret = "my-secret"
        body = "test payload"
        sig = _compute_sig(secret, body)

        assert verify_webhook_signature(secret, body, {"x-trustlink-signature": [sig]}) is True

    def test_custom_header_name(self):
        secret = "custom-sec"
        body = "payload"
        sig = _compute_sig(secret, body)

        assert (
            verify_webhook_signature(
                secret,
                body,
                {"X-Custom-Signature": sig},
                header_name="X-Custom-Signature",
            )
            is True
        )

    def test_tampered_payload_returns_false(self):
        secret = "my-secret"
        body = '{"amount":100}'
        sig = _compute_sig(secret, body)
        tampered_body = '{"amount":1000}'

        assert verify_webhook_signature(secret, tampered_body, {TRUSTLINK_SIGNATURE_HEADER: sig}) is False

    def test_wrong_secret_returns_false(self):
        secret = "correct-secret"
        wrong_secret = "wrong-secret"
        body = '{"amount":100}'
        sig = _compute_sig(secret, body)

        assert verify_webhook_signature(wrong_secret, body, {TRUSTLINK_SIGNATURE_HEADER: sig}) is False

    def test_corrupted_signature_returns_false(self):
        secret = "secret"
        body = "body"
        sig = _compute_sig(secret, body)
        # Flip first character
        flipped = ("0" if sig[0] != "0" else "1") + sig[1:]

        assert verify_webhook_signature(secret, body, {TRUSTLINK_SIGNATURE_HEADER: flipped}) is False

    def test_invalid_hex_length_or_format_returns_false(self):
        secret = "secret"
        body = "body"

        # Too short
        assert verify_webhook_signature(secret, body, "abcd") is False
        # Too long
        assert verify_webhook_signature(secret, body, "a" * 65) is False
        # Non-hex characters
        assert verify_webhook_signature(secret, body, "z" * 64) is False

    def test_missing_header_or_empty_inputs_returns_false(self):
        secret = "secret"
        body = "body"

        assert verify_webhook_signature(secret, body, {}) is False
        assert verify_webhook_signature("", body, "a" * 64) is False
        assert verify_webhook_signature(secret, None, "a" * 64) is False
        assert verify_webhook_signature(secret, body, None) is False
        assert verify_webhook_signature(secret, body, {"x-trustlink-signature": ""}) is False
