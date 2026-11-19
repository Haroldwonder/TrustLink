"""Tests for CONTRACT_ERRORS mapping completeness (Issue #932)."""

import pytest
from trustlink.types import CONTRACT_ERRORS, ContractError


class TestContractErrors:
    """Test that CONTRACT_ERRORS mapping is complete and matches TypeScript SDK."""

    def test_contract_errors_contains_required_error_codes(self):
        """CONTRACT_ERRORS should contain all error codes 1-30 and 44."""
        # Verify codes 1-30 are present
        for code in range(1, 31):
            assert code in CONTRACT_ERRORS, f"Missing error code {code}"

        # Verify code 44 is present
        assert 44 in CONTRACT_ERRORS, "Missing error code 44 (InvalidSourceReferenceError)"

    def test_contract_errors_has_correct_error_names(self):
        """CONTRACT_ERRORS should map to correct error class names."""
        expected_errors = {
            1: "AlreadyInitializedError",
            2: "NotInitializedError",
            3: "UnauthorizedError",
            4: "NotFoundError",
            5: "DuplicateAttestationError",
            6: "AlreadyRevokedError",
            7: "ExpiredError",
            8: "InvalidValidFromError",
            9: "InvalidExpirationError",
            10: "MetadataTooLongError",
            11: "InvalidTimestampError",
            12: "InvalidFeeError",
            13: "FeeTokenRequiredError",
            14: "TooManyTagsError",
            15: "TagTooLongError",
            16: "InvalidThresholdError",
            17: "NotRequiredSignerError",
            18: "AlreadySignedError",
            19: "ProposalFinalizedError",
            20: "ProposalExpiredError",
            21: "ReasonTooLongError",
            22: "CannotEndorseOwnError",
            23: "AlreadyEndorsedError",
            24: "ContractPausedError",
            25: "SubjectNotWhitelistedError",
            26: "InvalidClaimTypeError",
            27: "InvalidJurisdictionError",
            28: "RateLimitedError",
            29: "LimitExceededError",
            30: "ProposalCancelledError",
            44: "InvalidSourceReferenceError",
        }

        for code, expected_name in expected_errors.items():
            assert code in CONTRACT_ERRORS, f"Missing error code {code}"
            assert CONTRACT_ERRORS[code] == expected_name, \
                f"Error code {code} should map to {expected_name}, got {CONTRACT_ERRORS[code]}"

    def test_contract_errors_does_not_include_code_0(self):
        """CONTRACT_ERRORS should start at code 1, not 0."""
        assert 0 not in CONTRACT_ERRORS, "Code 0 should not be in CONTRACT_ERRORS (error codes start at 1)"

    def test_sample_contract_error_decoding(self):
        """Test that a sample of contract error codes can be decoded correctly."""
        test_cases = [
            (1, "AlreadyInitializedError"),
            (3, "UnauthorizedError"),
            (7, "ExpiredError"),
            (28, "RateLimitedError"),
            (44, "InvalidSourceReferenceError"),
        ]

        for code, expected_name in test_cases:
            assert code in CONTRACT_ERRORS
            assert CONTRACT_ERRORS[code] == expected_name

    def test_contract_errors_covers_all_typescript_errors(self):
        """Verify Python's CONTRACT_ERRORS covers all error codes from TypeScript SDK."""
        # These are the error codes defined in sdk/typescript/src/types.ts
        ts_error_codes = {
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
            21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 44
        }

        python_error_codes = set(CONTRACT_ERRORS.keys())

        missing_codes = ts_error_codes - python_error_codes
        assert not missing_codes, f"Python CONTRACT_ERRORS missing codes: {missing_codes}"
