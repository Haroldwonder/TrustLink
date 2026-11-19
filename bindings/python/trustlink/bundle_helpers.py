"""TrustLink Bundle Helpers — Python SDK

Helper methods for creating and verifying attestation bundles.
Bundles allow issuing multiple related attestations atomically with a shared bundle ID.
"""

from typing import List, Optional, Dict, Any
from .types import Attestation, AttestationBundle


class BundleOptions:
    """Options for creating an attestation bundle."""
    
    def __init__(
        self,
        issuer: str,
        subject: str,
        claim_types: List[str],
        expiration: Optional[int] = None,
        metadata: Optional[str] = None,
        tags: Optional[List[str]] = None,
    ):
        """Initialize bundle creation options.
        
        Args:
            issuer: Issuer address (must be authorized)
            subject: Subject address receiving the attestations
            claim_types: List of claim types to issue (order determines bundle ID)
            expiration: Optional expiration time (applied to all attestations)
            metadata: Optional metadata (shared across attestations)
            tags: Optional tags (applied to all attestations)
        """
        self.issuer = issuer
        self.subject = subject
        self.claim_types = claim_types
        self.expiration = expiration
        self.metadata = metadata
        self.tags = tags


def verify_attestations_in_same_bundle(
    attestations: List[Attestation],
) -> Optional[str]:
    """Verify that a set of attestations belong to the same bundle.
    
    Args:
        attestations: Array of attestations to verify
        
    Returns:
        The common bundle ID if all share one, or None if not all in same bundle
        
    Example:
        >>> bundle_id = verify_attestations_in_same_bundle(attestations)
        >>> if bundle_id:
        ...     print(f"All attestations are from bundle: {bundle_id}")
    """
    if not attestations:
        return None
    
    first_bundle_id = attestations[0].get("bundle_id")
    if not first_bundle_id:
        return None
    
    for attestation in attestations:
        if attestation.get("bundle_id") != first_bundle_id:
            return None
    
    return first_bundle_id


def verify_bundle_claim_types(
    bundle: AttestationBundle,
    expected_claim_types: List[str],
) -> bool:
    """Verify a bundle contains expected claim types.
    
    Args:
        bundle: The bundle to verify
        expected_claim_types: Expected claim types (order matters)
        
    Returns:
        True if bundle contains exactly the expected claim types in order
        
    Example:
        >>> is_expected = verify_bundle_claim_types(
        ...     bundle,
        ...     ["KYC_PASSED", "AGE_VERIFIED"]
        ... )
        >>> print(f"Bundle has expected claim types: {is_expected}")
    """
    if len(bundle["claim_types"]) != len(expected_claim_types):
        return False
    
    for i, claim_type in enumerate(bundle["claim_types"]):
        if claim_type != expected_claim_types[i]:
            return False
    
    return True


def verify_bundle_size(bundle: AttestationBundle, expected_count: int) -> bool:
    """Verify a bundle contains expected number of attestations.
    
    Args:
        bundle: The bundle to verify
        expected_count: Expected number of attestations
        
    Returns:
        True if bundle has exactly the expected number of attestations
        
    Example:
        >>> is_expected = verify_bundle_size(bundle, 3)
        >>> print(f"Bundle has expected size: {is_expected}")
    """
    return (
        len(bundle["attestation_ids"]) == expected_count
        and len(bundle["claim_types"]) == expected_count
    )


def verify_bundle_subjects(
    bundle: AttestationBundle,
    attestations: List[Attestation],
) -> bool:
    """Verify all attestations in a bundle are for the same subject.
    
    Args:
        bundle: The bundle metadata
        attestations: The attestations to verify
        
    Returns:
        True if all attestations are issued to the bundle's subject
    """
    if not attestations:
        return True
    
    for attestation in attestations:
        if attestation["subject"] != bundle["subject"]:
            return False
    
    return True


def verify_bundle_issuer(
    bundle: AttestationBundle,
    attestations: List[Attestation],
) -> bool:
    """Verify all attestations in a bundle are from the same issuer.
    
    Args:
        bundle: The bundle metadata
        attestations: The attestations to verify
        
    Returns:
        True if all attestations are from the bundle's issuer
    """
    if not attestations:
        return True
    
    for attestation in attestations:
        if attestation["issuer"] != bundle["issuer"]:
            return False
    
    return True


def get_bundle_summary(bundle: AttestationBundle) -> Dict[str, Any]:
    """Get a human-readable summary of a bundle.
    
    Args:
        bundle: The bundle to summarize
        
    Returns:
        Dictionary with bundle summary information
        
    Example:
        >>> summary = get_bundle_summary(bundle)
        >>> print(f"Bundle {summary['id']} from {summary['issuer']}")
        >>> print(f"  To: {summary['subject']}")
        >>> print(f"  Claims: {', '.join(summary['claim_types'])}")
        >>> print(f"  Valid: {summary['all_valid']}")
    """
    return {
        "id": bundle["id"],
        "issuer": bundle["issuer"],
        "subject": bundle["subject"],
        "claim_types": bundle["claim_types"],
        "attestation_count": len(bundle["attestation_ids"]),
        "all_valid": bundle["all_valid"],
        "created_at": bundle["timestamp"],
    }


def group_attestations_by_bundle(
    attestations: List[Attestation],
) -> Dict[Optional[str], List[Attestation]]:
    """Group attestations by their bundle ID.
    
    Args:
        attestations: List of attestations to group
        
    Returns:
        Dictionary mapping bundle IDs to lists of attestations
        
    Example:
        >>> groups = group_attestations_by_bundle(attestations)
        >>> for bundle_id, atts in groups.items():
        ...     print(f"Bundle {bundle_id}: {len(atts)} attestations")
    """
    groups: Dict[Optional[str], List[Attestation]] = {}
    
    for attestation in attestations:
        bundle_id = attestation.get("bundle_id")
        if bundle_id not in groups:
            groups[bundle_id] = []
        groups[bundle_id].append(attestation)
    
    return groups
