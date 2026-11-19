"""TrustLink Event Topics and Event Subscription Utilities.

Provides canonical event topic definitions, type-safe event subscriptions,
and helpers for filtering contract events via the indexer or direct ledger watching.
"""

from dataclasses import dataclass
from typing import List, Optional, Tuple, Callable, Set, Any
from datetime import datetime
import asyncio
import json

try:
    import websockets
except ImportError:
    websockets = None  # type: ignore


# ─── Event Topic Constants ─────────────────────────────────────────────────────

class EventTopics:
    """Canonical event topic constants matching TrustLink contract.
    
    All topics are ≤9 characters as required by Soroban symbol_short!().
    """
    
    # ─── Attestation Lifecycle ────────────────────────────────────────────────────
    CREATED = "created"
    IMPORTED = "imported"
    BRIDGED = "bridged"
    REVOKED = "revoked"
    RENEWED = "renewed"
    UPDATED = "updated"
    EXPIRED = "expired"
    ENDORSED = "endorsed"
    AMENDED = "amended"
    DEL_REQ = "del_req"
    XFER = "xfer"
    ATT_XFER = "att_xfer"

    # ─── Issuer Lifecycle ──────────────────────────────────────────────────────────
    ISS_REG = "iss_reg"
    ISS_TIER = "iss_tier"
    ISS_REM = "iss_rem"

    # ─── Admin & Governance ────────────────────────────────────────────────────────
    ADM_INIT = "adm_init"
    ADM_XFER = "adm_xfer"
    ADM_ADD = "adm_add"
    ADM_REM = "adm_rem"
    ADM_PROP = "adm_prop"

    # ─── Compliance & Requests ────────────────────────────────────────────────────
    CLM_TYPE = "clm_type"
    ATT_REQ = "att_req"
    REQ_OK = "req_ok"
    REQ_NO = "req_no"
    REQ_CNCL = "req_cncl"
    DEL_CRTD = "del_crtd"
    DEL_RVKD = "del_rvkd"
    WL_ON = "wl_on"
    WL_ADD = "wl_add"
    WL_REM = "wl_rem"

    # ─── Multi-Sig & Proposals ────────────────────────────────────────────────────
    MS_PROP = "ms_prop"
    MS_SIGN = "ms_sign"
    MS_ACTV = "ms_actv"
    MS_CANCEL = "ms_cancel"

    # ─── Dispute & Amendment ──────────────────────────────────────────────────────
    DISPUTED = "disputed"
    DSP_RES = "dsp_res"

    # ─── Pause & Lifecycle ────────────────────────────────────────────────────────
    PAUSED = "paused"
    UNPAUSED = "unpaused"

    # ─── Templates ────────────────────────────────────────────────────────────────
    TMPL_CRT = "tmpl_crt"
    TPL_DEL = "tpl_del"

    # ─── Council Governance ───────────────────────────────────────────────────────
    CNCL_INI = "cncl_ini"
    PROP_NEW = "prop_new"
    PROP_OK = "prop_ok"
    PROP_EXE = "prop_exe"
    TL_START = "tl_start"
    EXP_HOOK = "exp_hook"

    @classmethod
    def all_topics(cls) -> List[str]:
        """Returns list of all valid event topics."""
        return [
            cls.CREATED,
            cls.IMPORTED,
            cls.BRIDGED,
            cls.REVOKED,
            cls.RENEWED,
            cls.UPDATED,
            cls.EXPIRED,
            cls.ENDORSED,
            cls.AMENDED,
            cls.DEL_REQ,
            cls.XFER,
            cls.ATT_XFER,
            cls.ISS_REG,
            cls.ISS_TIER,
            cls.ISS_REM,
            cls.ADM_INIT,
            cls.ADM_XFER,
            cls.ADM_ADD,
            cls.ADM_REM,
            cls.ADM_PROP,
            cls.CLM_TYPE,
            cls.ATT_REQ,
            cls.REQ_OK,
            cls.REQ_NO,
            cls.REQ_CNCL,
            cls.DEL_CRTD,
            cls.DEL_RVKD,
            cls.WL_ON,
            cls.WL_ADD,
            cls.WL_REM,
            cls.MS_PROP,
            cls.MS_SIGN,
            cls.MS_ACTV,
            cls.MS_CANCEL,
            cls.DISPUTED,
            cls.DSP_RES,
            cls.PAUSED,
            cls.UNPAUSED,
            cls.TMPL_CRT,
            cls.TPL_DEL,
            cls.CNCL_INI,
            cls.PROP_NEW,
            cls.PROP_OK,
            cls.PROP_EXE,
            cls.TL_START,
            cls.EXP_HOOK,
        ]

    @classmethod
    def is_valid_topic(cls, topic: str) -> bool:
        """Check if a topic string is a valid event topic."""
        return topic in cls.all_topics()


# ─── Event Topic Categories ───────────────────────────────────────────────────

class EventCategories:
    """Event topic categories for convenient subscription grouping."""
    
    ATTESTATION_LIFECYCLE = [
        EventTopics.CREATED,
        EventTopics.IMPORTED,
        EventTopics.BRIDGED,
        EventTopics.REVOKED,
        EventTopics.RENEWED,
        EventTopics.UPDATED,
        EventTopics.EXPIRED,
        EventTopics.ENDORSED,
        EventTopics.AMENDED,
        EventTopics.XFER,
    ]

    ISSUER_COMPLIANCE = [
        EventTopics.ISS_REG,
        EventTopics.ISS_TIER,
        EventTopics.ISS_REM,
        EventTopics.WL_ON,
        EventTopics.WL_ADD,
        EventTopics.WL_REM,
        EventTopics.DEL_CRTD,
        EventTopics.DEL_RVKD,
    ]

    REQUEST_LIFECYCLE = [
        EventTopics.ATT_REQ,
        EventTopics.REQ_OK,
        EventTopics.REQ_NO,
        EventTopics.REQ_CNCL,
    ]

    MULTISIG = [
        EventTopics.MS_PROP,
        EventTopics.MS_SIGN,
        EventTopics.MS_ACTV,
        EventTopics.MS_CANCEL,
    ]

    DISPUTE_AMENDMENT = [
        EventTopics.DISPUTED,
        EventTopics.DSP_RES,
        EventTopics.AMENDED,
    ]

    ADMIN_ACTIONS = [
        EventTopics.ADM_INIT,
        EventTopics.ADM_XFER,
        EventTopics.ADM_ADD,
        EventTopics.ADM_REM,
        EventTopics.ADM_PROP,
        EventTopics.PAUSED,
        EventTopics.UNPAUSED,
    ]

    COUNCIL_GOVERNANCE = [
        EventTopics.CNCL_INI,
        EventTopics.PROP_NEW,
        EventTopics.PROP_OK,
        EventTopics.PROP_EXE,
        EventTopics.TL_START,
    ]

    INDEXED_PRIORITY = [
        EventTopics.CREATED,
        EventTopics.REVOKED,
        EventTopics.IMPORTED,
        EventTopics.BRIDGED,
        EventTopics.MS_PROP,
        EventTopics.MS_SIGN,
        EventTopics.MS_ACTV,
        EventTopics.ISS_REG,
    ]


# ─── Data Classes ─────────────────────────────────────────────────────────────

@dataclass
class RawContractEvent:
    """Represents a raw event from the ledger."""
    id: str
    event_type: str
    ledger: int
    tx_hash: str
    topics: List[str]
    data: Any
    timestamp: datetime


@dataclass
class ContractEvent:
    """Represents a mapped/normalized contract event."""
    topic: str
    ledger: int
    tx_hash: str
    data: dict
    timestamp: datetime
    raw: RawContractEvent


# ─── Options and Callbacks ────────────────────────────────────────────────────

@dataclass
class EventSubscriptionOptions:
    """Options for event subscriptions."""
    
    # Event topics to subscribe to. If empty, subscribes to all topics.
    topics: Optional[List[str]] = None
    
    # Filter by subject address
    subject: Optional[str] = None
    
    # Filter by issuer address
    issuer: Optional[str] = None
    
    # Polling interval in milliseconds for direct ledger watching
    polling_interval_ms: int = 5000
    
    # Maximum number of events to fetch per poll
    page_size: int = 100
    
    # Starting ledger for direct ledger watching (optional)
    start_ledger: Optional[int] = None


@dataclass
class GraphQLSubscriptionOptions(EventSubscriptionOptions):
    """Options for GraphQL-based event subscriptions."""
    
    # GraphQL endpoint WebSocket URL
    graphql_url: Optional[str] = None
    
    # Custom headers for WebSocket connection
    headers: Optional[dict] = None
    
    # Reconnection config
    reconnect_attempts: int = 5
    reconnect_delay_ms: int = 1000


@dataclass
class LedgerEventWatchOptions(EventSubscriptionOptions):
    """Options for direct ledger event watching."""
    
    # Soroban RPC endpoint (required)
    rpc_url: str = ""
    
    # Contract ID to watch (required)
    contract_id: str = ""
    
    # Network passphrase
    network_passphrase: str = ""


# ─── Validation ───────────────────────────────────────────────────────────────

def validate_event_topics(topics: List[str]) -> List[str]:
    """Validates that all provided topics are valid event topics.
    
    Args:
        topics: Array of topics to validate
        
    Returns:
        The validated topics array
        
    Raises:
        ValueError: If any topic is invalid
    """
    valid_topics = set(EventTopics.all_topics())
    for topic in topics:
        if not isinstance(topic, str) or topic not in valid_topics:
            raise ValueError(
                f'Invalid event topic: "{topic}". See EventTopics for valid values.'
            )
    return topics


def normalize_event_topic(raw: str) -> Optional[str]:
    """Normalizes event topic constants to their string representation.
    
    Args:
        raw: Raw topic string from ledger event
        
    Returns:
        Normalized topic, or None if not recognized
    """
    normalized = raw.lower().strip()
    if EventTopics.is_valid_topic(normalized):
        return normalized
    return None


def get_topic_description(topic: str) -> str:
    """Returns a human-readable description of an event topic.
    
    Args:
        topic: The event topic
        
    Returns:
        Description of what this topic represents
    """
    descriptions = {
        EventTopics.CREATED: "Attestation created",
        EventTopics.IMPORTED: "Attestation imported from external source",
        EventTopics.BRIDGED: "Attestation bridged from another blockchain",
        EventTopics.REVOKED: "Attestation revoked by issuer",
        EventTopics.RENEWED: "Attestation expiration renewed",
        EventTopics.UPDATED: "Attestation updated",
        EventTopics.EXPIRED: "Attestation expired",
        EventTopics.ENDORSED: "Attestation endorsed",
        EventTopics.AMENDED: "Attestation metadata amended",
        EventTopics.DEL_REQ: "Deletion requested",
        EventTopics.XFER: "Attestation issuer transferred",
        EventTopics.ATT_XFER: "Attestation transferred",
        EventTopics.ISS_REG: "Issuer registered",
        EventTopics.ISS_TIER: "Issuer tier updated",
        EventTopics.ISS_REM: "Issuer removed",
        EventTopics.ADM_INIT: "Admin initialized",
        EventTopics.ADM_XFER: "Admin transferred",
        EventTopics.ADM_ADD: "Admin added",
        EventTopics.ADM_REM: "Admin removed",
        EventTopics.ADM_PROP: "Admin transfer proposed",
        EventTopics.CLM_TYPE: "Claim type registered",
        EventTopics.ATT_REQ: "Attestation requested",
        EventTopics.REQ_OK: "Request fulfilled",
        EventTopics.REQ_NO: "Request rejected",
        EventTopics.REQ_CNCL: "Request cancelled",
        EventTopics.DEL_CRTD: "Delegation created",
        EventTopics.DEL_RVKD: "Delegation revoked",
        EventTopics.WL_ON: "Whitelist mode enabled",
        EventTopics.WL_ADD: "Subject added to whitelist",
        EventTopics.WL_REM: "Subject removed from whitelist",
        EventTopics.MS_PROP: "Multi-sig proposal created",
        EventTopics.MS_SIGN: "Multi-sig proposal cosigned",
        EventTopics.MS_ACTV: "Multi-sig proposal activated",
        EventTopics.MS_CANCEL: "Multi-sig proposal cancelled",
        EventTopics.DISPUTED: "Dispute raised",
        EventTopics.DSP_RES: "Dispute resolved",
        EventTopics.PAUSED: "Contract paused",
        EventTopics.UNPAUSED: "Contract unpaused",
        EventTopics.TMPL_CRT: "Template created",
        EventTopics.TPL_DEL: "Template deleted",
        EventTopics.CNCL_INI: "Council initialized",
        EventTopics.PROP_NEW: "Council proposal created",
        EventTopics.PROP_OK: "Council proposal approved",
        EventTopics.PROP_EXE: "Council proposal executed",
        EventTopics.TL_START: "Timelock started",
        EventTopics.EXP_HOOK: "Expiration hook triggered",
    }
    return descriptions.get(topic, "Unknown event")


# ─── Type Aliases ────────────────────────────────────────────────────────────

EventCallback = Callable[[ContractEvent], None]
AsyncEventCallback = Callable[[ContractEvent], Any]
