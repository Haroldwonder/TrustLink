#!/usr/bin/env python3
"""
TrustLink Attestation Bundles Example (Python)

This example demonstrates how to create and verify attestation bundles.
Bundles allow issuing multiple related attestations atomically with a shared bundle ID.
"""

import asyncio
import os
import sys
import json
from datetime import datetime, timedelta

from trustlink import (
    TrustLinkClient,
    verify_attestations_in_same_bundle,
    verify_bundle_claim_types,
    verify_bundle_size,
    verify_bundle_subjects,
    verify_bundle_issuer,
    get_bundle_summary,
    group_attestations_by_bundle,
)


# ─── Example 1: Create a KYC Bundle ────────────────────────────────────────


async def example1_create_kyc_bundle():
    """Create a bundle with KYC-related claims."""
    print("=== Example 1: Create a KYC Bundle ===\n")

    client = TrustLinkClient(
        contract_id=os.getenv("CONTRACT_ID", "C..."),
        rpc_url="https://soroban-testnet.stellar.org",
    )

    issuer_address = os.getenv("ISSUER_ADDRESS", "GBRPYHIL2CI3...")
    subject_address = os.getenv("SUBJECT_ADDRESS", "GBBD5YHQVW53...")

    # Create a bundle with KYC-related claims
    expiration = int((datetime.now() + timedelta(days=365)).timestamp())

    bundle_id = client.create_attestation_bundle(
        issuer=issuer_address,
        subject=subject_address,
        claim_types=["KYC_PASSED", "AGE_VERIFIED", "JURISDICTION_US"],
        expiration=expiration,
        metadata=json.dumps({
            "kyc_level": "enhanced",
            "checked_at": datetime.now().isoformat(),
            "check_method": "video_call",
        }),
    )

    print(f"✓ Bundle created: {bundle_id}")
    print(f"  Contains 3 attestations: KYC_PASSED, AGE_VERIFIED, JURISDICTION_US\n")


# ─── Example 2: Verify Bundle Membership ──────────────────────────────────


async def example2_verify_bundle_membership():
    """Verify bundle membership and get bundle details."""
    print("=== Example 2: Verify Bundle Membership ===\n")

    client = TrustLinkClient(
        contract_id=os.getenv("CONTRACT_ID", "C..."),
        rpc_url="https://soroban-testnet.stellar.org",
    )

    bundle_id = sys.argv[2] if len(sys.argv) > 2 else "bundle_id_here"

    # Get bundle details
    bundle = client.get_bundle(bundle_id)

    print(f"Bundle: {bundle['id']}")
    print(f"  Issuer: {bundle['issuer']}")
    print(f"  Subject: {bundle['subject']}")
    print(f"  Created: {datetime.fromtimestamp(bundle['timestamp']).isoformat()}")
    print(f"  Claim Types: {', '.join(bundle['claim_types'])}")
    print(f"  Attestation Count: {len(bundle['attestation_ids'])}")
    print(f"  All Valid: {bundle['all_valid']}\n")

    # Get attestations in bundle
    attestations = client.get_bundle_attestations(bundle_id)

    # Verify all attestations are from the same bundle
    common_bundle_id = verify_attestations_in_same_bundle(attestations)
    if common_bundle_id == bundle_id:
        print("✓ All attestations are from the same bundle\n")
    else:
        print("✗ Attestations are not all from the same bundle\n")

    # Show individual attestations
    print("Individual Attestations:")
    for att in attestations:
        print(f"  - {att['claim_type']}: {att['id']}")
        print(f"    Issuer: {att['issuer']}")
        print(f"    Valid: {not att['revoked']}")
    print()


# ─── Example 3: Verify Bundle Integrity ───────────────────────────────────


async def example3_verify_bundle_integrity():
    """Verify bundle integrity and claim types."""
    print("=== Example 3: Verify Bundle Integrity ===\n")

    client = TrustLinkClient(
        contract_id=os.getenv("CONTRACT_ID", "C..."),
        rpc_url="https://soroban-testnet.stellar.org",
    )

    bundle_id = sys.argv[2] if len(sys.argv) > 2 else "bundle_id_here"

    # Check bundle validity
    is_valid = client.is_bundle_valid(bundle_id)
    print(f"Bundle is valid: {is_valid}")

    # Get bundle details
    bundle = client.get_bundle(bundle_id)

    # Verify expected claim types
    expected_claims = ["KYC_PASSED", "AGE_VERIFIED", "JURISDICTION_US"]
    has_expected_claims = verify_bundle_claim_types(bundle, expected_claims)
    print(f"Has expected claim types: {has_expected_claims}")

    # Verify bundle size
    expected_size = 3
    has_expected_size = verify_bundle_size(bundle, expected_size)
    print(f"Has expected size ({expected_size}): {has_expected_size}\n")


# ─── Example 4: Query Bundles ──────────────────────────────────────────────


async def example4_query_bundles():
    """Query bundles by issuer and subject."""
    print("=== Example 4: Query Bundles ===\n")

    client = TrustLinkClient(
        contract_id=os.getenv("CONTRACT_ID", "C..."),
        rpc_url="https://soroban-testnet.stellar.org",
    )

    issuer_address = os.getenv("ISSUER_ADDRESS", "GBRPYHIL2CI3...")
    subject_address = os.getenv("SUBJECT_ADDRESS", "GBBD5YHQVW53...")

    # Get all bundles created by issuer
    issuer_bundles = client.get_issuer_bundles(issuer_address)
    print(f"Bundles created by issuer: {len(issuer_bundles)}")
    for bundle_id in issuer_bundles[:5]:
        print(f"  - {bundle_id}")
    print()

    # Get all bundles issued to subject
    subject_bundles = client.get_subject_bundles(subject_address)
    print(f"Bundles issued to subject: {len(subject_bundles)}")
    for bundle_id in subject_bundles[:5]:
        print(f"  - {bundle_id}")
    print()


# ─── Example 5: Compliance Bundle Scenario ────────────────────────────────


async def example5_compliance_bundle():
    """Create a compliance bundle with multiple certifications."""
    print("=== Example 5: Compliance Bundle Scenario ===\n")

    client = TrustLinkClient(
        contract_id=os.getenv("CONTRACT_ID", "C..."),
        rpc_url="https://soroban-testnet.stellar.org",
    )

    compliance_issuer = os.getenv("COMPLIANCE_ISSUER", "GBRPYHIL2CI3...")
    organization_address = os.getenv("ORG_ADDRESS", "GBBD5YHQVW53...")

    # Create a compliance bundle with multiple certifications
    expiration = int((datetime.now() + timedelta(days=365)).timestamp())

    bundle_id = client.create_attestation_bundle(
        issuer=compliance_issuer,
        subject=organization_address,
        claim_types=["SOC_2_CERTIFIED", "ISO_27001_APPROVED", "PCI_DSS_COMPLIANT", "GDPR_COMPLIANT"],
        expiration=expiration,
        metadata=json.dumps({
            "audit_date": datetime.now().isoformat(),
            "audit_firm": "Big Four Auditor",
            "compliance_level": "critical_infrastructure",
        }),
    )

    print(f"✓ Compliance bundle created: {bundle_id}")
    print(f"  Contains 4 compliance certifications")
    print(f"  All issued atomically to ensure consistency\n")

    # Verify the compliance bundle
    bundle = client.get_bundle(bundle_id)
    print("Compliance Attestations:")
    for i, claim_type in enumerate(bundle["claim_types"], 1):
        print(f"  {i}. {claim_type}")
        print(f"     ID: {bundle['attestation_ids'][i-1]}")
    print()


# ─── Example 6: Bundle Analysis ────────────────────────────────────────────


async def example6_bundle_analysis():
    """Analyze and group attestations by bundle."""
    print("=== Example 6: Bundle Analysis ===\n")

    client = TrustLinkClient(
        contract_id=os.getenv("CONTRACT_ID", "C..."),
        rpc_url="https://soroban-testnet.stellar.org",
    )

    subject_address = os.getenv("SUBJECT_ADDRESS", "GBBD5YHQVW53...")

    # Get all bundles for a subject
    bundle_ids = client.get_subject_bundles(subject_address)
    print(f"Subject has received {len(bundle_ids)} bundles\n")

    if not bundle_ids:
        print("No bundles found for this subject")
        return

    # Analyze each bundle
    for bundle_id in bundle_ids[:3]:
        print(f"Bundle: {bundle_id}")
        bundle = client.get_bundle(bundle_id)
        summary = get_bundle_summary(bundle)

        print(f"  Issuer: {summary['issuer']}")
        print(f"  Claims: {', '.join(summary['claim_types'])}")
        print(f"  Count: {summary['attestation_count']}")
        print(f"  Valid: {summary['all_valid']}")
        print(f"  Created: {datetime.fromtimestamp(summary['created_at']).isoformat()}")
        print()


# ─── Example 7: Error Handling ─────────────────────────────────────────────


async def example7_error_handling():
    """Demonstrate error handling."""
    print("=== Example 7: Error Handling ===\n")

    client = TrustLinkClient(
        contract_id=os.getenv("CONTRACT_ID", "C..."),
        rpc_url="https://soroban-testnet.stellar.org",
    )

    issuer_address = os.getenv("ISSUER_ADDRESS", "GBRPYHIL2CI3...")
    subject_address = os.getenv("SUBJECT_ADDRESS", "GBBD5YHQVW53...")

    try:
        # Try to create a bundle with too many claims (should fail)
        too_many_claims = [f"CLAIM_{i}" for i in range(60)]
        client.create_attestation_bundle(
            issuer=issuer_address,
            subject=subject_address,
            claim_types=too_many_claims,
        )
    except Exception as error:
        print(f"✓ Caught expected error: {error}")
        print(f"  Bundle size is limited to 50 attestations\n")

    try:
        # Try to create a bundle with same issuer and subject (should fail)
        client.create_attestation_bundle(
            issuer=issuer_address,
            subject=issuer_address,  # Same as issuer - invalid
            claim_types=["CLAIM_TYPE"],
        )
    except Exception as error:
        print(f"✓ Caught expected error: {error}")
        print(f"  Issuers cannot create attestations for themselves\n")


# ─── Main: Run examples ────────────────────────────────────────────────────


async def main():
    """Main entry point for examples."""
    print("TrustLink Attestation Bundles Examples\n")
    print("Select an example to run:\n")
    print("1. Create a KYC Bundle")
    print("2. Verify Bundle Membership")
    print("3. Verify Bundle Integrity")
    print("4. Query Bundles")
    print("5. Compliance Bundle Scenario")
    print("6. Bundle Analysis")
    print("7. Error Handling\n")

    example = sys.argv[1] if len(sys.argv) > 1 else "1"

    try:
        if example == "1":
            await example1_create_kyc_bundle()
        elif example == "2":
            await example2_verify_bundle_membership()
        elif example == "3":
            await example3_verify_bundle_integrity()
        elif example == "4":
            await example4_query_bundles()
        elif example == "5":
            await example5_compliance_bundle()
        elif example == "6":
            await example6_bundle_analysis()
        elif example == "7":
            await example7_error_handling()
        else:
            print(f"Unknown example: {example}")
    except Exception as error:
        print(f"Error: {error}")


if __name__ == "__main__":
    asyncio.run(main())
