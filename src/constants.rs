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

