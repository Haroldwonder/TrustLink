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
from stellar_sdk.operation import InvokeHostFunction
from stellar_sdk.utils import parse_transaction_envelope_from_xdr

from .types import (
    Attestation,
    AttestationStatus,
    ClaimTypeInfo,
    GlobalStats,
    IssuerStats,
    MultiSigProposal,
    TrustLinkError,
    ContractError,
    CONTRACT_ERRORS,
)


class TrustLinkClient:
    """Client for interacting with TrustLink contract."""

    def __init__(
        self,
        contract_id: str,
        rpc_url: str,
        network_passphrase: str = Networks.TESTNET_NETWORK_PASSPHRASE,
    ):
        """Initialize TrustLink client.

        Args:
            contract_id: Deployed contract address (C...)
            rpc_url: Stellar RPC server URL
            network_passphrase: Network passphrase (defaults to testnet)
        """
        self.contract_id = contract_id
        self.rpc_url = rpc_url
        self.network_passphrase = network_passphrase
        self.server = Server(rpc_url)
        self.contract = Contract(contract_id)

    # ─── Read Operations ───────────────────────────────────────────────────────

    def get_subject_attestations(
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
        return self._simulate(
            "get_subject_attestations",
            self._addr(subject),
            self._u32(offset),
            self._u32(limit),
        )

    def has_valid_claim(self, subject: str, claim_type: str) -> bool:
        """Check if subject has valid claim.

        Args:
            subject: Subject address
            claim_type: Claim type identifier

        Returns:
            True if subject has valid claim
        """
        return self._simulate(
            "has_valid_claim", self._addr(subject), self._str(claim_type)
        )

    def has_valid_claim_from_issuer(
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
        return self._simulate(
            "has_valid_claim_from_issuer",
            self._addr(subject),
            self._str(claim_type),
            self._addr(issuer),
        )

    def has_any_claim(self, subject: str, claim_types: List[str]) -> bool:
        """Check if subject has any of the claim types.

        Args:
            subject: Subject address
            claim_types: List of claim type identifiers

        Returns:
            True if subject has any of the claim types
        """
        return self._simulate(
            "has_any_claim",
            self._addr(subject),
            self._vec_str(claim_types),
        )

    def has_all_claims(self, subject: str, claim_types: List[str]) -> bool:
        """Check if subject has all claim types.

        Args:
            subject: Subject address
            claim_types: List of claim type identifiers

        Returns:
            True if subject has all claim types
        """
        return self._simulate(
            "has_all_claims",
            self._addr(subject),
            self._vec_str(claim_types),
        )

    def get_attestation(self, attestation_id: str) -> Attestation:
        """Get specific attestation.

        Args:
            attestation_id: Attestation ID

        Returns:
            Attestation record
        """
        return self._simulate("get_attestation", self._str(attestation_id))

    def get_attestation_status(self, attestation_id: str) -> AttestationStatus:
        """Get attestation status.

        Args:
            attestation_id: Attestation ID

        Returns:
            Attestation status (Valid, Expired, or Revoked)
        """
        return self._simulate("get_attestation_status", self._str(attestation_id))

    def get_issuer_attestations(
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
        return self._simulate(
            "get_issuer_attestations",
            self._addr(issuer),
            self._u32(offset),
            self._u32(limit),
        )

    def list_claim_types(self, offset: int = 0, limit: int = 50) -> List[ClaimTypeInfo]:
        """List registered claim types.

        Args:
            offset: Pagination offset
            limit: Pagination limit

        Returns:
            List of claim type info
        """
        return self._simulate(
            "list_claim_types", self._u32(offset), self._u32(limit)
        )

    def get_global_stats(self) -> GlobalStats:
        """Get contract-wide statistics.

        Returns:
            Global statistics
        """
        return self._simulate("get_global_stats")

    def is_issuer(self, address: str) -> bool:
        """Check if address is registered issuer.

        Args:
            address: Address to check

        Returns:
            True if address is registered issuer
        """
        return self._simulate("is_issuer", self._addr(address))

    # ─── Write Operations ──────────────────────────────────────────────────────

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

    # ─── Internal Helpers ──────────────────────────────────────────────────────

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

    @staticmethod
    def _str(s: str) -> xdr.SCVal:
        """Convert string to SCVal."""
        return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_SYMBOL, sym=s.encode())

    @staticmethod
    def _addr(a: str) -> xdr.SCVal:
        """Convert address to SCVal."""
        return xdr.SCVal(
            type=xdr.SCValType.SC_VAL_TYPE_ADDRESS,
            address=xdr.SCAddress(
                type=xdr.SCAddressType.SC_ADDRESS_TYPE_ACCOUNT,
                account_id=xdr.AccountID(
                    type=xdr.PublicKeyType.PUBLIC_KEY_TYPE_ED25519,
                    ed25519=xdr.Uint256(Keypair.from_public_key(a).raw_public_key()),
                ),
            ),
        )

    @staticmethod
    def _u32(n: int) -> xdr.SCVal:
        """Convert u32 to SCVal."""
        return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_U32, u32=xdr.Uint32(n))

    @staticmethod
    def _u64(n: int) -> xdr.SCVal:
        """Convert u64 to SCVal."""
        return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_U64, u64=xdr.Uint64(n))

    @staticmethod
    def _opt_str(s: Optional[str]) -> xdr.SCVal:
        """Convert optional string to SCVal."""
        if s is None:
            return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_VEC, vec=[])
        return xdr.SCVal(
            type=xdr.SCValType.SC_VAL_TYPE_VEC,
            vec=[xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_SYMBOL, sym=s.encode())],
        )

    @staticmethod
    def _opt_u64(n: Optional[int]) -> xdr.SCVal:
        """Convert optional u64 to SCVal."""
        if n is None:
            return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_VEC, vec=[])
        return xdr.SCVal(
            type=xdr.SCValType.SC_VAL_TYPE_VEC,
            vec=[xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_U64, u64=xdr.Uint64(n))],
        )

    @staticmethod
    def _vec_str(strs: List[str]) -> xdr.SCVal:
        """Convert list of strings to SCVal."""
        return xdr.SCVal(
            type=xdr.SCValType.SC_VAL_TYPE_VEC,
            vec=[xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_SYMBOL, sym=s.encode()) for s in strs],
        )

    @staticmethod
    def _vec_addr(addrs: List[str]) -> xdr.SCVal:
        """Convert list of addresses to SCVal."""
        return xdr.SCVal(
            type=xdr.SCValType.SC_VAL_TYPE_VEC,
            vec=[TrustLinkClient._addr(a) for a in addrs],
        )

    @staticmethod
    def _null() -> xdr.SCVal:
        """Return null SCVal."""
        return xdr.SCVal(type=xdr.SCValType.SC_VAL_TYPE_VEC, vec=[])
