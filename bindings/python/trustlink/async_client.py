"""Async TrustLink contract client for Python."""

from typing import Optional, List, Any

from stellar_sdk import Keypair, Networks, SorobanServerAsync, xdr
from stellar_sdk import Account, TransactionBuilder, BASE_FEE

from .types import (
    Attestation,
    AttestationStatus,
    AttestationTemplate,
    ClaimTypeInfo,
    Delegation,
    GlobalStats,
    TrustLinkError,
)
from . import _base


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
        return await self._simulate(
            "get_subject_attestations",
            _base.sc_addr(subject),
            _base.sc_u32(offset),
            _base.sc_u32(limit),
        )

    async def has_valid_claim(self, subject: str, claim_type: str) -> bool:
        return await self._simulate(
            "has_valid_claim",
            _base.sc_addr(subject),
            _base.sc_str(claim_type),
        )

    async def has_valid_claim_from_issuer(
        self, subject: str, claim_type: str, issuer: str
    ) -> bool:
        return await self._simulate(
            "has_valid_claim_from_issuer",
            _base.sc_addr(subject),
            _base.sc_str(claim_type),
            _base.sc_addr(issuer),
        )

    async def has_any_claim(self, subject: str, claim_types: List[str]) -> bool:
        return await self._simulate(
            "has_any_claim",
            _base.sc_addr(subject),
            _base.sc_vec_str(claim_types),
        )

    async def has_all_claims(self, subject: str, claim_types: List[str]) -> bool:
        return await self._simulate(
            "has_all_claims",
            _base.sc_addr(subject),
            _base.sc_vec_str(claim_types),
        )

    async def get_attestation(self, attestation_id: str) -> Attestation:
        return await self._simulate(
            "get_attestation", _base.sc_str(attestation_id)
        )

    async def get_attestation_status(self, attestation_id: str) -> AttestationStatus:
        return await self._simulate(
            "get_attestation_status", _base.sc_str(attestation_id)
        )

    async def get_issuer_attestations(
        self, issuer: str, offset: int = 0, limit: int = 50
    ) -> List[Attestation]:
        return await self._simulate(
            "get_issuer_attestations",
            _base.sc_addr(issuer),
            _base.sc_u32(offset),
            _base.sc_u32(limit),
        )

    async def list_claim_types(
        self, offset: int = 0, limit: int = 50
    ) -> List[ClaimTypeInfo]:
        return await self._simulate(
            "list_claim_types",
            _base.sc_u32(offset),
            _base.sc_u32(limit),
        )

    async def get_global_stats(self) -> GlobalStats:
        return await self._simulate("get_global_stats")

    async def is_issuer(self, address: str) -> bool:
        return await self._simulate("is_issuer", _base.sc_addr(address))

    async def get_template(self, issuer: str, template_id: str) -> AttestationTemplate:
        """Get a named attestation template."""
        return await self._simulate(
            "get_template", _base.sc_addr(issuer), _base.sc_str(template_id)
        )

    async def list_templates(
        self, issuer: str, start: int = 0, limit: int = 50
    ) -> List[str]:
        """List template IDs registered for an issuer."""
        return await self._simulate(
            "list_templates",
            _base.sc_addr(issuer),
            _base.sc_u32(start),
            _base.sc_u32(limit),
        )

    async def get_delegation(
        self, delegator: str, delegate: str, claim_type: str
    ) -> Optional[Delegation]:
        """Get a delegation record."""
        return await self._simulate(
            "get_delegation",
            _base.sc_addr(delegator),
            _base.sc_addr(delegate),
            _base.sc_str(claim_type),
        )

    # ─── Write Operations ──────────────────────────────────────────────────────

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
