"""TrustLink Python bindings."""

from .client import TrustLinkClient
from .types import (
    Attestation,
    AttestationStatus,
    ClaimTypeInfo,
    GlobalStats,
    IssuerStats,
    MultiSigProposal,
    TrustLinkError,
    ContractError,
)

__version__ = "0.1.0"
__all__ = [
    "TrustLinkClient",
    "Attestation",
    "AttestationStatus",
    "ClaimTypeInfo",
    "GlobalStats",
    "IssuerStats",
    "MultiSigProposal",
    "TrustLinkError",
    "ContractError",
]
