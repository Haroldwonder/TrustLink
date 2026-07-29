#!/usr/bin/env python3
"""
TrustLink Server-Side Verification Example

This script demonstrates how to verify attestations on the server side
using the Stellar RPC API. Useful for backend KYC checks before processing
sensitive operations.
"""

import os
import sys
import json
from typing import Optional
import requests
from stellar_sdk import (
    Keypair,
    TransactionBuilder,
    Network,
    Server,
    Address,
    Soroban,
)
from stellar_sdk.soroban import SorobanServer


class TrustLinkVerifier:
    """Verify TrustLink attestations via Stellar RPC."""

    def __init__(
        self,
        rpc_url: str,
        network_passphrase: str,
        contract_id: str,
    ):
        self.rpc_url = rpc_url
        self.network_passphrase = network_passphrase
        self.contract_id = contract_id
        self.server = SorobanServer(rpc_url)

    def verify_claim(
        self,
        subject_address: str,
        claim_type: str,
    ) -> bool:
        """
        Verify if a subject has a valid claim of the given type.

        Args:
            subject_address: Stellar address of the subject
            claim_type: Type of claim to verify (e.g., "KYC_PASSED")

        Returns:
            True if subject has a valid claim, False otherwise
        """
        try:
            # Build the contract call
            contract = Soroban.contract(self.contract_id)
            call_builder = contract.call(
                "has_valid_claim",
                Address(subject_address),
                claim_type,
            )

            # Create a dummy transaction for simulation
            source = Keypair.random()
            account = self.server.get_account(source.public_key)
            tx = (
                TransactionBuilder(
                    account,
                    base_fee=100,
                    network_passphrase=self.network_passphrase,
                )
                .add_text_memo("verify")
                .set_timeout(30)
                .append_invoke_host_function_op(
                    host_function=call_builder,
                    auth=[],
                )
                .build()
            )

            # Simulate the transaction
            sim_response = self.server.simulate_transaction(tx)

            # Extract result
            if hasattr(sim_response, "result") and sim_response.result:
                result_xdr = sim_response.result.return_value
                # Parse the XDR result (true/false)
                return self._parse_bool_result(result_xdr)

            return False

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
        Verify if a subject has a valid claim from a specific issuer.

        Args:
            subject_address: Stellar address of the subject
            claim_type: Type of claim to verify
            issuer_address: Stellar address of the issuer

        Returns:
            True if subject has a valid claim from the issuer, False otherwise
        """
        try:
            contract = Soroban.contract(self.contract_id)
            call_builder = contract.call(
                "has_valid_claim_from_issuer",
                Address(subject_address),
                claim_type,
                Address(issuer_address),
            )

            source = Keypair.random()
            account = self.server.get_account(source.public_key)
            tx = (
                TransactionBuilder(
                    account,
                    base_fee=100,
                    network_passphrase=self.network_passphrase,
                )
                .add_text_memo("verify_issuer")
                .set_timeout(30)
                .append_invoke_host_function_op(
                    host_function=call_builder,
                    auth=[],
                )
                .build()
            )

            sim_response = self.server.simulate_transaction(tx)

            if hasattr(sim_response, "result") and sim_response.result:
                result_xdr = sim_response.result.return_value
                return self._parse_bool_result(result_xdr)

            return False

        except Exception as e:
            print(f"Error verifying claim from issuer: {e}", file=sys.stderr)
            return False

    def get_attestation_status(self, attestation_id: str) -> Optional[str]:
        """
        Get the status of an attestation.

        Args:
            attestation_id: ID of the attestation

        Returns:
            Status string ("Valid", "Expired", "Revoked") or None on error
        """
        try:
            contract = Soroban.contract(self.contract_id)
            call_builder = contract.call(
                "get_attestation_status",
                attestation_id,
            )

            source = Keypair.random()
            account = self.server.get_account(source.public_key)
            tx = (
                TransactionBuilder(
                    account,
                    base_fee=100,
                    network_passphrase=self.network_passphrase,
                )
                .add_text_memo("status")
                .set_timeout(30)
                .append_invoke_host_function_op(
                    host_function=call_builder,
                    auth=[],
                )
                .build()
            )

            sim_response = self.server.simulate_transaction(tx)

            if hasattr(sim_response, "result") and sim_response.result:
                result_xdr = sim_response.result.return_value
                return self._parse_string_result(result_xdr)

            return None

        except Exception as e:
            print(f"Error getting attestation status: {e}", file=sys.stderr)
            return None

    @staticmethod
    def _parse_bool_result(xdr: str) -> bool:
        """Parse a boolean result from XDR."""
        # Simplified parsing - in production, use proper XDR decoding
        return "01" in xdr.lower()

    @staticmethod
    def _parse_string_result(xdr: str) -> str:
        """Parse a string result from XDR."""
        # Simplified parsing - in production, use proper XDR decoding
        if "56616c6964" in xdr:  # "Valid" in hex
            return "Valid"
        elif "45787069726564" in xdr:  # "Expired" in hex
            return "Expired"
        elif "52657661" in xdr:  # "Revoked" in hex
            return "Revoked"
        return "Unknown"


def main():
    """Run verification examples."""
    # Configuration from environment
    rpc_url = os.getenv(
        "RPC_URL",
        "https://soroban-testnet.stellar.org",
    )
    network_passphrase = os.getenv(
        "NETWORK_PASSPHRASE",
        "Test SDF Network ; September 2015",
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
        print("   ✓ Subject has valid KYC - proceed with operation")
    else:
        print("   ✗ Subject lacks valid KYC - deny operation")

    # Example 2: Verify claim from specific issuer
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
    claim_types = ["KYC_PASSED", "AML_CLEARED"]
    for claim_type in claim_types:
        has_claim = verifier.verify_claim(subject_address, claim_type)
        status = "✓" if has_claim else "✗"
        print(f"   {status} {claim_type}: {has_claim}")

    print("\n=== Verification Complete ===")


if __name__ == "__main__":
    main()
