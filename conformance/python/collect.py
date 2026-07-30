#!/usr/bin/env python3
"""Collect conformance observations from the Python bindings client."""

from __future__ import annotations

import json
import re
import sys
from typing import Any

from trustlink import TrustLinkClient
from trustlink.types import ContractError, TrustLinkError


def normalize_attestation_ids(value: Any) -> list[str]:
    if not isinstance(value, list):
        return []
    ids: list[str] = []
    for entry in value:
        if isinstance(entry, str):
            ids.append(entry)
        elif isinstance(entry, dict) and "id" in entry:
            ids.append(str(entry["id"]))
        else:
            ids.append(str(entry))
    return ids


def parse_error_observation(step: str, error: Exception) -> dict[str, Any]:
    if isinstance(error, ContractError):
        return {
            "step": step,
            "kind": "error",
            "code": error.code,
            "name": error.message,
        }

    message = str(error)
    match = re.search(r"Error\(Contract,\s*#(\d+)\)", message)
    if match:
        code = int(match.group(1))
        return {"step": step, "kind": "error", "code": code, "name": f"ContractError({code})"}

    raise RuntimeError(f"Unable to normalize error for step {step}: {message}") from error


def main() -> None:
    ctx = json.load(sys.stdin)
    client = TrustLinkClient(
        contract_id=ctx["contractId"],
        rpc_url=ctx["rpcUrl"],
        network_passphrase=ctx["networkPassphrase"],
    )

    observations: list[dict[str, Any]] = [
        {
            "step": "has_valid_claim_existing",
            "kind": "boolean",
            "value": client.has_valid_claim(ctx["subject"], "KYC_PASSED"),
        },
        {
            "step": "has_valid_claim_missing",
            "kind": "boolean",
            "value": client.has_valid_claim(ctx["subject"], "ACCREDITED_INVESTOR"),
        },
        {
            "step": "get_subject_attestations",
            "kind": "ids",
            "value": normalize_attestation_ids(
                client.get_subject_attestations(ctx["subject"], 0, 10)
            ),
        },
    ]

    try:
        client.get_attestation("nonexistent-conformance-id")
        raise RuntimeError("expected get_attestation to fail")
    except (ContractError, TrustLinkError) as error:
        observations.append(parse_error_observation("get_attestation_not_found", error))

    json.dump(observations, sys.stdout)


if __name__ == "__main__":
    main()
