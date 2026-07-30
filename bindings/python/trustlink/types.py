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
    bundle_id: Optional[str]


class AttestationBundle(TypedDict):
    """Bundle of attestations issued atomically."""
    id: str
    issuer: str
    subject: str
    claim_types: List[str]
    timestamp: int
    attestation_ids: List[str]
    all_valid: bool


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


class AttestationTemplate(TypedDict):
    """Named attestation template owned by an issuer."""
    issuer: str
    template_id: str
    claim_type: str
    metadata: Optional[str]
    metadata_template: Optional[str]
    default_expiration_days: Optional[int]


class Delegation(TypedDict):
    """Sub-issuer delegation record."""
    delegator: str
    delegate: str
    claim_type: str
    expiration: Optional[int]


class ContractConfig(TypedDict):
    """Contract configuration."""
    admin: str
    initialized: bool
    whitelist_mode: bool


class ContractMetadata(TypedDict):
    """Contract metadata."""
    name: str
    description: str
    version: str


class TrustLinkError(Exception):
    """Base exception for TrustLink SDK errors."""
    pass


class ContractError(TrustLinkError):
    """Contract execution error."""
    def __init__(self, code: int, message: str):
        self.code = code
        self.message = message
        super().__init__(f"Contract error #{code}: {message}")


# Contract error codes — must stay in sync with sdk/error-codes.json and src/errors.rs
CONTRACT_ERRORS = {
    1: "AlreadyInitialized",
    2: "NotInitialized",
    3: "Unauthorized",
    4: "NotFound",
    5: "DuplicateAttestation",
    6: "AlreadyRevoked",
    7: "Expired",
    8: "InvalidValidFrom",
    9: "InvalidExpiration",
    10: "MetadataTooLong",
    11: "InvalidTimestamp",
    12: "InvalidFee",
    13: "FeeTokenRequired",
    14: "TooManyTags",
    15: "TagTooLong",
    16: "InvalidThreshold",
    17: "NotRequiredSigner",
    18: "AlreadySigned",
    19: "ProposalFinalized",
    20: "ProposalExpired",
    21: "ReasonTooLong",
    22: "CannotEndorseOwn",
    23: "AlreadyEndorsed",
    24: "ContractPaused",
    25: "SubjectNotWhitelisted",
    26: "InvalidClaimType",
    27: "InvalidJurisdiction",
    28: "RateLimited",
    29: "LimitExceeded",
    30: "ProposalCancelled",
    44: "InvalidSourceReference",
}


def decode_contract_error(error_message: str):
    """Decode a contract simulation error string into a ContractError.

    Looks for ``Error(Contract, #<code>)`` patterns (Soroban RPC style) and
    maps the numeric code via ``CONTRACT_ERRORS``. Returns ``None`` if the
    message does not match a known contract error.
    """
    import re

    match = re.search(r"Error\(Contract,\s*#(\d+)\)", error_message)
    if match:
        code = int(match.group(1))
        name = CONTRACT_ERRORS.get(code)
        if name is not None:
            return ContractError(code, name)
    for code, name in CONTRACT_ERRORS.items():
        if name in error_message:
            return ContractError(code, name)
    return None


def classify_error_code(code: int):
    """Return the canonical error name for a numeric code, or None if unknown."""
    return CONTRACT_ERRORS.get(code)
