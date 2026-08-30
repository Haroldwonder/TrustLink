"""TrustLink contract client for Python."""

from typing import Optional, List, Any
from stellar_sdk import (
    Account,
    Contract,
    Keypair,
    Networks,
    Server,
    TransactionBuilder,
    BASE_FEE,
    xdr,
)

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
)
from . import _base
from ._retry import with_retry


class TrustLinkClient:
    """Client for interacting with TrustLink contract.

    All read-only methods automatically retry on transient RPC failures using
    exponential backoff (default: 3 attempts, starting at 200 ms).

    To opt out of retries on a specific call, pass ``retry_attempts=1``::

        client.has_valid_claim(subject, "KYC_PASSED", retry_attempts=1)

    To change the defaults for all calls, pass keyword arguments to the
    constructor::

        client = TrustLinkClient(
            contract_id, rpc_url,
            retry_attempts=5,
            retry_base_ms=500,
        )
    """

    def __init__(
        self,
        contract_id: str,
        rpc_url: str,
        network_passphrase: str = Networks.TESTNET_NETWORK_PASSPHRASE,
        *,
        retry_attempts: int = 3,
        retry_base_ms: float = 200.0,
        retry_max_ms: float = 10_000.0,
    ):
        """Initialize TrustLink client.

        Args:
            contract_id: Deployed contract address (C...)
            rpc_url: Stellar RPC server URL
            network_passphrase: Network passphrase (defaults to testnet)
            retry_attempts: Max retry attempts for transient read failures
                (default: 3). Pass 1 to disable automatic retries.
            retry_base_ms: Initial backoff delay in milliseconds (default: 200).
            retry_max_ms: Maximum backoff cap in milliseconds (default: 10 000).
        """
        self.contract_id = contract_id
        self.rpc_url = rpc_url
        self.network_passphrase = network_passphrase
        self.server = Server(rpc_url)
        self.contract = Contract(contract_id)
        self._retry_attempts = retry_attempts
        self._retry_base_ms = retry_base_ms
        self._retry_max_ms = retry_max_ms

    # ─── Read Operations ───────────────────────────────────────────────────────

    def get_subject_attestations(
        self, subject: str, offset: int = 0, limit: int = 50,
        *, retry_attempts: Optional[int] = None,
    ) -> List[Attestation]:
        """Get attestations for a subject.

        Args:
            subject: Subject address
            offset: Pagination offset
            limit: Pagination limit
            retry_attempts: Override default retry count for this call.

        Returns:
            List of attestations
        """
        return self._simulate_with_retry(
            retry_attempts,
            "get_subject_attestations",
            self._addr(subject),
            self._u32(offset),
            self._u32(limit),
        )

    def has_valid_claim(
        self, subject: str, claim_type: str,
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if subject has valid claim.

        Args:
            subject: Subject address
            claim_type: Claim type identifier
            retry_attempts: Override default retry count for this call.

        Returns:
            True if subject has valid claim
        """
        return self._simulate_with_retry(
            retry_attempts,
            "has_valid_claim", self._addr(subject), self._str(claim_type)
        )

    def has_valid_claim_from_issuer(
        self, subject: str, claim_type: str, issuer: str,
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if subject has valid claim from specific issuer.

        Args:
            subject: Subject address
            claim_type: Claim type identifier
            issuer: Issuer address
            retry_attempts: Override default retry count for this call.

        Returns:
            True if subject has valid claim from issuer
        """
        return self._simulate_with_retry(
            retry_attempts,
            "has_valid_claim_from_issuer",
            self._addr(subject),
            self._str(claim_type),
            self._addr(issuer),
        )

    def has_any_claim(
        self, subject: str, claim_types: List[str],
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if subject has any of the claim types.

        Args:
            subject: Subject address
            claim_types: List of claim type identifiers (empty list always returns False)
            retry_attempts: Override default retry count for this call.

        Returns:
            True if subject has at least one of the claim types
        """
        if not isinstance(claim_types, list):
            raise TrustLinkError("claim_types must be a list")
        for ct in claim_types:
            if not isinstance(ct, str) or not ct:
                raise TrustLinkError("Each claim type must be a non-empty string")
        if not claim_types:
            return False
        return self._simulate_with_retry(
            retry_attempts,
            "has_any_claim",
            self._addr(subject),
            self._vec_str(claim_types),
        )

    def has_all_claims(
        self, subject: str, claim_types: List[str],
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if subject has all claim types.

        Args:
            subject: Subject address
            claim_types: List of claim type identifiers (empty list always returns True)
            retry_attempts: Override default retry count for this call.

        Returns:
            True if subject has every claim type in the list
        """
        if not isinstance(claim_types, list):
            raise TrustLinkError("claim_types must be a list")
        for ct in claim_types:
            if not isinstance(ct, str) or not ct:
                raise TrustLinkError("Each claim type must be a non-empty string")
        if not claim_types:
            return True
        return self._simulate_with_retry(
            retry_attempts,
            "has_all_claims",
            self._addr(subject),
            self._vec_str(claim_types),
        )

    def get_attestation(
        self, attestation_id: str,
        *, retry_attempts: Optional[int] = None,
    ) -> Attestation:
        """Get specific attestation.

        Args:
            attestation_id: Attestation ID
            retry_attempts: Override default retry count for this call.

        Returns:
            Attestation record
        """
        return self._simulate_with_retry(
            retry_attempts,
            "get_attestation", self._str(attestation_id)
        )

    def get_attestation_status(
        self, attestation_id: str,
        *, retry_attempts: Optional[int] = None,
    ) -> AttestationStatus:
        """Get attestation status.

        Args:
            attestation_id: Attestation ID
            retry_attempts: Override default retry count for this call.

        Returns:
            Attestation status (Valid, Expired, or Revoked)
        """
        return self._simulate_with_retry(
            retry_attempts,
            "get_attestation_status", self._str(attestation_id)
        )

    def get_issuer_attestations(
        self, issuer: str, offset: int = 0, limit: int = 50,
        *, retry_attempts: Optional[int] = None,
    ) -> List[Attestation]:
        """Get attestations issued by issuer.

        Args:
            issuer: Issuer address
            offset: Pagination offset
            limit: Pagination limit
            retry_attempts: Override default retry count for this call.

        Returns:
            List of attestations
        """
        return self._simulate_with_retry(
            retry_attempts,
            "get_issuer_attestations",
            self._addr(issuer),
            self._u32(offset),
            self._u32(limit),
        )

    def list_claim_types(
        self, offset: int = 0, limit: int = 50,
        *, retry_attempts: Optional[int] = None,
    ) -> List[ClaimTypeInfo]:
        """List registered claim types.

        Args:
            offset: Pagination offset
            limit: Pagination limit
            retry_attempts: Override default retry count for this call.

        Returns:
            List of claim type info
        """
        return self._simulate_with_retry(
            retry_attempts,
            "list_claim_types", self._u32(offset), self._u32(limit)
        )

    def get_global_stats(
        self, *, retry_attempts: Optional[int] = None,
    ) -> GlobalStats:
        """Get contract-wide statistics.

        Args:
            retry_attempts: Override default retry count for this call.

        Returns:
            Global statistics
        """
        return self._simulate_with_retry(retry_attempts, "get_global_stats")

    def is_issuer(
        self, address: str,
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if address is registered issuer.

        Args:
            address: Address to check
            retry_attempts: Override default retry count for this call.

        Returns:
            True if address is registered issuer
        """
        return self._simulate_with_retry(
            retry_attempts,
            "is_issuer", self._addr(address)
        )

    def get_template(
        self, issuer: str, template_id: str,
        *, retry_attempts: Optional[int] = None,
    ) -> AttestationTemplate:
        """Get a named attestation template.

        Args:
            issuer: Issuer address
            template_id: Template identifier
            retry_attempts: Override default retry count for this call.

        Returns:
            AttestationTemplate record
        """
        return self._simulate_with_retry(
            retry_attempts,
            "get_template", self._addr(issuer), self._str(template_id)
        )

    def list_templates(
        self, issuer: str, start: int = 0, limit: int = 50,
        *, retry_attempts: Optional[int] = None,
    ) -> List[str]:
        """List template IDs registered for an issuer.

        Args:
            issuer: Issuer address
            start: Pagination offset
            limit: Pagination limit
            retry_attempts: Override default retry count for this call.

        Returns:
            List of template IDs
        """
        return self._simulate_with_retry(
            retry_attempts,
            "list_templates",
            self._addr(issuer),
            self._u32(start),
            self._u32(limit),
        )

    def get_delegation(
        self, delegator: str, delegate: str, claim_type: str,
        *, retry_attempts: Optional[int] = None,
    ) -> Optional[Delegation]:
        """Get a delegation record.

        Args:
            delegator: Delegating issuer address
            delegate: Delegate address
            claim_type: Delegated claim type
            retry_attempts: Override default retry count for this call.

        Returns:
            Delegation record, or None if not found
        """
        return self._simulate_with_retry(
            retry_attempts,
            "get_delegation",
            self._addr(delegator),
            self._addr(delegate),
            self._str(claim_type),
        )

    def get_valid_claims(
        self, subject: str,
        *, retry_attempts: Optional[int] = None,
    ) -> List[str]:
        """Get all valid claim IDs for a subject.

        Args:
            subject: Subject address
            retry_attempts: Override default retry count for this call.

        Returns:
            List of valid claim IDs
        """
        return self._simulate_with_retry(
            retry_attempts,
            "get_valid_claims", self._addr(subject)
        )

    def get_valid_claim_count(
        self, subject: str,
        *, retry_attempts: Optional[int] = None,
    ) -> int:
        """Get count of valid claims for a subject.

        Args:
            subject: Subject address
            retry_attempts: Override default retry count for this call.

        Returns:
            Number of valid (non-revoked, non-expired) claims
        """
        return self._simulate_with_retry(
            retry_attempts,
            "get_valid_claim_count", self._addr(subject)
        )

    def get_config(
        self, *, retry_attempts: Optional[int] = None,
    ) -> ContractConfig:
        """Get contract configuration.

        Args:
            retry_attempts: Override default retry count for this call.

        Returns:
            Contract configuration including admin address and flags
        """
        return self._simulate_with_retry(retry_attempts, "get_config")

    def get_contract_metadata(
        self, *, retry_attempts: Optional[int] = None,
    ) -> ContractMetadata:
        """Get contract metadata.

        Args:
            retry_attempts: Override default retry count for this call.

        Returns:
            Contract metadata including name, description, and version
        """
        return self._simulate_with_retry(retry_attempts, "get_contract_metadata")

    def get_version(
        self, *, retry_attempts: Optional[int] = None,
    ) -> str:
        """Get contract version string.

        Args:
            retry_attempts: Override default retry count for this call.

        Returns:
            Semantic version string (e.g. "0.1.0")
        """
        return self._simulate_with_retry(retry_attempts, "get_version")

    def get_multisig_proposal(
        self, proposal_id: str,
        *, retry_attempts: Optional[int] = None,
    ) -> MultiSigProposal:
        """Get multi-sig attestation proposal.

        Args:
            proposal_id: Proposal ID
            retry_attempts: Override default retry count for this call.

        Returns:
            MultiSigProposal with cosigner count and finalization status
        """
        return self._simulate_with_retry(
            retry_attempts,
            "get_multisig_proposal", self._str(proposal_id)
        )

    def is_whitelisted(
        self, issuer: str, subject: str,
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if subject is whitelisted by issuer.

        Args:
            issuer: Issuer address
            subject: Subject address
            retry_attempts: Override default retry count for this call.

        Returns:
            True if subject is whitelisted
        """
        return self._simulate_with_retry(
            retry_attempts,
            "is_whitelisted", self._addr(issuer), self._addr(subject)
        )

    # ─── Write Operations ──────────────────────────────────────────────────────
    # Write operations are NOT retried automatically — retrying a submitted
    # transaction can cause double-submission.  Callers who need idempotency
    # on writes must implement their own logic.

    def create_attestation(
        self,
        issuer_secret: str,
        subject: str,
        claim_type: str,
        expiration: Optional[int] = None,
        metadata: Optional[str] = None,
    ) -> None:
        """Create attestation.

        Args:
            issuer_secret: Issuer secret key
            subject: Subject address
            claim_type: Claim type identifier
            expiration: Optional expiration timestamp
            metadata: Optional metadata
        """
        self._invoke(
            issuer_secret,
            "create_attestation",
            self._addr(Keypair.from_secret(issuer_secret).public_key),
            self._addr(subject),
            self._str(claim_type),
            self._opt_u64(expiration),
            self._opt_str(metadata),
            self._null(),  # tags
        )

    def revoke_attestation(
        self,
        issuer_secret: str,
        attestation_id: str,
        reason: Optional[str] = None,
    ) -> None:
        """Revoke attestation.

        Args:
            issuer_secret: Issuer secret key
            attestation_id: Attestation ID
            reason: Optional revocation reason
        """
        self._invoke(
            issuer_secret,
            "revoke_attestation",
            self._addr(Keypair.from_secret(issuer_secret).public_key),
            self._str(attestation_id),
            self._opt_str(reason),
        )

    def register_issuer(self, admin_secret: str, issuer: str) -> None:
        """Register issuer (admin only).

        Args:
            admin_secret: Admin secret key
            issuer: Issuer address to register
        """
        admin_addr = Keypair.from_secret(admin_secret).public_key
        self._invoke(
            admin_secret,
            "register_issuer",
            self._addr(admin_addr),
            self._addr(issuer),
        )

    def remove_issuer(self, admin_secret: str, issuer: str) -> None:
        """Remove issuer (admin only).

        Args:
            admin_secret: Admin secret key
            issuer: Issuer address to remove
        """
        admin_addr = Keypair.from_secret(admin_secret).public_key
        self._invoke(
            admin_secret,
            "remove_issuer",
            self._addr(admin_addr),
            self._addr(issuer),
        )

    def propose_attestation(
        self,
        issuer_secret: str,
        subject: str,
        claim_type: str,
        required_signers: List[str],
        threshold: int,
    ) -> str:
        """Propose multi-sig attestation.

        Args:
            issuer_secret: Proposer secret key
            subject: Subject address
            claim_type: Claim type identifier
            required_signers: List of required signer addresses
            threshold: Signature threshold

        Returns:
            Proposal ID
        """
        issuer_addr = Keypair.from_secret(issuer_secret).public_key
        return self._invoke(
            issuer_secret,
            "propose_attestation",
            self._addr(issuer_addr),
            self._addr(subject),
            self._str(claim_type),
            self._vec_addr(required_signers),
            self._u32(threshold),
        )

    def cosign_attestation(self, issuer_secret: str, proposal_id: str) -> None:
        """Co-sign multi-sig proposal.

        Args:
            issuer_secret: Co-signer secret key
            proposal_id: Proposal ID
        """
        issuer_addr = Keypair.from_secret(issuer_secret).public_key
        self._invoke(
            issuer_secret,
            "cosign_attestation",
            self._addr(issuer_addr),
            self._str(proposal_id),
        )

    def add_to_whitelist(self, issuer_secret: str, subject: str) -> None:
        """Add subject to issuer's whitelist.

        Args:
            issuer_secret: Issuer secret key
            subject: Subject address to whitelist
        """
        issuer_addr = Keypair.from_secret(issuer_secret).public_key
        self._invoke(
            issuer_secret,
            "add_to_whitelist",
            self._addr(issuer_addr),
            self._addr(subject),
        )

    def enable_whitelist_mode(self, issuer_secret: str, enabled: bool) -> None:
        """Toggle whitelist mode for issuer.

        Args:
            issuer_secret: Issuer secret key
            enabled: True to enable whitelist mode, False to disable
        """
        issuer_addr = Keypair.from_secret(issuer_secret).public_key
        self._invoke(
            issuer_secret,
            "enable_whitelist_mode",
            self._addr(issuer_addr),
            xdr.SCVal(
                type=xdr.SCValType.SC_VAL_TYPE_BOOL,
                b=enabled,
            ),
        )

    # ─── Internal Helpers ──────────────────────────────────────────────────────

    def _simulate_with_retry(
        self,
        retry_attempts: Optional[int],
        method: str,
        *args: Any,
    ) -> Any:
        """Simulate *method* with automatic retry on transient failures.

        Uses ``retry_attempts`` if given, otherwise falls back to the
        instance-level default set in the constructor.
        """
        attempts = retry_attempts if retry_attempts is not None else self._retry_attempts
        return with_retry(
            self._simulate,
            method,
            *args,
            max_attempts=attempts,
            base_ms=self._retry_base_ms,
            max_ms=self._retry_max_ms,
        )

    def _simulate(self, method: str, *args: Any) -> Any:
        """Simulate contract call (read-only)."""
        dummy_keypair = Keypair.random()
        account = Account(dummy_keypair.public_key, 0)
        tx = (
            TransactionBuilder(
                account,
                base_fee=BASE_FEE,
                network_passphrase=self.network_passphrase,
            )
            .add_text_memo("sim")
            .append_invoke_host_function_op(
                host_function=xdr.HostFunction(
                    type=xdr.HostFunctionType.HOST_FUNCTION_TYPE_INVOKE_CONTRACT,
                    args=[
                        xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_ADDRESS, address=xdr.SCAddress(
                            type=xdr.SCAddressType.SC_ADDRESS_TYPE_CONTRACT,
                            contract_id=xdr.Hash(self.contract_id.encode()),
                        )),
                        xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_SYMBOL, sym=method.encode()),
                        *args,
                    ],
                ),
                auth=[],
            )
            .set_timeout(30)
            .build()
        )

        result = self.server.simulate_transaction(tx)
        if hasattr(result, "error"):
            raise TrustLinkError(f"Simulation error: {result.error}")

        if not hasattr(result, "result") or not result.result:
            raise TrustLinkError(f"No result from {method}")

        return result.result.retval

    def _invoke(self, secret: str, method: str, *args: Any) -> Any:
        """Invoke contract method (state-changing)."""
        keypair = Keypair.from_secret(secret)
        account = self.server.load_account(keypair.public_key)

        tx = (
            TransactionBuilder(
                account,
                base_fee=BASE_FEE,
                network_passphrase=self.network_passphrase,
            )
            .add_text_memo("invoke")
            .append_invoke_host_function_op(
                host_function=xdr.HostFunction(
                    type=xdr.HostFunctionType.HOST_FUNCTION_TYPE_INVOKE_CONTRACT,
                    args=[
                        xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_ADDRESS, address=xdr.SCAddress(
                            type=xdr.SCAddressType.SC_ADDRESS_TYPE_CONTRACT,
                            contract_id=xdr.Hash(self.contract_id.encode()),
                        )),
                        xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_SYMBOL, sym=method.encode()),
                        *args,
                    ],
                ),
                auth=[],
            )
            .set_timeout(30)
            .build()
        )

        sim_result = self.server.simulate_transaction(tx)
        if hasattr(sim_result, "error"):
            raise TrustLinkError(f"Simulation error: {sim_result.error}")

        tx = self.server.prepare_transaction(tx)
        tx.sign(keypair)

        response = self.server.submit_transaction(tx)
        if response.get("status") == "ERROR":
            raise TrustLinkError(f"Transaction failed: {response}")

        return response

    # ─── SCVal Helpers (delegate to _base) ────────────────────────────────────

    _str = staticmethod(_base.sc_str)
    _addr = staticmethod(_base.sc_addr)
    _u32 = staticmethod(_base.sc_u32)
    _u64 = staticmethod(_base.sc_u64)
    _opt_str = staticmethod(_base.sc_opt_str)
    _opt_u64 = staticmethod(_base.sc_opt_u64)
    _vec_str = staticmethod(_base.sc_vec_str)
    _vec_addr = staticmethod(_base.sc_vec_addr)
    _null = staticmethod(_base.sc_null)
