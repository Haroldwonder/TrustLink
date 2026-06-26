"""TrustLink Python bindings."""

from .client import TrustLinkClient
from .async_client import AsyncTrustLinkClient
from .types import (
    Attestation,
    AttestationStatus,
    ClaimTypeInfo,
    ContractConfig,
    ContractMetadata,
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
    "ClaimTypeInfo",
    "ContractConfig",
    "ContractMetadata",
    "GlobalStats",
    "IssuerStats",
    "MultiSigProposal",
    "TrustLinkError",
    "ContractError",
]
