//! Shared TTL constants for TrustLink.

/// Default TTL for attestations in days.
pub const DEFAULT_TTL_DAYS: u32 = 30;

/// Seconds in one day.
pub const SECS_PER_DAY: u64 = 86_400;

/// Expected number of ledgers in one day (5-second ledger close time).
pub const DAY_IN_LEDGERS: u32 = 17_280;

/// Default instance lifetime in ledgers.
pub const DEFAULT_INSTANCE_LIFETIME: u32 = DAY_IN_LEDGERS * DEFAULT_TTL_DAYS;

/// Only extend TTL on read if remaining TTL drops below this threshold.
pub const MIN_TTL_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;
