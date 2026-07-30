"""TrustLink Python bindings."""

from .client import TrustLinkClient
from .async_client import AsyncTrustLinkClient
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
)
from .events import (
    EventTopics,
    EventCategories,
    ContractEvent,
    RawContractEvent,
    EventSubscriptionOptions,
    GraphQLSubscriptionOptions,
    LedgerEventWatchOptions,
    validate_event_topics,
    normalize_event_topic,
    get_topic_description,
)
from .event_subscriptions import (
    subscribe_to_direct_ledger_events,
    subscribe_to_graphql_events,
    EventSubscription,
)

__version__ = "0.1.0"
__all__ = [
    "TrustLinkClient",
    "AsyncTrustLinkClient",
    "Attestation",
    "AttestationStatus",
    "AttestationTemplate",
    "ClaimTypeInfo",
    "ContractConfig",
    "ContractMetadata",
    "Delegation",
    "GlobalStats",
    "IssuerStats",
    "MultiSigProposal",
    "TrustLinkError",
    "ContractError",
    "EventTopics",
    "EventCategories",
    "ContractEvent",
    "RawContractEvent",
    "EventSubscriptionOptions",
    "GraphQLSubscriptionOptions",
    "LedgerEventWatchOptions",
    "validate_event_topics",
    "normalize_event_topic",
    "get_topic_description",
    "subscribe_to_direct_ledger_events",
    "subscribe_to_graphql_events",
    "EventSubscription",
]
