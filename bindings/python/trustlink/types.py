"""Type definitions for TrustLink contract."""

from typing import TypedDict, Optional, List, Literal

AttestationStatus = Literal["Valid", "Expired", "Revoked", "Pending"]

# Matches RequestStatus on-chain enum (src/types.rs).
# - Pending:   open, not yet fulfilled or rejected.
# - Fulfilled: issuer created an attestation from this request.
# - Rejected:  issuer explicitly rejected the request.
# - Cancelled: subject cancelled the request before it was processed.
# - Expired:   expires_at passed without fulfillment; computed on read or
#              persisted by cleanup_expired_requests.
RequestStatus = Literal["Pending", "Fulfilled", "Rejected", "Cancelled", "Expired"]


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


class AttestationRequest(TypedDict):
    """A pull-based attestation request submitted by a subject to an issuer.

    The ``status`` field reflects the on-chain ``RequestStatus`` enum.
    A request that is still ``Pending`` but whose ``expires_at`` has passed
    will be returned as ``Expired`` by the contract's ``get_attestation_request``
    entrypoint (computed lazily on read, or eagerly by ``cleanup_expired_requests``).
    """
    id: str
    subject: str
    issuer: str
    claim_type: str
    timestamp: int
    expires_at: int
    status: RequestStatus
    rejection_reason: Optional[str]


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


# Contract error codes are generated from src/errors.rs — do not hand-edit.
# Run `node scripts/generate-error-codes.mjs` (or `make generate`) to regenerate.
from trustlink.generated_error_codes import CONTRACT_ERRORS  # noqa: E402, F401


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
