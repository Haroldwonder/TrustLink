"""Async TrustLink contract client for Python."""

import asyncio
from functools import partial
from typing import Optional, List

from .client import TrustLinkClient
from .types import (
    Attestation,
    AttestationStatus,
    ClaimTypeInfo,
    ContractConfig,
    ContractMetadata,
    GlobalStats,
    IssuerStats,
    MultiSigProposal,
)


class AsyncTrustLinkClient:
    """Async client for interacting with TrustLink contract.

    Wraps TrustLinkClient so async services (e.g. FastAPI) can submit
    transactions without blocking the event loop.
    """

    def __init__(
        self,
        contract_id: str,
        rpc_url: str,
        network_passphrase: str = "Test SDF Network ; September 2015",
    ):
        self._sync = TrustLinkClient(contract_id, rpc_url, network_passphrase)

    async def _run(self, func, *args, **kwargs):
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, partial(func, *args, **kwargs))

    # ─── Read Operations ───────────────────────────────────────────────────────

    async def get_subject_attestations(
        self, subject: str, offset: int = 0, limit: int = 50
    ) -> List[Attestation]:
        return await self._run(self._sync.get_subject_attestations, subject, offset, limit)

    async def has_valid_claim(self, subject: str, claim_type: str) -> bool:
        return await self._run(self._sync.has_valid_claim, subject, claim_type)

    async def has_valid_claim_from_issuer(
        self, subject: str, claim_type: str, issuer: str
    ) -> bool:
        return await self._run(self._sync.has_valid_claim_from_issuer, subject, claim_type, issuer)

    async def has_any_claim(self, subject: str, claim_types: List[str]) -> bool:
        return await self._run(self._sync.has_any_claim, subject, claim_types)

    async def has_all_claims(self, subject: str, claim_types: List[str]) -> bool:
        return await self._run(self._sync.has_all_claims, subject, claim_types)

    async def get_attestation(self, attestation_id: str) -> Attestation:
        return await self._run(self._sync.get_attestation, attestation_id)

    async def get_attestation_status(self, attestation_id: str) -> AttestationStatus:
        return await self._run(self._sync.get_attestation_status, attestation_id)

    async def get_issuer_attestations(
        self, issuer: str, offset: int = 0, limit: int = 50
    ) -> List[Attestation]:
        return await self._run(self._sync.get_issuer_attestations, issuer, offset, limit)

    async def list_claim_types(
        self, offset: int = 0, limit: int = 50
    ) -> List[ClaimTypeInfo]:
        return await self._run(self._sync.list_claim_types, offset, limit)

    async def get_global_stats(self) -> GlobalStats:
        return await self._run(self._sync.get_global_stats)

    async def is_issuer(self, address: str) -> bool:
        return await self._run(self._sync.is_issuer, address)

    async def get_config(self) -> ContractConfig:
        return await self._run(self._sync.get_config)

    async def get_contract_metadata(self) -> ContractMetadata:
        return await self._run(self._sync.get_contract_metadata)

    async def get_version(self) -> str:
        return await self._run(self._sync.get_version)

    async def get_multisig_proposal(self, proposal_id: str) -> MultiSigProposal:
        return await self._run(self._sync.get_multisig_proposal, proposal_id)

    async def is_whitelisted(self, issuer: str, subject: str) -> bool:
        return await self._run(self._sync.is_whitelisted, issuer, subject)

    # ─── Write Operations ──────────────────────────────────────────────────────

    async def create_attestation(
        self,
        issuer_secret: str,
        subject: str,
        claim_type: str,
        expiration: Optional[int] = None,
        metadata: Optional[str] = None,
    ) -> None:
        return await self._run(
            self._sync.create_attestation,
            issuer_secret,
            subject,
            claim_type,
            expiration,
            metadata,
        )

    async def revoke_attestation(
        self,
        issuer_secret: str,
        attestation_id: str,
        reason: Optional[str] = None,
    ) -> None:
        return await self._run(
            self._sync.revoke_attestation, issuer_secret, attestation_id, reason
        )

    async def propose_attestation(
        self,
        issuer_secret: str,
        subject: str,
        claim_type: str,
        required_signers: List[str],
        threshold: int,
    ) -> str:
        return await self._run(
            self._sync.propose_attestation,
            issuer_secret,
            subject,
            claim_type,
            required_signers,
            threshold,
        )

    async def cosign_attestation(self, issuer_secret: str, proposal_id: str) -> None:
        return await self._run(self._sync.cosign_attestation, issuer_secret, proposal_id)

    async def add_to_whitelist(self, issuer_secret: str, subject: str) -> None:
        return await self._run(self._sync.add_to_whitelist, issuer_secret, subject)

    async def enable_whitelist_mode(self, issuer_secret: str, enabled: bool) -> None:
        return await self._run(self._sync.enable_whitelist_mode, issuer_secret, enabled)
