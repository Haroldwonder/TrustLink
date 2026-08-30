"""Tests for trustlink.events topic definitions and parsing helpers (Issue #1151)."""

from datetime import datetime

import pytest

from trustlink.events import (
    EventTopics,
    EventCategories,
    RawContractEvent,
    ContractEvent,
    EventSubscriptionOptions,
    GraphQLSubscriptionOptions,
    LedgerEventWatchOptions,
    validate_event_topics,
    normalize_event_topic,
    get_topic_description,
)


class TestEventTopics:
    """Test the canonical event topic constants."""

    def test_all_topics_is_non_empty(self):
        """all_topics() should return a non-empty list."""
        topics = EventTopics.all_topics()
        assert isinstance(topics, list)
        assert len(topics) > 0

    def test_all_topics_has_no_duplicates(self):
        """Every topic in all_topics() should be unique."""
        topics = EventTopics.all_topics()
        assert len(topics) == len(set(topics))

    def test_all_topics_respect_soroban_symbol_length(self):
        """Every topic must be <=9 characters (Soroban symbol_short! limit)."""
        for topic in EventTopics.all_topics():
            assert len(topic) <= 9, f'Topic "{topic}" exceeds 9 characters'

    def test_all_topics_are_lowercase_strings(self):
        """Topics are matched case-sensitively, so they must be lowercase strings."""
        for topic in EventTopics.all_topics():
            assert isinstance(topic, str)
            assert topic == topic.lower()

    def test_is_valid_topic_accepts_known_topics(self):
        """is_valid_topic() should return True for every listed topic."""
        for topic in EventTopics.all_topics():
            assert EventTopics.is_valid_topic(topic) is True

    def test_is_valid_topic_rejects_unknown_topic(self):
        """is_valid_topic() should return False for an unlisted topic."""
        assert EventTopics.is_valid_topic("not_a_topic") is False

    def test_is_valid_topic_is_case_sensitive(self):
        """is_valid_topic() should not accept uppercased variants."""
        assert EventTopics.is_valid_topic("CREATED") is False

    def test_known_constants_are_present(self):
        """A representative set of constants should be included in all_topics()."""
        topics = EventTopics.all_topics()
        for topic in (
            EventTopics.CREATED,
            EventTopics.REVOKED,
            EventTopics.ISS_REG,
            EventTopics.MS_PROP,
            EventTopics.PAUSED,
        ):
            assert topic in topics


class TestEventCategories:
    """Test that category groupings reference only valid topics."""

    ALL_CATEGORIES = [
        EventCategories.ATTESTATION_LIFECYCLE,
        EventCategories.ISSUER_COMPLIANCE,
        EventCategories.REQUEST_LIFECYCLE,
        EventCategories.MULTISIG,
        EventCategories.DISPUTE_AMENDMENT,
        EventCategories.ADMIN_ACTIONS,
        EventCategories.COUNCIL_GOVERNANCE,
        EventCategories.INDEXED_PRIORITY,
    ]

    def test_categories_only_contain_valid_topics(self):
        """Every topic in every category must be a valid event topic."""
        valid = set(EventTopics.all_topics())
        for category in self.ALL_CATEGORIES:
            assert len(category) > 0
            for topic in category:
                assert topic in valid

    def test_categories_have_no_internal_duplicates(self):
        """A single category should not list the same topic twice."""
        for category in self.ALL_CATEGORIES:
            assert len(category) == len(set(category))


class TestValidateEventTopics:
    """Test validate_event_topics()."""

    def test_returns_the_same_topics_when_valid(self):
        """Valid topics should be returned unchanged."""
        topics = [EventTopics.CREATED, EventTopics.REVOKED]
        assert validate_event_topics(topics) == topics

    def test_empty_list_is_valid(self):
        """An empty list has no invalid topics and should pass."""
        assert validate_event_topics([]) == []

    def test_raises_on_unknown_topic(self):
        """An unknown topic should raise ValueError."""
        with pytest.raises(ValueError):
            validate_event_topics([EventTopics.CREATED, "bogus"])

    def test_raises_on_non_string_topic(self):
        """A non-string topic should raise ValueError."""
        with pytest.raises(ValueError):
            validate_event_topics([123])  # type: ignore[list-item]

    def test_error_message_includes_offending_topic(self):
        """The ValueError message should name the invalid topic."""
        with pytest.raises(ValueError, match="bogus"):
            validate_event_topics(["bogus"])


class TestNormalizeEventTopic:
    """Test normalize_event_topic()."""

    def test_passthrough_for_canonical_topic(self):
        """A canonical topic should normalize to itself."""
        assert normalize_event_topic(EventTopics.CREATED) == EventTopics.CREATED

    def test_lowercases_input(self):
        """Uppercased input should be normalized to the canonical lowercase topic."""
        assert normalize_event_topic("CREATED") == EventTopics.CREATED

    def test_strips_surrounding_whitespace(self):
        """Surrounding whitespace should be stripped before matching."""
        assert normalize_event_topic("  created  ") == EventTopics.CREATED

    def test_returns_none_for_unknown_topic(self):
        """An unrecognized topic should normalize to None."""
        assert normalize_event_topic("definitely_not_real") is None


class TestGetTopicDescription:
    """Test get_topic_description()."""

    def test_returns_description_for_known_topic(self):
        """A known topic should map to its human-readable description."""
        assert get_topic_description(EventTopics.CREATED) == "Attestation created"

    def test_every_topic_has_a_description(self):
        """Every canonical topic should have a non-default description."""
        for topic in EventTopics.all_topics():
            description = get_topic_description(topic)
            assert description
            assert description != "Unknown event"

    def test_unknown_topic_returns_default(self):
        """An unknown topic should fall back to the default description."""
        assert get_topic_description("nope") == "Unknown event"


class TestEventDataclasses:
    """Test the event data containers and subscription option defaults."""

    def _raw_event(self):
        return RawContractEvent(
            id="ev-1",
            event_type="contract",
            ledger=100,
            tx_hash="deadbeef",
            topics=[EventTopics.CREATED],
            data={"attestation_id": "abc"},
            timestamp=datetime(2026, 1, 1),
        )

    def test_raw_contract_event_fields(self):
        """RawContractEvent should retain the values it was constructed with."""
        raw = self._raw_event()
        assert raw.id == "ev-1"
        assert raw.ledger == 100
        assert raw.topics == [EventTopics.CREATED]
        assert raw.data == {"attestation_id": "abc"}

    def test_contract_event_wraps_raw_event(self):
        """ContractEvent should carry the normalized fields and the raw event."""
        raw = self._raw_event()
        event = ContractEvent(
            topic=EventTopics.CREATED,
            ledger=raw.ledger,
            tx_hash=raw.tx_hash,
            data={"attestation_id": "abc"},
            timestamp=raw.timestamp,
            raw=raw,
        )
        assert event.topic == EventTopics.CREATED
        assert event.raw is raw

    def test_subscription_options_defaults(self):
        """EventSubscriptionOptions should provide sensible defaults."""
        options = EventSubscriptionOptions()
        assert options.topics is None
        assert options.subject is None
        assert options.issuer is None
        assert options.polling_interval_ms == 5000
        assert options.page_size == 100
        assert options.start_ledger is None

    def test_graphql_options_inherit_base_defaults(self):
        """GraphQLSubscriptionOptions should extend the base options."""
        options = GraphQLSubscriptionOptions()
        assert isinstance(options, EventSubscriptionOptions)
        assert options.page_size == 100
        assert options.graphql_url is None
        assert options.reconnect_attempts == 5
        assert options.reconnect_delay_ms == 1000

    def test_ledger_watch_options_inherit_base_defaults(self):
        """LedgerEventWatchOptions should extend the base options."""
        options = LedgerEventWatchOptions()
        assert isinstance(options, EventSubscriptionOptions)
        assert options.rpc_url == ""
        assert options.contract_id == ""
        assert options.network_passphrase == ""
