"""Tests for trustlink.event_subscriptions.

Covers:
  * the ``EventSubscription`` handle (activity flag, task tracking, unsubscribe)
  * ``subscribe_to_direct_ledger_events`` — poll loop, event dispatch, topic /
    subject filtering, and that ``unsubscribe`` stops polling
  * ``subscribe_to_graphql_events`` — websocket dispatch, ``unsubscribe`` and the
    missing-``websockets`` ImportError guard
  * the raw / GraphQL event mapping helpers

The module imports ``stellar_sdk.Server`` and (optionally) ``websockets`` at
import time; both are stubbed here so the tests never touch the network.
"""

import asyncio
import json

import pytest

from trustlink import (
    EventSubscription,
    GraphQLSubscriptionOptions,
    LedgerEventWatchOptions,
    subscribe_to_direct_ledger_events,
    subscribe_to_graphql_events,
)
from trustlink import event_subscriptions as es


# ── async helper ──────────────────────────────────────────────────────────────
# The subscribe_* functions spawn background tasks with asyncio.create_task, so
# they must run inside an event loop. We drive them with a fresh loop per test
# rather than depending on the pytest-asyncio plugin being configured.

def run_async(coro):
    loop = asyncio.new_event_loop()
    try:
        return loop.run_until_complete(coro)
    finally:
        loop.close()


# ── fakes ─────────────────────────────────────────────────────────────────────

class _FakeLedger:
    def __init__(self, sequence: int):
        self.sequence = sequence


class _FakeServer:
    """Stand-in for stellar_sdk.Server used by direct ledger watching."""

    def __init__(self, events, latest_sequence: int = 1):
        self._events = events
        self._latest_sequence = latest_sequence
        self.get_events_calls = []

    def get_latest_ledger(self):
        return _FakeLedger(self._latest_sequence)

    def get_events(self, **kwargs):
        self.get_events_calls.append(kwargs)
        return {"events": list(self._events)}


class _FakeWebSocket:
    """Async-context-manager / async-iterator stub for websockets.connect()."""

    def __init__(self, messages):
        self._messages = messages
        self.sent = []

    async def __aenter__(self):
        return self

    async def __aexit__(self, *exc):
        return False

    async def send(self, message):
        self.sent.append(message)

    async def __aiter__(self):
        for message in self._messages:
            yield message
        # Keep the "connection" open so connect_and_listen() does not busy-loop
        # reconnecting; the test cancels this via subscription.unsubscribe().
        while True:
            await asyncio.sleep(3600)


def _raw_event(topic="created", subject="GSUBJECT", **overrides):
    event = {
        "id": "ev-1",
        "type": "contract",
        "ledger": 100,
        "txHash": "txhash-1",
        "topics": [topic, subject],
        "value": {"claim": "KYC_PASSED"},
    }
    event.update(overrides)
    return event


# ── EventSubscription handle ──────────────────────────────────────────────────

class TestEventSubscriptionHandle:
    def test_starts_active(self):
        assert EventSubscription().is_active() is True

    def test_unsubscribe_marks_inactive(self):
        sub = EventSubscription()
        run_async(sub.unsubscribe())
        assert sub.is_active() is False

    def test_unsubscribe_cancels_tracked_tasks(self):
        async def _test():
            sub = EventSubscription()

            async def _forever():
                await asyncio.sleep(3600)

            task = asyncio.ensure_future(_forever())
            sub._add_task(task)
            await asyncio.sleep(0)  # let the task start

            await sub.unsubscribe()
            assert task.cancelled() or task.done()

        run_async(_test())

    def test_unsubscribe_is_idempotent(self):
        async def _test():
            sub = EventSubscription()
            await sub.unsubscribe()
            await sub.unsubscribe()  # must not raise
            assert sub.is_active() is False

        run_async(_test())


# ── subscribe_to_direct_ledger_events ────────────────────────────────────────

class TestDirectLedgerSubscription:
    def _patch_server(self, monkeypatch, fake):
        monkeypatch.setattr(es, "Server", lambda *a, **k: fake)

    def test_returns_active_subscription_handle(self, monkeypatch):
        self._patch_server(monkeypatch, _FakeServer([]))

        async def _test():
            sub = await subscribe_to_direct_ledger_events(
                LedgerEventWatchOptions(
                    rpc_url="https://rpc.example",
                    contract_id="C123",
                    start_ledger=1,
                    polling_interval_ms=10,
                ),
                lambda _e: None,
            )
            assert isinstance(sub, EventSubscription)
            assert sub.is_active() is True
            await sub.unsubscribe()

        run_async(_test())

    def test_dispatches_matching_event_to_callback(self, monkeypatch):
        self._patch_server(monkeypatch, _FakeServer([_raw_event(topic="created")]))
        received = []

        async def _test():
            async def on_event(event):
                received.append(event)

            sub = await subscribe_to_direct_ledger_events(
                LedgerEventWatchOptions(
                    rpc_url="https://rpc.example",
                    contract_id="C123",
                    start_ledger=1,
                    polling_interval_ms=10,
                    topics=["created"],
                ),
                on_event,
            )
            await asyncio.sleep(0.1)
            await sub.unsubscribe()

        run_async(_test())
        assert len(received) == 1
        assert received[0].topic == "created"
        assert received[0].data == {"claim": "KYC_PASSED"}

    def test_sync_callback_is_supported(self, monkeypatch):
        self._patch_server(monkeypatch, _FakeServer([_raw_event(topic="revoked")]))
        received = []

        async def _test():
            sub = await subscribe_to_direct_ledger_events(
                LedgerEventWatchOptions(
                    rpc_url="https://rpc.example",
                    contract_id="C123",
                    start_ledger=1,
                    polling_interval_ms=10,
                    topics=["revoked"],
                ),
                received.append,
            )
            await asyncio.sleep(0.1)
            await sub.unsubscribe()

        run_async(_test())
        assert [e.topic for e in received] == ["revoked"]

    def test_filters_out_non_matching_topic(self, monkeypatch):
        self._patch_server(monkeypatch, _FakeServer([_raw_event(topic="revoked")]))
        received = []

        async def _test():
            sub = await subscribe_to_direct_ledger_events(
                LedgerEventWatchOptions(
                    rpc_url="https://rpc.example",
                    contract_id="C123",
                    start_ledger=1,
                    polling_interval_ms=10,
                    topics=["created"],
                ),
                received.append,
            )
            await asyncio.sleep(0.1)
            await sub.unsubscribe()

        run_async(_test())
        assert received == []

    def test_filters_by_subject(self, monkeypatch):
        self._patch_server(
            monkeypatch,
            _FakeServer([_raw_event(topic="created", subject="GOTHER")]),
        )
        received = []

        async def _test():
            sub = await subscribe_to_direct_ledger_events(
                LedgerEventWatchOptions(
                    rpc_url="https://rpc.example",
                    contract_id="C123",
                    start_ledger=1,
                    polling_interval_ms=10,
                    topics=["created"],
                    subject="GSUBJECT",
                ),
                received.append,
            )
            await asyncio.sleep(0.1)
            await sub.unsubscribe()

        run_async(_test())
        assert received == []

    def test_unsubscribe_stops_polling(self, monkeypatch):
        fake = _FakeServer([])
        self._patch_server(monkeypatch, fake)

        async def _test():
            sub = await subscribe_to_direct_ledger_events(
                LedgerEventWatchOptions(
                    rpc_url="https://rpc.example",
                    contract_id="C123",
                    start_ledger=1,
                    polling_interval_ms=10,
                ),
                lambda _e: None,
            )
            await asyncio.sleep(0.05)
            await sub.unsubscribe()
            calls_after_stop = len(fake.get_events_calls)
            await asyncio.sleep(0.05)
            assert len(fake.get_events_calls) == calls_after_stop
            assert sub.is_active() is False

        run_async(_test())

    def test_invalid_topic_raises_before_subscribing(self, monkeypatch):
        self._patch_server(monkeypatch, _FakeServer([]))

        async def _test():
            with pytest.raises(ValueError, match="Invalid event topic"):
                await subscribe_to_direct_ledger_events(
                    LedgerEventWatchOptions(
                        rpc_url="https://rpc.example",
                        contract_id="C123",
                        start_ledger=1,
                        topics=["not_a_real_topic"],
                    ),
                    lambda _e: None,
                )

        run_async(_test())


# ── subscribe_to_graphql_events ──────────────────────────────────────────────

class TestGraphQLSubscription:
    def test_raises_without_websockets(self, monkeypatch):
        monkeypatch.setattr(es, "websockets", None)

        async def _test():
            with pytest.raises(ImportError, match="websockets"):
                await subscribe_to_graphql_events(
                    GraphQLSubscriptionOptions(graphql_url="wss://indexer.example/graphql"),
                    lambda _e: None,
                )

        run_async(_test())

    def _install_fake_ws(self, monkeypatch, messages):
        holder = {}

        class _FakeWebSocketsModule:
            @staticmethod
            def connect(*args, **kwargs):
                ws = _FakeWebSocket(messages)
                holder["ws"] = ws
                holder["connect_args"] = (args, kwargs)
                return ws

        monkeypatch.setattr(es, "websockets", _FakeWebSocketsModule)
        return holder

    def test_dispatches_created_event(self, monkeypatch):
        message = json.dumps(
            {
                "type": "next",
                "payload": {
                    "data": {
                        "onAttestationCreated": {
                            "id": "att-1",
                            "subject": "GSUBJECT",
                            "issuer": "GISSUER",
                            "claimType": "KYC_PASSED",
                        }
                    }
                },
            }
        )
        self._install_fake_ws(monkeypatch, [message])
        received = []

        async def _test():
            async def on_event(event):
                received.append(event)

            sub = await subscribe_to_graphql_events(
                GraphQLSubscriptionOptions(graphql_url="wss://indexer.example/graphql"),
                on_event,
            )
            await asyncio.sleep(0.1)
            await sub.unsubscribe()

        run_async(_test())
        assert len(received) == 1
        assert received[0].topic == "created"
        assert received[0].data["subject"] == "GSUBJECT"

    def test_subject_filter_excludes_other_subjects(self, monkeypatch):
        message = json.dumps(
            {
                "type": "next",
                "payload": {
                    "data": {
                        "onAttestationCreated": {
                            "id": "att-2",
                            "subject": "GOTHER",
                            "issuer": "GISSUER",
                        }
                    }
                },
            }
        )
        self._install_fake_ws(monkeypatch, [message])
        received = []

        async def _test():
            sub = await subscribe_to_graphql_events(
                GraphQLSubscriptionOptions(
                    graphql_url="wss://indexer.example/graphql",
                    subject="GSUBJECT",
                ),
                received.append,
            )
            await asyncio.sleep(0.1)
            await sub.unsubscribe()

        run_async(_test())
        assert received == []

    def test_sends_connection_init_and_subscribe_frames(self, monkeypatch):
        holder = self._install_fake_ws(monkeypatch, [])

        async def _test():
            sub = await subscribe_to_graphql_events(
                GraphQLSubscriptionOptions(graphql_url="wss://indexer.example/graphql"),
                lambda _e: None,
            )
            await asyncio.sleep(0.05)
            await sub.unsubscribe()

        run_async(_test())
        sent = [json.loads(m) for m in holder["ws"].sent]
        types = [m["type"] for m in sent]
        assert types[0] == "connection_init"
        assert "subscribe" in types

    def test_ignores_non_json_messages(self, monkeypatch):
        self._install_fake_ws(monkeypatch, ["<<not json>>"])
        received = []

        async def _test():
            sub = await subscribe_to_graphql_events(
                GraphQLSubscriptionOptions(graphql_url="wss://indexer.example/graphql"),
                received.append,
            )
            await asyncio.sleep(0.1)
            await sub.unsubscribe()

        run_async(_test())
        assert received == []

    def test_unsubscribe_marks_inactive(self, monkeypatch):
        self._install_fake_ws(monkeypatch, [])

        async def _test():
            sub = await subscribe_to_graphql_events(
                GraphQLSubscriptionOptions(graphql_url="wss://indexer.example/graphql"),
                lambda _e: None,
            )
            assert sub.is_active() is True
            await sub.unsubscribe()
            assert sub.is_active() is False

        run_async(_test())


# ── mapping helpers ──────────────────────────────────────────────────────────

class TestEventMappingHelpers:
    def test_map_raw_event_normalizes_fields(self):
        mapped = es._map_raw_event(_raw_event(topic="created"))
        assert mapped is not None
        assert mapped.topic == "created"
        assert mapped.ledger == 100
        assert mapped.tx_hash == "txhash-1"
        assert mapped.data == {"claim": "KYC_PASSED"}
        assert mapped.raw.topics == ["created", "GSUBJECT"]

    def test_map_raw_event_returns_none_without_topics(self):
        assert es._map_raw_event({"id": "x", "topics": []}) is None

    def test_map_raw_event_coerces_non_dict_value_to_empty_dict(self):
        mapped = es._map_raw_event(_raw_event(value="scalar"))
        assert mapped is not None
        assert mapped.data == {}

    def test_map_graphql_event_sets_topic_and_data(self):
        data = {"id": "att-9", "subject": "GSUBJECT"}
        mapped = es._map_graphql_event("revoked", data)
        assert mapped.topic == "revoked"
        assert mapped.data == data
        assert mapped.raw.topics == ["revoked"]
