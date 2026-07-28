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
    CONTRACT_ERRORS,
    decode_contract_error,
    classify_error_code,
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
    "CONTRACT_ERRORS",
    "decode_contract_error",
    "classify_error_code",
]
