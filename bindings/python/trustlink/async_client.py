"""Async TrustLink contract client for Python."""

from typing import Optional, List, Any

from stellar_sdk import Keypair, Networks, SorobanServerAsync, xdr
from stellar_sdk import Account, TransactionBuilder, BASE_FEE

from .client import TrustLinkClient
from .types import (
    Attestation,
    AttestationStatus,
    ClaimTypeInfo,
    GlobalStats,
    TrustLinkError,
)


class AsyncTrustLinkClient:
    """Async client for interacting with TrustLink contract.

    Supports use as an async context manager for automatic resource cleanup::

        async with AsyncTrustLinkClient(contract_id, rpc_url) as client:
            has_kyc = await client.has_valid_claim("GXXX", "KYC_PASSED")
    """

    def __init__(
        self,
        contract_id: str,
        rpc_url: str,
        network_passphrase: str = Networks.TESTNET_NETWORK_PASSPHRASE,
    ) -> None:
        """Initialize async TrustLink client.

        Args:
            contract_id: Deployed contract address (C...)
            rpc_url: Stellar RPC server URL
            network_passphrase: Network passphrase (defaults to testnet)
        """
        self.contract_id = contract_id
        self.rpc_url = rpc_url
        self.network_passphrase = network_passphrase
        self._server = SorobanServerAsync(rpc_url)

    async def close(self) -> None:
        """Close the underlying HTTP session."""
        await self._server.close()

    async def __aenter__(self) -> "AsyncTrustLinkClient":
        return self

    async def __aexit__(self, *_: Any) -> None:
        await self.close()

    # ─── Read Operations ───────────────────────────────────────────────────────

    async def get_subject_attestations(
        self, subject: str, offset: int = 0, limit: int = 50
    ) -> List[Attestation]:
        """Get attestations for a subject.

        Args:
            subject: Subject address
            offset: Pagination offset
            limit: Pagination limit

        Returns:
            List of attestations
        """
        return await self._simulate(
            "get_subject_attestations",
            TrustLinkClient._addr(subject),
            TrustLinkClient._u32(offset),
            TrustLinkClient._u32(limit),
        )

    async def has_valid_claim(self, subject: str, claim_type: str) -> bool:
        """Check if subject has a valid claim.

        Args:
            subject: Subject address
            claim_type: Claim type identifier

        Returns:
            True if subject has valid claim
        """
        return await self._simulate(
            "has_valid_claim",
            TrustLinkClient._addr(subject),
            TrustLinkClient._str(claim_type),
        )

    async def has_valid_claim_from_issuer(
        self, subject: str, claim_type: str, issuer: str
    ) -> bool:
        """Check if subject has valid claim from specific issuer.

        Args:
            subject: Subject address
            claim_type: Claim type identifier
            issuer: Issuer address

        Returns:
            True if subject has valid claim from issuer
        """
        return await self._simulate(
            "has_valid_claim_from_issuer",
            TrustLinkClient._addr(subject),
            TrustLinkClient._str(claim_type),
            TrustLinkClient._addr(issuer),
        )

    async def has_any_claim(self, subject: str, claim_types: List[str]) -> bool:
        """Check if subject has any of the claim types.

        Args:
            subject: Subject address
            claim_types: List of claim type identifiers

        Returns:
            True if subject has any of the claim types
        """
        return await self._simulate(
            "has_any_claim",
            TrustLinkClient._addr(subject),
            TrustLinkClient._vec_str(claim_types),
        )

    async def has_all_claims(self, subject: str, claim_types: List[str]) -> bool:
        """Check if subject has all claim types.

        Args:
            subject: Subject address
            claim_types: List of claim type identifiers

        Returns:
            True if subject has all claim types
        """
        return await self._simulate(
            "has_all_claims",
            TrustLinkClient._addr(subject),
            TrustLinkClient._vec_str(claim_types),
        )

    async def get_attestation(self, attestation_id: str) -> Attestation:
        """Get specific attestation.

        Args:
            attestation_id: Attestation ID

        Returns:
            Attestation record
        """
        return await self._simulate(
            "get_attestation", TrustLinkClient._str(attestation_id)
        )

    async def get_attestation_status(self, attestation_id: str) -> AttestationStatus:
        """Get attestation status.

        Args:
            attestation_id: Attestation ID

        Returns:
            Attestation status (Valid, Expired, or Revoked)
        """
        return await self._simulate(
            "get_attestation_status", TrustLinkClient._str(attestation_id)
        )

    async def get_issuer_attestations(
        self, issuer: str, offset: int = 0, limit: int = 50
    ) -> List[Attestation]:
        """Get attestations issued by issuer.

        Args:
            issuer: Issuer address
            offset: Pagination offset
            limit: Pagination limit

        Returns:
            List of attestations
        """
        return await self._simulate(
            "get_issuer_attestations",
            TrustLinkClient._addr(issuer),
            TrustLinkClient._u32(offset),
            TrustLinkClient._u32(limit),
        )

    async def list_claim_types(
        self, offset: int = 0, limit: int = 50
    ) -> List[ClaimTypeInfo]:
        """List registered claim types.

        Args:
            offset: Pagination offset
            limit: Pagination limit

        Returns:
            List of claim type info
        """
        return await self._simulate(
            "list_claim_types",
            TrustLinkClient._u32(offset),
            TrustLinkClient._u32(limit),
        )

    async def get_global_stats(self) -> GlobalStats:
        """Get contract-wide statistics.

        Returns:
            Global statistics
        """
        return await self._simulate("get_global_stats")

    async def is_issuer(self, address: str) -> bool:
        """Check if address is a registered issuer.

        Args:
            address: Address to check

        Returns:
            True if address is registered issuer
        """
        return await self._simulate("is_issuer", TrustLinkClient._addr(address))

    # ─── Internal Helpers ──────────────────────────────────────────────────────

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
