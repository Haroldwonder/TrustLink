"""Tests for trustlink.bundle_helpers."""

from trustlink.bundle_helpers import (
    BundleOptions,
    verify_attestations_in_same_bundle,
    verify_bundle_claim_types,
    verify_bundle_size,
    verify_bundle_subjects,
    verify_bundle_issuer,
    get_bundle_summary,
    group_attestations_by_bundle,
)


def _att(bundle_id=None, subject="S", issuer="I"):
    return {"bundle_id": bundle_id, "subject": subject, "issuer": issuer}


def _bundle():
    return {
        "id": "bundle-1",
        "issuer": "I",
        "subject": "S",
        "claim_types": ["KYC_PASSED", "AGE_VERIFIED"],
        "attestation_ids": [1, 2],
        "all_valid": True,
        "timestamp": 1234,
    }


class TestBundleOptions:
    def test_stores_fields(self):
        opts = BundleOptions("I", "S", ["KYC_PASSED"], expiration=10, metadata="m", tags=["t"])
        assert opts.issuer == "I"
        assert opts.subject == "S"
        assert opts.claim_types == ["KYC_PASSED"]
        assert opts.expiration == 10
        assert opts.metadata == "m"
        assert opts.tags == ["t"]

    def test_optional_defaults_none(self):
        opts = BundleOptions("I", "S", ["KYC_PASSED"])
        assert opts.expiration is None
        assert opts.metadata is None
        assert opts.tags is None


class TestVerifyAttestationsInSameBundle:
    def test_returns_common_bundle_id(self):
        atts = [_att("b1"), _att("b1")]
        assert verify_attestations_in_same_bundle(atts) == "b1"

    def test_returns_none_for_empty(self):
        assert verify_attestations_in_same_bundle([]) is None

    def test_returns_none_when_first_has_no_bundle(self):
        assert verify_attestations_in_same_bundle([_att(None), _att("b1")]) is None

    def test_returns_none_when_mismatched(self):
        assert verify_attestations_in_same_bundle([_att("b1"), _att("b2")]) is None


class TestVerifyBundleClaimTypes:
    def test_true_when_exact_match_in_order(self):
        assert verify_bundle_claim_types(_bundle(), ["KYC_PASSED", "AGE_VERIFIED"]) is True

    def test_false_when_wrong_order(self):
        assert verify_bundle_claim_types(_bundle(), ["AGE_VERIFIED", "KYC_PASSED"]) is False

    def test_false_when_length_differs(self):
        assert verify_bundle_claim_types(_bundle(), ["KYC_PASSED"]) is False


class TestVerifyBundleSize:
    def test_true_when_counts_match(self):
        assert verify_bundle_size(_bundle(), 2) is True

    def test_false_when_counts_differ(self):
        assert verify_bundle_size(_bundle(), 3) is False


class TestVerifyBundleSubjects:
    def test_true_when_all_match(self):
        assert verify_bundle_subjects(_bundle(), [_att(subject="S"), _att(subject="S")]) is True

    def test_true_for_empty(self):
        assert verify_bundle_subjects(_bundle(), []) is True

    def test_false_when_one_differs(self):
        assert verify_bundle_subjects(_bundle(), [_att(subject="S"), _att(subject="X")]) is False


class TestVerifyBundleIssuer:
    def test_true_when_all_match(self):
        assert verify_bundle_issuer(_bundle(), [_att(issuer="I"), _att(issuer="I")]) is True

    def test_true_for_empty(self):
        assert verify_bundle_issuer(_bundle(), []) is True

    def test_false_when_one_differs(self):
        assert verify_bundle_issuer(_bundle(), [_att(issuer="I"), _att(issuer="X")]) is False


class TestGetBundleSummary:
    def test_summary_contents(self):
        summary = get_bundle_summary(_bundle())
        assert summary == {
            "id": "bundle-1",
            "issuer": "I",
            "subject": "S",
            "claim_types": ["KYC_PASSED", "AGE_VERIFIED"],
            "attestation_count": 2,
            "all_valid": True,
            "created_at": 1234,
        }


class TestGroupAttestationsByBundle:
    def test_groups_by_bundle_id(self):
        a, b, c = _att("b1"), _att("b1"), _att("b2")
        groups = group_attestations_by_bundle([a, b, c])
        assert groups == {"b1": [a, b], "b2": [c]}

    def test_none_bundle_id_grouped_together(self):
        a, b = _att(None), _att(None)
        groups = group_attestations_by_bundle([a, b])
        assert groups == {None: [a, b]}

    def test_empty_input(self):
        assert group_attestations_by_bundle([]) == {}
