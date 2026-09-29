#!/usr/bin/env python3
"""
TrustLink Server-Side Verification Example

This script demonstrates how to verify attestations on the server side
using the Stellar Soroban RPC API.  Useful for backend KYC checks before
processing sensitive operations.

Requires stellar-sdk >= 21 (Soroban / Protocol 20+).
"""

import os
import sys
from typing import Optional

from stellar_sdk import (
    Keypair,
    Network,
    TransactionBuilder,
)
from stellar_sdk.contract import ContractClient
from stellar_sdk.soroban_rpc import SorobanServer
from stellar_sdk import scval


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_server(rpc_url: str) -> SorobanServer:
    """Return a SorobanServer for the given RPC endpoint."""
    return SorobanServer(rpc_url)


def _simulate_bool(
    server: SorobanServer,
    network_passphrase: str,
    contract_id: str,
    function_name: str,
    args: list,
) -> bool:
    """
    Build a transaction that invokes *function_name* on the TrustLink
    contract, simulate it, and return the decoded boolean result.

    A random keypair is used as the transaction source because the calls are
    read-only simulations — no signature or real account is needed.
    """
    source_keypair = Keypair.random()
    # stellar_sdk >= 21: SorobanServer.load_account returns a BaseAccount
    source_account = server.load_account(source_keypair.public_key)

    client = ContractClient(contract_id, server)
    invocation = client.invoke(
        function_name,
        args,
        source=source_keypair.public_key,
        base_fee=100,
        transaction_timeout=30,
    )

    sim = server.simulate_transaction(invocation.transaction)

    if hasattr(sim, "error") and sim.error:
        raise RuntimeError(f"Simulation error: {sim.error}")

    if sim.results and len(sim.results) > 0:
        result_val = sim.results[0].xdr
        # stellar_sdk.scval.to_native decodes SCVal XDR to a Python value
        decoded = scval.to_native(result_val)
        return bool(decoded)

    return False


def _simulate_string(
    server: SorobanServer,
    network_passphrase: str,
    contract_id: str,
    function_name: str,
    args: list,
) -> Optional[str]:
    """
    Like _simulate_bool but returns the decoded string result, or None on
    missing/error.
    """
    source_keypair = Keypair.random()
    source_account = server.load_account(source_keypair.public_key)

    client = ContractClient(contract_id, server)
    invocation = client.invoke(
        function_name,
        args,
        source=source_keypair.public_key,
        base_fee=100,
        transaction_timeout=30,
    )

    sim = server.simulate_transaction(invocation.transaction)

    if hasattr(sim, "error") and sim.error:
        raise RuntimeError(f"Simulation error: {sim.error}")

    if sim.results and len(sim.results) > 0:
        result_val = sim.results[0].xdr
        decoded = scval.to_native(result_val)
        if decoded is None:
            return None
        # Enum variants (e.g. AttestationStatus) decode as dicts like {"Valid": None}
        if isinstance(decoded, dict):
            return next(iter(decoded), str(decoded))
        return str(decoded)

    return None


# ---------------------------------------------------------------------------
# Verifier
# ---------------------------------------------------------------------------

class TrustLinkVerifier:
    """Verify TrustLink attestations via Stellar Soroban RPC."""

    def __init__(
        self,
        rpc_url: str,
        network_passphrase: str,
        contract_id: str,
    ):
        self.rpc_url = rpc_url
        self.network_passphrase = network_passphrase
        self.contract_id = contract_id
        self.server = _make_server(rpc_url)

    def verify_claim(
        self,
        subject_address: str,
        claim_type: str,
    ) -> bool:
        """
        Return True if *subject_address* holds a valid claim of *claim_type*.

        Calls the TrustLink contract's ``has_valid_claim`` function via a
        read-only simulation — no on-chain transaction is submitted.

        Args:
            subject_address: Stellar address of the subject.
            claim_type:       Claim type identifier, e.g. ``"KYC_PASSED"``.

        Returns:
            True if the subject has a valid claim, False otherwise.
        """
        try:
            return _simulate_bool(
                self.server,
                self.network_passphrase,
                self.contract_id,
                "has_valid_claim",
                [
                    scval.to_address(subject_address),
                    scval.to_string(claim_type),
                ],
            )
        except Exception as e:
            print(f"Error verifying claim: {e}", file=sys.stderr)
            return False

    def verify_claim_from_issuer(
        self,
        subject_address: str,
        claim_type: str,
        issuer_address: str,
    ) -> bool:
        """
        Return True if *subject_address* holds a valid *claim_type* issued
        specifically by *issuer_address*.

        Args:
            subject_address: Stellar address of the subject.
            claim_type:       Claim type identifier.
            issuer_address:   Stellar address of the required issuer.

        Returns:
            True if the subject has a valid claim from that issuer, False
            otherwise.
        """
        try:
            return _simulate_bool(
                self.server,
                self.network_passphrase,
                self.contract_id,
                "has_valid_claim_from_issuer",
                [
                    scval.to_address(subject_address),
                    scval.to_string(claim_type),
                    scval.to_address(issuer_address),
                ],
            )
        except Exception as e:
            print(f"Error verifying claim from issuer: {e}", file=sys.stderr)
            return False

    def get_attestation_status(self, attestation_id: str) -> Optional[str]:
        """
        Return the status of an attestation: ``"Valid"``, ``"Expired"``, or
        ``"Revoked"``.  Returns None on error or when the attestation is not
        found.

        Args:
            attestation_id: TrustLink attestation ID string.

        Returns:
            Status string or None.
        """
        try:
            return _simulate_string(
                self.server,
                self.network_passphrase,
                self.contract_id,
                "get_attestation_status",
                [scval.to_string(attestation_id)],
            )
        except Exception as e:
            print(f"Error getting attestation status: {e}", file=sys.stderr)
            return None


# ---------------------------------------------------------------------------
# CLI entry-point
# ---------------------------------------------------------------------------

def main():
    """Run verification examples using values from environment variables."""
    rpc_url = os.getenv("RPC_URL", "https://soroban-testnet.stellar.org")
    network_passphrase = os.getenv(
        "NETWORK_PASSPHRASE",
        Network.TESTNET_NETWORK_PASSPHRASE,
    )
    contract_id = os.getenv("TRUSTLINK_CONTRACT_ID", "")
    subject_address = os.getenv("SUBJECT_ADDRESS", "")
    issuer_address = os.getenv("ISSUER_ADDRESS", "")

    if not contract_id:
        print("Error: TRUSTLINK_CONTRACT_ID environment variable not set")
        sys.exit(1)

    if not subject_address:
        print("Error: SUBJECT_ADDRESS environment variable not set")
        sys.exit(1)

    print("=== TrustLink Server-Side Verification ===\n")

    verifier = TrustLinkVerifier(rpc_url, network_passphrase, contract_id)

    # Example 1: Verify any valid KYC claim
    print("1) Checking if subject has valid KYC_PASSED claim...")
    has_kyc = verifier.verify_claim(subject_address, "KYC_PASSED")
    print(f"   Result: {has_kyc}")
    if has_kyc:
        print("   ✓ Subject has valid KYC — proceed with operation")
    else:
        print("   ✗ Subject lacks valid KYC — deny operation")

    # Example 2: Verify claim from a specific issuer
    if issuer_address:
        print(f"\n2) Checking if subject has KYC from specific issuer...")
        has_issuer_kyc = verifier.verify_claim_from_issuer(
            subject_address,
            "KYC_PASSED",
            issuer_address,
        )
        print(f"   Result: {has_issuer_kyc}")
        if has_issuer_kyc:
            print("   ✓ Subject has KYC from trusted issuer")
        else:
            print("   ✗ Subject lacks KYC from this issuer")

    # Example 3: Check multiple claim types
    print("\n3) Checking multiple claim types...")
    for claim_type in ["KYC_PASSED", "AML_CLEARED"]:
        has_claim = verifier.verify_claim(subject_address, claim_type)
        status = "✓" if has_claim else "✗"
        print(f"   {status} {claim_type}: {has_claim}")

    print("\n=== Verification Complete ===")


if __name__ == "__main__":
    main()
