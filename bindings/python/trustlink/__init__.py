"""TrustLink Python bindings."""

from .client import TrustLinkClient
from .async_client import AsyncTrustLinkClient
from .types import (
    Attestation,
    AttestationStatus,
    AttestationTemplate,
    ClaimTypeInfo,
    ContractConfig,
    ContractMetadata,
    Delegation,
    GlobalStats,
    IssuerStats,
    MultiSigProposal,
    TrustLinkError,
    ContractError,
)

__version__ = "0.1.0"
__all__ = [
    "TrustLinkClient",
    "AsyncTrustLinkClient",
    "Attestation",
    "AttestationStatus",
    "AttestationTemplate",
    "ClaimTypeInfo",
    "ContractConfig",
    "ContractMetadata",
    "Delegation",
    "GlobalStats",
    "IssuerStats",
    "MultiSigProposal",
    "TrustLinkError",
    "ContractError",
]
