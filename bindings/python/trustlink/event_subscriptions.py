"""Event subscription helpers for TrustLink Python SDK.

Provides convenient methods to subscribe to contract events via GraphQL subscriptions
or direct ledger event watching with optional topic filtering.
"""

import asyncio
import json
import logging
from typing import Optional, List, Callable, Any
from datetime import datetime

from stellar_sdk import Server
from .events import (
    EventTopics,
    EventSubscriptionOptions,
    GraphQLSubscriptionOptions,
    LedgerEventWatchOptions,
    ContractEvent,
    RawContractEvent,
    AsyncEventCallback,
    EventCallback,
    validate_event_topics,
)

logger = logging.getLogger(__name__)

# Optional websockets import for GraphQL subscriptions
try:
    import websockets
except ImportError:
    websockets = None  # type: ignore


class EventSubscription:
    """A subscription handle that can be used to unsubscribe from events."""

    def __init__(self):
        self._is_active = True
        self._tasks: List[asyncio.Task[Any]] = []

    async def unsubscribe(self) -> None:
        """Unsubscribe from the event stream."""
        self._is_active = False
        # Cancel all pending tasks
        for task in self._tasks:
            if not task.done():
                task.cancel()
        # Wait for all tasks to complete
        if self._tasks:
            await asyncio.gather(*self._tasks, return_exceptions=True)

    def is_active(self) -> bool:
        """Check if the subscription is still active."""
        return self._is_active

    def _add_task(self, task: asyncio.Task[Any]) -> None:
        """Add a task to be tracked."""
        self._tasks.append(task)


async def subscribe_to_direct_ledger_events(
    options: LedgerEventWatchOptions,
    callback: AsyncEventCallback,
) -> EventSubscription:
    """Subscribes to contract events via direct ledger watching with optional topic filtering.

    This method polls the Soroban RPC for new ledger events, allowing fine-grained
    filtering by topics without depending on the indexer.

    Args:
        options: Configuration for ledger event watching
        callback: Async callback invoked for each matching event

    Returns:
        EventSubscription handle for unsubscribing

    Example:
        ```python
        from trustlink import (
            subscribe_to_direct_ledger_events,
            LedgerEventWatchOptions,
            EventTopics,
        )

        async def handle_event(event):
            print(f"Event: {event.topic} at ledger {event.ledger}")

        sub = await subscribe_to_direct_ledger_events(
            LedgerEventWatchOptions(
                rpc_url="https://soroban-testnet.stellar.org",
                contract_id="C...",
                network_passphrase="Test SDF Future Network ; October 2024",
                topics=[EventTopics.CREATED, EventTopics.REVOKED],
            ),
            handle_event,
        )
        ```
    """
    if not websockets and isinstance(options, GraphQLSubscriptionOptions):
        raise ImportError("websockets library required for GraphQL subscriptions")

    # Validate topics if provided
    topics = validate_event_topics(options.topics) if options.topics else None

    server = Server(options.rpc_url, client=None)

    subscription = EventSubscription()

    # Get initial ledger if not specified
    cursor = options.start_ledger
    if not cursor:
        latest_ledger = server.get_latest_ledger()
        cursor = latest_ledger.sequence

    async def poll_loop() -> None:
        """Poll loop for direct ledger watching."""
        nonlocal cursor

        while subscription.is_active():
            try:
                # Get latest ledger
                latest_ledger = server.get_latest_ledger()
                latest = latest_ledger.sequence

                if cursor <= latest:
                    # Fetch events in range
                    events = server.get_events(
                        start_ledger=cursor,
                        filters=[
                            {
                                "type": "contract",
                                "contractIds": [options.contract_id],
                            }
                        ],
                        limit=options.page_size,
                    )

                    for event in events.get("events", []):
                        if not subscription.is_active():
                            break

                        # Filter by topic
                        event_topics = event.get("topics", [])
                        if not event_topics:
                            continue

                        event_topic = event_topics[0]
                        if topics and event_topic not in topics:
                            continue

                        # Filter by subject
                        if options.subject and len(event_topics) > 1:
                            if event_topics[1] != options.subject:
                                continue

                        # Filter by issuer
                        if options.issuer and len(event_topics) > 1:
                            if event_topics[1] != options.issuer:
                                continue

                        # Map and invoke callback
                        mapped_event = _map_raw_event(event)
                        if mapped_event:
                            result = callback(mapped_event)
                            if hasattr(result, "__await__"):
                                await result

                    cursor = latest + 1

                # Wait before next poll
                await asyncio.sleep(options.polling_interval_ms / 1000.0)

            except Exception as e:
                logger.error(f"Error polling ledger events: {e}")
                if subscription.is_active():
                    await asyncio.sleep(options.polling_interval_ms / 1000.0)

    # Start polling loop in background
    task = asyncio.create_task(poll_loop())
    subscription._add_task(task)

    return subscription


async def subscribe_to_graphql_events(
    options: GraphQLSubscriptionOptions,
    callback: AsyncEventCallback,
) -> EventSubscription:
    """Subscribes to contract events via GraphQL subscriptions from the indexer.

    This method uses the indexer's GraphQL subscriptions for real-time event streaming.

    Args:
        options: Configuration for GraphQL subscriptions
        callback: Async callback invoked for each matching event

    Returns:
        EventSubscription handle for unsubscribing

    Example:
        ```python
        from trustlink import (
            subscribe_to_graphql_events,
            GraphQLSubscriptionOptions,
            EventTopics,
            EventCategories,
        )

        async def handle_event(event):
            print(f"Event: {event.topic}")

        sub = await subscribe_to_graphql_events(
            GraphQLSubscriptionOptions(
                graphql_url="wss://indexer.trustlink.io/graphql",
                topics=EventCategories.ATTESTATION_LIFECYCLE,
                subject="GBBD...",
            ),
            handle_event,
        )
        ```
    """
    if not websockets:
        raise ImportError("websockets library required for GraphQL subscriptions")

    # Validate topics if provided
    topics = validate_event_topics(options.topics) if options.topics else None

    graphql_url = (
        options.graphql_url or "wss://localhost:4000/graphql"
    )

    subscription = EventSubscription()

    async def connect_and_listen() -> None:
        """Connect to GraphQL WebSocket and listen for events."""
        reconnect_count = 0

        while subscription.is_active() and reconnect_count < options.reconnect_attempts:
            try:
                async with websockets.connect(
                    graphql_url,
                    subprotocols=["graphql-transport-ws"],
                    extra_headers=options.headers or {},
                ) as ws:
                    reconnect_count = 0

                    # Send connection init
                    await ws.send(
                        json.dumps(
                            {"type": "connection_init", "payload": {}}
                        )
                    )

                    # Subscribe to relevant topics
                    await _subscribe_to_relevant_topics(
                        ws, options, topics
                    )

                    # Listen for messages
                    async for message_str in ws:
                        if not subscription.is_active():
                            break

                        try:
                            message = json.loads(message_str)
                        except json.JSONDecodeError:
                            continue

                        if message.get("type") == "next":
                            payload = message.get("payload", {})
                            data = payload.get("data", {})

                            # Handle different subscription types
                            if "onAttestationCreated" in data:
                                if (
                                    not topics
                                    or not topics
                                    or "created" in topics
                                ):
                                    event_data = data["onAttestationCreated"]
                                    if (
                                        not options.subject
                                        or event_data.get("subject")
                                        == options.subject
                                    ):
                                        mapped_event = _map_graphql_event(
                                            "created", event_data
                                        )
                                        result = callback(mapped_event)
                                        if hasattr(result, "__await__"):
                                            await result

                            if "onAttestationRevoked" in data:
                                if (
                                    not topics
                                    or not topics
                                    or "revoked" in topics
                                ):
                                    event_data = data["onAttestationRevoked"]
                                    if (
                                        not options.issuer
                                        or event_data.get("issuer")
                                        == options.issuer
                                    ):
                                        mapped_event = _map_graphql_event(
                                            "revoked", event_data
                                        )
                                        result = callback(mapped_event)
                                        if hasattr(result, "__await__"):
                                            await result

                            if "onIssuerRegistered" in data:
                                if (
                                    not topics
                                    or not topics
                                    or "iss_reg" in topics
                                ):
                                    event_data = data["onIssuerRegistered"]
                                    mapped_event = _map_graphql_event(
                                        "iss_reg", event_data
                                    )
                                    result = callback(mapped_event)
                                    if hasattr(result, "__await__"):
                                        await result

            except Exception as e:
                logger.error(f"GraphQL subscription error: {e}")
                if subscription.is_active():
                    reconnect_count += 1
                    if reconnect_count < options.reconnect_attempts:
                        delay = (
                            options.reconnect_delay_ms / 1000.0
                            * (2 ** (reconnect_count - 1))
                        )
                        await asyncio.sleep(delay)

    # Start connection loop in background
    task = asyncio.create_task(connect_and_listen())
    subscription._add_task(task)

    return subscription


# ─── Helper Functions ────────────────────────────────────────────────────────

async def _subscribe_to_relevant_topics(
    ws: Any, options: GraphQLSubscriptionOptions, topics: Optional[List[str]]
) -> None:
    """Subscribe to relevant GraphQL subscriptions based on topic filters."""
    # Subscribe to attestation created
    if not topics or not topics or "created" in (topics or []):
        query = "subscription { onAttestationCreated { id subject issuer claimType } }"
        await ws.send(
            json.dumps(
                {
                    "id": f"sub_created_{id(ws)}",
                    "type": "subscribe",
                    "payload": {"query": query},
                }
            )
        )

    # Subscribe to attestation revoked
    if not topics or not topics or "revoked" in (topics or []):
        query = "subscription { onAttestationRevoked { id issuer revokedAt } }"
        await ws.send(
            json.dumps(
                {
                    "id": f"sub_revoked_{id(ws)}",
                    "type": "subscribe",
                    "payload": {"query": query},
                }
            )
        )

    # Subscribe to issuer registered
    if not topics or not topics or "iss_reg" in (topics or []):
        query = "subscription { onIssuerRegistered { issuer registeredAt } }"
        await ws.send(
            json.dumps(
                {
                    "id": f"sub_iss_reg_{id(ws)}",
                    "type": "subscribe",
                    "payload": {"query": query},
                }
            )
        )


def _map_raw_event(raw_event: dict) -> Optional[ContractEvent]:
    """Maps a raw Soroban event to a normalized ContractEvent."""
    try:
        topics = raw_event.get("topics", [])
        if not topics:
            return None

        topic = topics[0]
        data = raw_event.get("value", {})

        raw = RawContractEvent(
            id=raw_event.get("id", ""),
            event_type=raw_event.get("type", "contract"),
            ledger=raw_event.get("ledger", 0),
            tx_hash=raw_event.get("txHash", ""),
            topics=topics,
            data=data,
            timestamp=datetime.now(),
        )

        return ContractEvent(
            topic=topic,
            ledger=raw_event.get("ledger", 0),
            tx_hash=raw_event.get("txHash", ""),
            data=data if isinstance(data, dict) else {},
            timestamp=datetime.now(),
            raw=raw,
        )
    except Exception as e:
        logger.error(f"Error mapping raw event: {e}")
        return None


def _map_graphql_event(topic: str, data: dict) -> ContractEvent:
    """Maps a GraphQL event to a normalized ContractEvent."""
    raw = RawContractEvent(
        id=json.dumps(data),
        event_type="contract",
        ledger=0,
        tx_hash="",
        topics=[topic],
        data=data,
        timestamp=datetime.now(),
    )

    return ContractEvent(
        topic=topic,
        ledger=0,
        tx_hash="",
        data=data,
        timestamp=datetime.now(),
        raw=raw,
    )
