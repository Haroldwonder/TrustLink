#!/usr/bin/env python3
"""
Per-Subject Attestation Limit Example

This example demonstrates how to use TrustLink's per-subject attestation limit
feature to prevent unbounded growth and query performance degradation.

Scenarios covered:
1. Setting the per-subject limit
2. Creating attestations up to the limit
3. Handling limit-exceeded errors
4. Querying the current limit
5. Disabling the limit
6. Creating bundles with per-subject limit enforcement
"""

import asyncio
import time
from trustlink import TrustLinkClient
from trustlink.types import Address
from stellar_sdk import Keypair, Network


# Setup: Initialize client and accounts
async def setup_example():
    """Initialize client and create test accounts."""
    # Contract configuration
    contract_id = "C..."  # Replace with your contract ID
    rpc_url = "https://soroban-testnet.stellar.org"
    network_passphrase = Network.TESTNET_NETWORK_PASSPHRASE

    # Create keypairs for participants
    admin_keypair = Keypair.random()
    issuer_keypair = Keypair.random()
    subject_keypair = Keypair.random()

    # Extract addresses
    admin_address = admin_keypair.public_key
    issuer_address = issuer_keypair.public_key
    subject_address = subject_keypair.public_key

    # Initialize client
    client = TrustLinkClient(
        contract_id=contract_id,
        rpc_url=rpc_url,
        network_passphrase=network_passphrase,
    )

    return {
        "client": client,
        "admin_address": admin_address,
        "issuer_address": issuer_address,
        "subject_address": subject_address,
        "admin_keypair": admin_keypair,
        "issuer_keypair": issuer_keypair,
        "subject_keypair": subject_keypair,
    }


# Scenario 1: Set the per-subject attestation limit
async def scenario1_set_limit():
    """Scenario 1: Set the per-subject attestation limit."""
    print("\n=== Scenario 1: Set Per-Subject Limit ===")
    setup = await setup_example()
    client = setup["client"]
    admin_address = setup["admin_address"]
    admin_keypair = setup["admin_keypair"]

    try:
        # Set limit to 100 attestations per subject
        result = await client.set_max_attestations_per_subject(
            admin=admin_address,
            limit=100,
            signers=[admin_keypair],
        )

        print("✓ Set per-subject limit to 100 attestations")
        print(f"  Transaction result: {result.get('id', 'success')}")

        # Verify the limit was set
        limit = await client.get_max_attestations_per_subject()
        print(f"✓ Verified limit: {limit} attestations per subject")

    except Exception as error:
        print(f"✗ Failed to set limit: {error}")
        raise


# Scenario 2: Create attestations up to the limit
async def scenario2_create_attestations_up_to_limit():
    """Scenario 2: Create attestations up to the limit."""
    print("\n=== Scenario 2: Create Attestations Up To Limit ===")
    setup = await setup_example()
    client = setup["client"]
    admin_address = setup["admin_address"]
    issuer_address = setup["issuer_address"]
    subject_address = setup["subject_address"]
    admin_keypair = setup["admin_keypair"]
    issuer_keypair = setup["issuer_keypair"]

    try:
        # First, set a small limit for demonstration
        await client.set_max_attestations_per_subject(
            admin=admin_address,
            limit=5,  # Small limit for this example
            signers=[admin_keypair],
        )
        print("✓ Set per-subject limit to 5 attestations (for demo)")

        # Create 5 attestations (up to limit)
        attestation_ids = []
        for i in range(1, 6):
            attestation_id = await client.create_attestation(
                issuer=issuer_address,
                subject=subject_address,
                claim_type=f"CREDENTIAL_{i}",
                expiration=None,
                metadata=f'{{"index":{i}}}',
                tags=[f"demo-{i}"],
                signers=[issuer_keypair],
            )
            attestation_ids.append(attestation_id)
            print(f"✓ Created attestation {i}/5: {attestation_id[:8]}...")

        print(f"\n✓ Successfully created 5 attestations (limit: 5, used: 5/5)")
        return attestation_ids

    except Exception as error:
        print(f"✗ Failed to create attestations: {error}")
        raise


# Scenario 3: Attempt to exceed the limit
async def scenario3_handle_limit_exceeded():
    """Scenario 3: Attempt to exceed the limit and handle error."""
    print("\n=== Scenario 3: Handle Limit-Exceeded Error ===")
    setup = await setup_example()
    client = setup["client"]
    admin_address = setup["admin_address"]
    issuer_address = setup["issuer_address"]
    subject_address = setup["subject_address"]
    admin_keypair = setup["admin_keypair"]
    issuer_keypair = setup["issuer_keypair"]

    try:
        # Set limit to 2 for easy demonstration
        await client.set_max_attestations_per_subject(
            admin=admin_address,
            limit=2,
            signers=[admin_keypair],
        )
        print("✓ Set per-subject limit to 2 attestations")

        # Create 2 attestations (at limit)
        for i in range(1, 3):
            await client.create_attestation(
                issuer=issuer_address,
                subject=subject_address,
                claim_type=f"CLAIM_{i}",
                expiration=None,
                metadata=None,
                tags=None,
                signers=[issuer_keypair],
            )
            print(f"✓ Created attestation {i}/2")

        # Attempt to create 3rd attestation (exceeds limit)
        print("\n→ Attempting to exceed limit (3rd attestation)...")
        try:
            await client.create_attestation(
                issuer=issuer_address,
                subject=subject_address,
                claim_type="CLAIM_3",
                expiration=None,
                metadata=None,
                tags=None,
                signers=[issuer_keypair],
            )
            print("✗ ERROR: Should have rejected 3rd attestation but succeeded!")
        except Exception as limit_error:
            if "LimitExceeded" in str(limit_error):
                print("✓ Correctly rejected 3rd attestation (limit exceeded)")
                print(f"  Error: {limit_error}")
            else:
                raise

    except Exception as error:
        print(f"✗ Scenario failed: {error}")
        raise


# Scenario 4: Query the current limit
async def scenario4_query_current_limit():
    """Scenario 4: Query the current limit."""
    print("\n=== Scenario 4: Query Current Limit ===")
    setup = await setup_example()
    client = setup["client"]
    admin_address = setup["admin_address"]
    admin_keypair = setup["admin_keypair"]

    try:
        # Initially check limit (should be unlimited)
        limit = await client.get_max_attestations_per_subject()
        print(f"✓ Initial limit: {limit or 'unlimited'}")

        # Set a limit
        await client.set_max_attestations_per_subject(
            admin=admin_address,
            limit=500,
            signers=[admin_keypair],
        )

        # Query after setting
        limit = await client.get_max_attestations_per_subject()
        print(f"✓ After setting: {limit} attestations per subject")

        # Query via contract config
        config = await client.get_config()
        print(f"✓ Via config: {config.get('max_attestations_per_subject')}")

    except Exception as error:
        print(f"✗ Failed to query limit: {error}")
        raise


# Scenario 5: Disable the limit (set to unlimited)
async def scenario5_disable_limit():
    """Scenario 5: Disable the per-subject limit."""
    print("\n=== Scenario 5: Disable Per-Subject Limit ===")
    setup = await setup_example()
    client = setup["client"]
    admin_address = setup["admin_address"]
    admin_keypair = setup["admin_keypair"]

    try:
        # Set initial limit
        await client.set_max_attestations_per_subject(
            admin=admin_address,
            limit=100,
            signers=[admin_keypair],
        )
        print("✓ Set limit to 100")

        # Verify it's set
        limit = await client.get_max_attestations_per_subject()
        print(f"✓ Current limit: {limit}")

        # Disable (set to null/None)
        await client.set_max_attestations_per_subject(
            admin=admin_address,
            limit=None,
            signers=[admin_keypair],
        )
        print("✓ Disabled per-subject limit (set to null)")

        # Verify it's disabled
        limit = await client.get_max_attestations_per_subject()
        print(f"✓ Limit after disabling: {limit or 'unlimited'}")

    except Exception as error:
        print(f"✗ Failed to disable limit: {error}")
        raise


# Scenario 6: Create bundle with per-subject limit enforcement
async def scenario6_bundle_with_limit_enforcement():
    """Scenario 6: Create bundle with per-subject limit enforcement."""
    print("\n=== Scenario 6: Bundle Creation With Limit Enforcement ===")
    setup = await setup_example()
    client = setup["client"]
    admin_address = setup["admin_address"]
    issuer_address = setup["issuer_address"]
    subject_address = setup["subject_address"]
    admin_keypair = setup["admin_keypair"]
    issuer_keypair = setup["issuer_keypair"]

    try:
        # Set limit to 5
        await client.set_max_attestations_per_subject(
            admin=admin_address,
            limit=5,
            signers=[admin_keypair],
        )
        print("✓ Set per-subject limit to 5")

        # Try to create bundle with 3 claims (within limit)
        claim_types = ["KYC_PASSED", "AGE_VERIFIED", "JURISDICTION_US"]
        expiration = int(time.time()) + (365 * 24 * 60 * 60)

        bundle_id = await client.create_attestation_bundle(
            issuer=issuer_address,
            subject=subject_address,
            claim_types=claim_types,
            expiration=expiration,
            metadata='{"bundle_type":"kyc"}',
            tags=["kyc_bundle"],
            signers=[issuer_keypair],
        )
        print(f"✓ Created bundle with 3 claims: {bundle_id[:8]}...")

        # Try to create another bundle with 3 claims (would total 6, exceeds 5)
        print("\n→ Attempting to create another 3-claim bundle (would exceed limit)...")
        try:
            await client.create_attestation_bundle(
                issuer=issuer_address,
                subject=subject_address,
                claim_types=["DOCUMENT_VERIFIED", "INCOME_VERIFIED", "CREDIT_CHECK"],
                expiration=expiration,
                metadata='{"bundle_type":"compliance"}',
                tags=["compliance_bundle"],
                signers=[issuer_keypair],
            )
            print("✗ ERROR: Should have rejected second bundle!")
        except Exception as limit_error:
            if "LimitExceeded" in str(limit_error):
                print("✓ Correctly rejected second bundle (would exceed limit)")
                print(f"  Current: 3/5, Requested: 3 more = 6/5 (exceeds!)")
            else:
                raise

    except Exception as error:
        print(f"✗ Scenario failed: {error}")
        raise


# Run all scenarios
async def main():
    """Run all scenarios."""
    print("╔════════════════════════════════════════════════════════════╗")
    print("║  TrustLink Per-Subject Attestation Limit Examples          ║")
    print("╚════════════════════════════════════════════════════════════╝")

    try:
        await scenario1_set_limit()
        await scenario2_create_attestations_up_to_limit()
        await scenario3_handle_limit_exceeded()
        await scenario4_query_current_limit()
        await scenario5_disable_limit()
        await scenario6_bundle_with_limit_enforcement()

        print("\n╔════════════════════════════════════════════════════════════╗")
        print("║  ✓ All scenarios completed successfully!                  ║")
        print("╚════════════════════════════════════════════════════════════╝")

    except Exception as error:
        print(f"\n✗ Example failed: {error}")
        raise


if __name__ == "__main__":
    asyncio.run(main())
