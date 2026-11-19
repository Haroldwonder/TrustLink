"""Tests for sc_str encoding with long attestation IDs (Issue #933)."""

import pytest
from stellar_sdk import xdr
from trustlink._base import sc_str


class TestScStrEncoding:
    """Test that sc_str correctly encodes strings, including long attestation IDs."""

    def test_sc_str_encodes_as_string_type_not_symbol(self):
        """sc_str should encode as SC_VAL_TYPE_STRING, not SC_VAL_TYPE_SYMBOL."""
        result = sc_str("test")
        assert result.type == xdr.SCValType.SC_VAL_TYPE_STRING
        assert result.str is not None

    def test_sc_str_handles_short_strings(self):
        """sc_str should handle short strings."""
        result = sc_str("test")
        assert result.type == xdr.SCValType.SC_VAL_TYPE_STRING
        assert result.str == b"test"

    def test_sc_str_handles_40_char_attestation_id(self):
        """sc_str should handle 40+ character attestation IDs without truncation."""
        attestation_id = "550e8400-e29b-41d4-a716-446655440000-ext"  # 44 chars
        result = sc_str(attestation_id)
        assert result.type == xdr.SCValType.SC_VAL_TYPE_STRING
        assert result.str == attestation_id.encode()
        assert len(result.str) == 44

    def test_sc_str_handles_uuid_with_hyphens(self):
        """sc_str should handle UUID-style strings with hyphens (not restricted to symbols)."""
        uuid_str = "550e8400-e29b-41d4-a716-446655440000"  # 36 chars
        result = sc_str(uuid_str)
        assert result.type == xdr.SCValType.SC_VAL_TYPE_STRING
        assert result.str == uuid_str.encode()

    def test_sc_str_handles_long_metadata_strings(self):
        """sc_str should handle long metadata strings without truncation."""
        long_metadata = "This is a very long metadata string that exceeds 32 characters and contains various symbols!@#$%"
        result = sc_str(long_metadata)
        assert result.type == xdr.SCValType.SC_VAL_TYPE_STRING
        assert result.str == long_metadata.encode()
        assert len(result.str) == len(long_metadata.encode())

    def test_sc_str_preserves_special_characters(self):
        """sc_str should preserve special characters in strings."""
        special_str = "test-value_with.special/chars@example.com"
        result = sc_str(special_str)
        assert result.type == xdr.SCValType.SC_VAL_TYPE_STRING
        assert result.str == special_str.encode()

    def test_sc_str_handles_empty_string(self):
        """sc_str should handle empty strings."""
        result = sc_str("")
        assert result.type == xdr.SCValType.SC_VAL_TYPE_STRING
        assert result.str == b""

    def test_sc_str_handles_unicode(self):
        """sc_str should handle unicode strings."""
        unicode_str = "test-αβγδ-中文"
        result = sc_str(unicode_str)
        assert result.type == xdr.SCValType.SC_VAL_TYPE_STRING
        assert result.str == unicode_str.encode()
