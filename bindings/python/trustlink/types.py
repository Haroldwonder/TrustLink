"""Type definitions for TrustLink contract."""

from typing import TypedDict, Optional, List, Literal

AttestationStatus = Literal["Valid", "Expired", "Revoked"]


class Attestation(TypedDict):
    """Attestation record."""
    id: str
    issuer: str
    subject: str
    claim_type: str
    timestamp: int
    expiration: Optional[int]
    revoked: bool
    metadata: Optional[str]
    imported: bool
    bridged: bool
    source_chain: Optional[str]
    source_tx: Optional[str]


class ClaimTypeInfo(TypedDict):
    """Claim type registry entry."""
    claim_type: str
    description: str


class GlobalStats(TypedDict):
    """Contract-wide statistics."""
    total_attestations: int
    total_revocations: int
    total_issuers: int


class IssuerStats(TypedDict):
    """Per-issuer statistics."""
    total_issued: int
    active: int
    revoked: int
    expired: int


class MultiSigProposal(TypedDict):
    """Multi-signature attestation proposal."""
    id: str
    proposer: str
    subject: str
    claim_type: str
    required_signers: List[str]
    signers: List[str]
    threshold: int
    expires_at: int
    finalized: bool


class TrustLinkError(Exception):
    """Base exception for TrustLink SDK errors."""
    pass


class ContractError(TrustLinkError):
    """Contract execution error."""
    def __init__(self, code: int, message: str):
        self.code = code
        self.message = message
        super().__init__(f"Contract error #{code}: {message}")


# Contract error codes
CONTRACT_ERRORS = {
    0: "AlreadyInitialized",
    1: "NotInitialized",
    2: "Unauthorized",
    3: "NotFound",
    4: "DuplicateAttestation",
    5: "AlreadyRevoked",
    6: "Expired",
    7: "LimitExceeded",
    8: "InvalidThreshold",
    9: "NotRequiredSigner",
    10: "AlreadySigned",
    11: "ProposalFinalized",
    12: "ProposalExpired",
}
