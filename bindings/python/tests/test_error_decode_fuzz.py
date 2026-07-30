"""Property / differential fuzz tests for TrustLink error decoding.

Asserts every known contract error code (from sdk/error-codes.json) and a
stream of random edge-case codes decode to the expected classification.
Paired with sdk/typescript/__tests__/errorDecode.fuzz.test.ts and
scripts/diff-fuzz-error-decode.sh for cross-SDK equivalence.
"""

from __future__ import annotations

import importlib.util
import json
import random
from pathlib import Path

import pytest

# Load types.py directly so we don't require stellar_sdk (imported by package __init__).
_TYPES_PATH = Path(__file__).resolve().parents[1] / "trustlink" / "types.py"
_spec = importlib.util.spec_from_file_location("trustlink_types_fuzz", _TYPES_PATH)
_types = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(_types)

CONTRACT_ERRORS = _types.CONTRACT_ERRORS
classify_error_code = _types.classify_error_code
decode_contract_error = _types.decode_contract_error

CORPUS_PATH = Path(__file__).resolve().parents[3] / "sdk" / "error-codes.json"
CORPUS = json.loads(CORPUS_PATH.read_text())
ERRORS = CORPUS["errors"]


def test_sdk_error_table_matches_shared_corpus():
    expected = {e["code"]: e["name"] for e in ERRORS}
    assert CONTRACT_ERRORS == expected

    for entry in ERRORS:
        code, name = entry["code"], entry["name"]
        assert classify_error_code(code) == name
        parsed = decode_contract_error(f"Error(Contract, #{code})")
        assert parsed is not None
        assert parsed.code == code
        assert parsed.message == name


def test_fuzz_random_and_edge_case_codes():
    known = {e["code"]: e["name"] for e in ERRORS}
    rng = random.Random(0x7E57110)  # fixed seed for reproducible CI failures

    samples = [e["code"] for e in ERRORS] + [
        0,
        -1,
        31,
        43,
        45,
        100,
        255,
        1000,
        2**31 - 1,
    ]
    samples.extend(rng.randrange(512) for _ in range(500))

    for code in samples:
        expected = known.get(code)
        assert classify_error_code(code) == expected

        msg = f"HostError: Error(Contract, #{code})"
        parsed = decode_contract_error(msg)
        if expected is None:
            assert parsed is None
        else:
            assert parsed.message == expected
            assert parsed.code == code


@pytest.mark.parametrize("entry", ERRORS, ids=lambda e: e["name"])
def test_each_corpus_entry_decodes(entry):
    code, name = entry["code"], entry["name"]
    assert decode_contract_error(f"Error(Contract, #{code})").message == name
