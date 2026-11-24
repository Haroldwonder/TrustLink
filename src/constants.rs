//! Shared TTL constants for TrustLink.

/// Default TTL for attestations in days.
pub const DEFAULT_TTL_DAYS: u32 = 30;

/// Maximum contract TTL in days, aligned with network limits.
pub const MAX_TTL_DAYS: u32 = 180;

/// Seconds in one day.
pub const SECS_PER_DAY: u64 = 86_400;

/// Expected number of ledgers in one day (5-second ledger close time).
pub const DAY_IN_LEDGERS: u32 = 17_280;

/// Default instance lifetime in ledgers.
pub const DEFAULT_INSTANCE_LIFETIME: u32 = DAY_IN_LEDGERS * DEFAULT_TTL_DAYS;

/// Only extend TTL on read if remaining TTL drops below this threshold.
pub const MIN_TTL_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;

/// Maximum days for expiring attestations query window (issue #945).
pub const MAX_EXPIRING_WINDOW_DAYS: u32 = 365;

/// Maximum number of required signers in a single multisig proposal.
/// Prevents per-entry size overflow and unbounded iteration in cosign_attestation.
pub const MAX_MULTISIG_SIGNERS: u32 = 20;

/// Issue #1579: Maximum allowed expiration days for a template.
/// Prevents overflow and unreasonable expiration windows.
pub const MAX_TEMPLATE_EXPIRATION_DAYS: u32 = 10_950; // ~30 years

/// Issue #1579: Minimum allowed expiration days for a template.
pub const MIN_TEMPLATE_EXPIRATION_DAYS: u32 = 1;

/// Issue #1579: Maximum length for template IDs (like claim types).
pub const MAX_TEMPLATE_ID_LEN: u32 = 256;

/// Issue #1579: Minimum length for template IDs.
pub const MIN_TEMPLATE_ID_LEN: u32 = 1;

/// Maximum number of audit log entries retained per attestation before the
/// oldest entries are dropped to prevent unbounded growth (issue #1549).
/// Enforced in `Storage::append_audit_entry`.
pub const MAX_AUDIT_LOG_ENTRIES: u32 = 100;

/// Maximum number of attestation version history snapshots retained per
/// attestation before the oldest entries are dropped (issue #1549).
/// Enforced in `Storage::push_attestation_version`.
pub const MAX_VERSION_HISTORY: u32 = 20;

/// Issue #1519: Minimum chunk size for `ChunkedIndex` writes.
/// A chunk size of `0` would make chunked writes impossible.
pub const MIN_CHUNK_SIZE: u32 = 1;

/// Issue #1519: Maximum chunk size for `ChunkedIndex` writes.
/// Prevents writing the whole index into a single ledger entry, which would
/// eventually exceed the ledger entry size limit and trap all creations.
pub const MAX_CHUNK_SIZE: u32 = 1_000;

/// Issue #1519: Minimum multisig proposal TTL in days.
/// A TTL of `0` would make every new proposal expire at creation.
pub const MIN_MULTISIG_TTL_DAYS: u32 = 1;

/// Issue #1519: Maximum multisig proposal TTL in days.
pub const MAX_MULTISIG_TTL_DAYS: u32 = 365;

/// Issue #1519: Minimum per-issuer / per-subject issuance limit.
/// A limit of `0` would silently block all issuance.
pub const MIN_LIMITS: u32 = 1;

/// Issue #1519: Maximum per-issuer / per-subject issuance limit.
pub const MAX_LIMITS: u32 = 1_000_000;

/// Issue #1519: Maximum rate limit (issuances per window).
/// Prevents values such as `u64::MAX` from halting an issuer forever.
pub const MAX_RATE_LIMIT: u64 = 1_000_000;

/// Issue #1519: Maximum decay half-life in days.
pub const MAX_HALF_LIFE_DAYS: u32 = 3_650;

/// Issue #1519: Maximum revocation weight, expressed as a percentage.
pub const MAX_REVOCATION_WEIGHT: u32 = 100;

/// Issue #1519: Maximum council timelock delay in seconds (30 days).
/// A huge delay would permanently block council actions.
pub const MAX_TIMELOCK_SECS: u64 = 30 * SECS_PER_DAY;

