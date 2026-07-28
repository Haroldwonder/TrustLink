"""Async TrustLink contract client for Python."""

import asyncio
from typing import Optional, List, Any

from stellar_sdk import Keypair, Networks, SorobanServerAsync, xdr
from stellar_sdk import Account, TransactionBuilder, BASE_FEE

from .types import (
    Attestation,
    AttestationStatus,
    AttestationTemplate,
    ClaimTypeInfo,
    ContractConfig,
    ContractMetadata,
    Delegation,
    GlobalStats,
    TrustLinkError,
)
from . import _base
from ._retry import with_retry_async


class AsyncTrustLinkClient:
    """Async client for interacting with TrustLink contract.

    Wraps TrustLinkClient so async services (e.g. FastAPI) can submit
    transactions without blocking the event loop.
    Supports use as an async context manager for automatic resource cleanup::

        async with AsyncTrustLinkClient(contract_id, rpc_url) as client:
            has_kyc = await client.has_valid_claim("GXXX", "KYC_PASSED")

    All read-only methods automatically retry on transient RPC failures using
    exponential backoff (default: 3 attempts, starting at 200 ms).

    To opt out of retries on a specific call, pass ``retry_attempts=1``::

        await client.has_valid_claim(subject, "KYC_PASSED", retry_attempts=1)

    To change the defaults for all calls, pass keyword arguments to the
    constructor::

        client = AsyncTrustLinkClient(
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
    ) -> None:
        self.contract_id = contract_id
        self.rpc_url = rpc_url
        self.network_passphrase = network_passphrase
        self._server = SorobanServerAsync(rpc_url)
        self._retry_attempts = retry_attempts
        self._retry_base_ms = retry_base_ms
        self._retry_max_ms = retry_max_ms

    async def close(self) -> None:
        """Close the underlying HTTP session."""
        await self._server.close()

    async def __aenter__(self) -> "AsyncTrustLinkClient":
        return self

    async def __aexit__(self, *_: Any) -> None:
        await self.close()

    # ─── Read Operations ───────────────────────────────────────────────────────

    async def get_subject_attestations(
        self, subject: str, offset: int = 0, limit: int = 50,
        *, retry_attempts: Optional[int] = None,
    ) -> List[Attestation]:
        """Get attestations for a subject.

        Args:
            subject: Subject address
            offset: Pagination offset
            limit: Pagination limit
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "get_subject_attestations",
            _base.sc_addr(subject),
            _base.sc_u32(offset),
            _base.sc_u32(limit),
        )

    async def has_valid_claim(
        self, subject: str, claim_type: str,
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if subject has valid claim.

        Args:
            subject: Subject address
            claim_type: Claim type identifier
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "has_valid_claim",
            _base.sc_addr(subject),
            _base.sc_str(claim_type),
        )

    async def has_valid_claim_from_issuer(
        self, subject: str, claim_type: str, issuer: str,
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if subject has valid claim from specific issuer.

        Args:
            subject: Subject address
            claim_type: Claim type identifier
            issuer: Issuer address
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "has_valid_claim_from_issuer",
            _base.sc_addr(subject),
            _base.sc_str(claim_type),
            _base.sc_addr(issuer),
        )

    async def has_any_claim(
        self, subject: str, claim_types: List[str],
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if subject has any of the claim types.

        Args:
            subject: Subject address
            claim_types: List of claim type identifiers
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "has_any_claim",
            _base.sc_addr(subject),
            _base.sc_vec_str(claim_types),
        )

    async def has_all_claims(
        self, subject: str, claim_types: List[str],
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if subject has all claim types.

        Args:
            subject: Subject address
            claim_types: List of claim type identifiers
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "has_all_claims",
            _base.sc_addr(subject),
            _base.sc_vec_str(claim_types),
        )

    async def get_attestation(
        self, attestation_id: str,
        *, retry_attempts: Optional[int] = None,
    ) -> Attestation:
        """Get specific attestation.

        Args:
            attestation_id: Attestation ID
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "get_attestation", _base.sc_str(attestation_id)
        )

    async def get_attestation_status(
        self, attestation_id: str,
        *, retry_attempts: Optional[int] = None,
    ) -> AttestationStatus:
        """Get attestation status.

        Args:
            attestation_id: Attestation ID
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "get_attestation_status", _base.sc_str(attestation_id)
        )

    async def get_issuer_attestations(
        self, issuer: str, offset: int = 0, limit: int = 50,
        *, retry_attempts: Optional[int] = None,
    ) -> List[Attestation]:
        """Get attestations issued by issuer.

        Args:
            issuer: Issuer address
            offset: Pagination offset
            limit: Pagination limit
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "get_issuer_attestations",
            _base.sc_addr(issuer),
            _base.sc_u32(offset),
            _base.sc_u32(limit),
        )

    async def list_claim_types(
        self, offset: int = 0, limit: int = 50,
        *, retry_attempts: Optional[int] = None,
    ) -> List[ClaimTypeInfo]:
        """List registered claim types.

        Args:
            offset: Pagination offset
            limit: Pagination limit
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "list_claim_types",
            _base.sc_u32(offset),
            _base.sc_u32(limit),
        )

    async def get_global_stats(
        self, *, retry_attempts: Optional[int] = None,
    ) -> GlobalStats:
        """Get contract-wide statistics.

        Args:
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(retry_attempts, "get_global_stats")

    async def is_issuer(
        self, address: str,
        *, retry_attempts: Optional[int] = None,
    ) -> bool:
        """Check if address is registered issuer.

        Args:
            address: Address to check
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "is_issuer", _base.sc_addr(address)
        )

    async def get_template(
        self, issuer: str, template_id: str,
        *, retry_attempts: Optional[int] = None,
    ) -> AttestationTemplate:
        """Get a named attestation template.

        Args:
            issuer: Issuer address
            template_id: Template identifier
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "get_template", _base.sc_addr(issuer), _base.sc_str(template_id)
        )

    async def list_templates(
        self, issuer: str, start: int = 0, limit: int = 50,
        *, retry_attempts: Optional[int] = None,
    ) -> List[str]:
        """List template IDs registered for an issuer.

        Args:
            issuer: Issuer address
            start: Pagination offset
            limit: Pagination limit
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "list_templates",
            _base.sc_addr(issuer),
            _base.sc_u32(start),
            _base.sc_u32(limit),
        )

    async def get_delegation(
        self, delegator: str, delegate: str, claim_type: str,
        *, retry_attempts: Optional[int] = None,
    ) -> Optional[Delegation]:
        """Get a delegation record.

        Args:
            delegator: Delegating issuer address
            delegate: Delegate address
            claim_type: Delegated claim type
            retry_attempts: Override default retry count for this call.
        """
        return await self._simulate_with_retry(
            retry_attempts,
            "get_delegation",
            _base.sc_addr(delegator),
            _base.sc_addr(delegate),
            _base.sc_str(claim_type),
        )

    # ─── Write Operations ──────────────────────────────────────────────────────
    # Write operations are NOT retried automatically — retrying a submitted
    # transaction can cause double-submission.  Callers who need idempotency
    # on writes must implement their own logic.

    async def create_attestation(
        self,
        issuer_secret: str,
        subject: str,
        claim_type: str,
        expiration: Optional[int] = None,
        metadata: Optional[str] = None,
    ) -> None:
        issuer_addr = Keypair.from_secret(issuer_secret).public_key
        await self._invoke(
            issuer_secret,
            "create_attestation",
            _base.sc_addr(issuer_addr),
            _base.sc_addr(subject),
            _base.sc_str(claim_type),
            _base.sc_opt_u64(expiration),
            _base.sc_opt_str(metadata),
            _base.sc_null(),
        )

    async def revoke_attestation(
        self,
        issuer_secret: str,
        attestation_id: str,
        reason: Optional[str] = None,
    ) -> None:
        issuer_addr = Keypair.from_secret(issuer_secret).public_key
        await self._invoke(
            issuer_secret,
            "revoke_attestation",
            _base.sc_addr(issuer_addr),
            _base.sc_str(attestation_id),
            _base.sc_opt_str(reason),
        )

    async def register_issuer(self, admin_secret: str, issuer: str) -> None:
        admin_addr = Keypair.from_secret(admin_secret).public_key
        await self._invoke(
            admin_secret,
            "register_issuer",
            _base.sc_addr(admin_addr),
            _base.sc_addr(issuer),
        )

    async def remove_issuer(self, admin_secret: str, issuer: str) -> None:
        admin_addr = Keypair.from_secret(admin_secret).public_key
        await self._invoke(
            admin_secret,
            "remove_issuer",
            _base.sc_addr(admin_addr),
            _base.sc_addr(issuer),
        )

    async def propose_attestation(
        self,
        issuer_secret: str,
        subject: str,
        claim_type: str,
        required_signers: List[str],
        threshold: int,
    ) -> str:
        issuer_addr = Keypair.from_secret(issuer_secret).public_key
        return await self._invoke(
            issuer_secret,
            "propose_attestation",
            _base.sc_addr(issuer_addr),
            _base.sc_addr(subject),
            _base.sc_str(claim_type),
            _base.sc_vec_addr(required_signers),
            _base.sc_u32(threshold),
        )

    async def cosign_attestation(self, issuer_secret: str, proposal_id: str) -> None:
        issuer_addr = Keypair.from_secret(issuer_secret).public_key
        await self._invoke(
            issuer_secret,
            "cosign_attestation",
            _base.sc_addr(issuer_addr),
            _base.sc_str(proposal_id),
        )

    # ─── Internal Helpers ──────────────────────────────────────────────────────

    async def _simulate_with_retry(
        self,
        retry_attempts: Optional[int],
        method: str,
        *args: Any,
    ) -> Any:
        """Simulate *method* with automatic retry on transient failures."""
        attempts = retry_attempts if retry_attempts is not None else self._retry_attempts
        return await with_retry_async(
            self._simulate,
            method,
            *args,
            max_attempts=attempts,
            base_ms=self._retry_base_ms,
            max_ms=self._retry_max_ms,
        )

    async def _simulate(self, method: str, *args: Any) -> Any:
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
                        xdr.SCVal(
                            type=xdr.SCValType.SC_VAL_TYPE_ADDRESS,
                            address=xdr.SCAddress(
                                type=xdr.SCAddressType.SC_ADDRESS_TYPE_CONTRACT,
                                contract_id=xdr.Hash(self.contract_id.encode()),
                            ),
                        ),
                        xdr.SCVal(
                            type=xdr.SCValType.SC_VAL_TYPE_SYMBOL,
                            sym=method.encode(),
                        ),
                        *args,
                    ],
                ),
                auth=[],
            )
            .set_timeout(30)
            .build()
        )

        result = await self._server.simulate_transaction(tx)
        if hasattr(result, "error"):
            raise TrustLinkError(f"Simulation error: {result.error}")
        if not hasattr(result, "result") or not result.result:
            raise TrustLinkError(f"No result from {method}")

        return result.result.retval

    async def _invoke(self, secret: str, method: str, *args: Any) -> Any:
        """Invoke contract method (state-changing)."""
        keypair = Keypair.from_secret(secret)
        account = await self._server.load_account(keypair.public_key)

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
                        xdr.SCVal(
                            type=xdr.SCValType.SC_VAL_TYPE_ADDRESS,
                            address=xdr.SCAddress(
                                type=xdr.SCAddressType.SC_ADDRESS_TYPE_CONTRACT,
                                contract_id=xdr.Hash(self.contract_id.encode()),
                            ),
                        ),
                        xdr.SCVal(
                            type=xdr.SCValType.SC_VAL_TYPE_SYMBOL,
                            sym=method.encode(),
                        ),
                        *args,
                    ],
                ),
                auth=[],
            )
            .set_timeout(30)
            .build()
        )

        sim_result = await self._server.simulate_transaction(tx)
        if hasattr(sim_result, "error"):
            raise TrustLinkError(f"Simulation error: {sim_result.error}")

        tx = await self._server.prepare_transaction(tx)
        tx.sign(keypair)

        response = await self._server.submit_transaction(tx)
        if response.get("status") == "ERROR":
            raise TrustLinkError(f"Transaction failed: {response}")

        return response
