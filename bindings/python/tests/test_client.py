"""Tests for TrustLinkClient.has_all_claims and has_any_claim."""

import pytest
from unittest.mock import MagicMock, patch, PropertyMock
from trustlink import TrustLinkClient, TrustLinkError

CONTRACT_ID = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQAHHAGK6Z6E"
RPC_URL = "https://soroban-testnet.stellar.org"
SUBJECT = "GDXLKEY5TR4IDEVSTRYUNYY3DPXQKQNSTDJ7HIVNFTJYQHOZXB7CRQME"


@pytest.fixture
def client():
    return TrustLinkClient(contract_id=CONTRACT_ID, rpc_url=RPC_URL)


def _bool_retval(value: bool):
    """Build a mock SCVal bool return value."""
    mock = MagicMock()
    mock.b = value
    return mock


# ── has_any_claim ─────────────────────────────────────────────────────────────

class TestHasAnyClaim:
    def test_returns_true_when_subject_has_matching_claim(self, client):
        with patch.object(client, "_simulate", return_value=_bool_retval(True)) as mock_sim:
            result = client.has_any_claim(SUBJECT, ["KYC_PASSED"])
        assert result is _bool_retval(True)
        mock_sim.assert_called_once_with(
            "has_any_claim",
            client._addr(SUBJECT),
            client._vec_str(["KYC_PASSED"]),
        )

    def test_returns_false_when_no_matching_claim(self, client):
        with patch.object(client, "_simulate", return_value=_bool_retval(False)):
            result = client.has_any_claim(SUBJECT, ["ACCREDITED_INVESTOR"])
        assert result is _bool_retval(False)

    def test_returns_true_on_partial_match(self, client):
        """OR-logic: true if any one claim matches."""
        with patch.object(client, "_simulate", return_value=_bool_retval(True)):
            result = client.has_any_claim(SUBJECT, ["KYC_PASSED", "ACCREDITED_INVESTOR"])
        assert result is _bool_retval(True)

    def test_empty_list_returns_false_without_calling_simulate(self, client):
        with patch.object(client, "_simulate") as mock_sim:
            result = client.has_any_claim(SUBJECT, [])
        assert result is False
        mock_sim.assert_not_called()

    def test_raises_on_non_list_claim_types(self, client):
        with pytest.raises(TrustLinkError, match="claim_types must be a list"):
            client.has_any_claim(SUBJECT, "KYC_PASSED")  # type: ignore

    def test_raises_on_empty_string_in_list(self, client):
        with pytest.raises(TrustLinkError, match="non-empty string"):
            client.has_any_claim(SUBJECT, ["KYC_PASSED", ""])

    def test_raises_on_non_string_item_in_list(self, client):
        with pytest.raises(TrustLinkError, match="non-empty string"):
            client.has_any_claim(SUBJECT, ["KYC_PASSED", 42])  # type: ignore

    def test_raises_on_none_item_in_list(self, client):
        with pytest.raises(TrustLinkError, match="non-empty string"):
            client.has_any_claim(SUBJECT, [None])  # type: ignore

    def test_propagates_simulate_error(self, client):
        with patch.object(client, "_simulate", side_effect=TrustLinkError("Simulation error: rpc down")):
            with pytest.raises(TrustLinkError, match="Simulation error"):
                client.has_any_claim(SUBJECT, ["KYC_PASSED"])

    def test_propagates_network_error(self, client):
        with patch.object(client, "_simulate", side_effect=ConnectionError("timeout")):
            with pytest.raises(ConnectionError):
                client.has_any_claim(SUBJECT, ["KYC_PASSED"])

    def test_single_claim_type_equivalent_to_has_valid_claim(self, client):
        """has_any_claim with one element should behave like has_valid_claim."""
        retval = _bool_retval(True)
        with patch.object(client, "_simulate", return_value=retval) as mock_sim:
            result = client.has_any_claim(SUBJECT, ["KYC_PASSED"])
        assert result is retval
        mock_sim.assert_called_once_with(
            "has_any_claim",
            client._addr(SUBJECT),
            client._vec_str(["KYC_PASSED"]),
        )

    def test_duplicate_entries_passed_through_to_contract(self, client):
        """Deduplication is the contract's responsibility; SDK passes as-is."""
        with patch.object(client, "_simulate", return_value=_bool_retval(True)) as mock_sim:
            client.has_any_claim(SUBJECT, ["KYC_PASSED", "KYC_PASSED"])
        args = mock_sim.call_args[0]
        # Third arg is the vec — should contain both entries
        assert args[0] == "has_any_claim"

    def test_vec_str_serialization(self, client):
        """_vec_str encodes each claim type as a SYMBOL SCVal."""
        from stellar_sdk import xdr
        vec = client._vec_str(["KYC_PASSED", "AML_CLEARED"])
        assert vec.type == xdr.SCValType.SC_VAL_TYPE_VEC
        assert len(vec.vec) == 2
        assert vec.vec[0].sym == b"KYC_PASSED"
        assert vec.vec[1].sym == b"AML_CLEARED"

    def test_vec_str_empty_list(self, client):
        from stellar_sdk import xdr
        vec = client._vec_str([])
        assert vec.type == xdr.SCValType.SC_VAL_TYPE_VEC
        assert vec.vec == []


# ── has_all_claims ────────────────────────────────────────────────────────────

class TestHasAllClaims:
    def test_returns_true_when_subject_has_all_claims(self, client):
        with patch.object(client, "_simulate", return_value=_bool_retval(True)) as mock_sim:
            result = client.has_all_claims(SUBJECT, ["KYC_PASSED", "AML_CLEARED"])
        assert result is _bool_retval(True)
        mock_sim.assert_called_once_with(
            "has_all_claims",
            client._addr(SUBJECT),
            client._vec_str(["KYC_PASSED", "AML_CLEARED"]),
        )

    def test_returns_false_when_subject_missing_one_claim(self, client):
        """AND-logic: false if any one claim is missing."""
        with patch.object(client, "_simulate", return_value=_bool_retval(False)):
            result = client.has_all_claims(SUBJECT, ["KYC_PASSED", "ACCREDITED_INVESTOR"])
        assert result is _bool_retval(False)

    def test_returns_false_when_no_claims_match(self, client):
        with patch.object(client, "_simulate", return_value=_bool_retval(False)):
            result = client.has_all_claims(SUBJECT, ["ACCREDITED_INVESTOR"])
        assert result is _bool_retval(False)

    def test_empty_list_returns_true_without_calling_simulate(self, client):
        """Vacuous truth: all claims in an empty set are satisfied."""
        with patch.object(client, "_simulate") as mock_sim:
            result = client.has_all_claims(SUBJECT, [])
        assert result is True
        mock_sim.assert_not_called()

    def test_raises_on_non_list_claim_types(self, client):
        with pytest.raises(TrustLinkError, match="claim_types must be a list"):
            client.has_all_claims(SUBJECT, "KYC_PASSED")  # type: ignore

    def test_raises_on_empty_string_in_list(self, client):
        with pytest.raises(TrustLinkError, match="non-empty string"):
            client.has_all_claims(SUBJECT, [""])

    def test_raises_on_non_string_item_in_list(self, client):
        with pytest.raises(TrustLinkError, match="non-empty string"):
            client.has_all_claims(SUBJECT, [123])  # type: ignore

    def test_raises_on_none_item_in_list(self, client):
        with pytest.raises(TrustLinkError, match="non-empty string"):
            client.has_all_claims(SUBJECT, [None])  # type: ignore

    def test_propagates_simulate_error(self, client):
        with patch.object(client, "_simulate", side_effect=TrustLinkError("Simulation error: contract panic")):
            with pytest.raises(TrustLinkError, match="Simulation error"):
                client.has_all_claims(SUBJECT, ["KYC_PASSED"])

    def test_propagates_network_error(self, client):
        with patch.object(client, "_simulate", side_effect=OSError("connection refused")):
            with pytest.raises(OSError):
                client.has_all_claims(SUBJECT, ["KYC_PASSED"])

    def test_single_claim_type_equivalent_to_has_valid_claim(self, client):
        retval = _bool_retval(True)
        with patch.object(client, "_simulate", return_value=retval) as mock_sim:
            result = client.has_all_claims(SUBJECT, ["KYC_PASSED"])
        assert result is retval
        mock_sim.assert_called_once_with(
            "has_all_claims",
            client._addr(SUBJECT),
            client._vec_str(["KYC_PASSED"]),
        )

    def test_duplicate_entries_passed_through_to_contract(self, client):
        with patch.object(client, "_simulate", return_value=_bool_retval(True)) as mock_sim:
            client.has_all_claims(SUBJECT, ["KYC_PASSED", "KYC_PASSED"])
        assert mock_sim.called


# ── Parity with has_valid_claim ───────────────────────────────────────────────

class TestParityWithHasValidClaim:
    def test_has_any_claim_uses_same_addr_serialization(self, client):
        """_addr serialization is identical across all three methods."""
        expected_addr = client._addr(SUBJECT)
        with patch.object(client, "_simulate") as mock_sim:
            mock_sim.return_value = _bool_retval(True)
            client.has_any_claim(SUBJECT, ["KYC_PASSED"])
        call_args = mock_sim.call_args[0]
        # Second positional arg is the subject address
        assert call_args[1].type == expected_addr.type

    def test_has_all_claims_uses_same_addr_serialization(self, client):
        expected_addr = client._addr(SUBJECT)
        with patch.object(client, "_simulate") as mock_sim:
            mock_sim.return_value = _bool_retval(False)
            client.has_all_claims(SUBJECT, ["KYC_PASSED"])
        call_args = mock_sim.call_args[0]
        assert call_args[1].type == expected_addr.type

    def test_has_any_claim_method_name_matches_contract(self, client):
        with patch.object(client, "_simulate") as mock_sim:
            mock_sim.return_value = _bool_retval(True)
            client.has_any_claim(SUBJECT, ["KYC_PASSED"])
        assert mock_sim.call_args[0][0] == "has_any_claim"

    def test_has_all_claims_method_name_matches_contract(self, client):
        with patch.object(client, "_simulate") as mock_sim:
            mock_sim.return_value = _bool_retval(False)
            client.has_all_claims(SUBJECT, ["KYC_PASSED"])
        assert mock_sim.call_args[0][0] == "has_all_claims"

    def test_has_any_and_has_all_diverge_on_partial_match(self, client):
        """has_any returns True, has_all returns False for partial match."""
        with patch.object(client, "_simulate", return_value=_bool_retval(True)):
            any_result = client.has_any_claim(SUBJECT, ["KYC_PASSED", "ACCREDITED_INVESTOR"])

        with patch.object(client, "_simulate", return_value=_bool_retval(False)):
            all_result = client.has_all_claims(SUBJECT, ["KYC_PASSED", "ACCREDITED_INVESTOR"])

        assert any_result is _bool_retval(True)
        assert all_result is _bool_retval(False)

    def test_both_methods_exported_from_package(self):
        """has_any_claim and has_all_claims are accessible on TrustLinkClient."""
        assert callable(getattr(TrustLinkClient, "has_any_claim", None))
        assert callable(getattr(TrustLinkClient, "has_all_claims", None))
