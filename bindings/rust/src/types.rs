//! TrustLink type definitions mirroring the on-chain contract types.
//!
//! All types are serializable via `serde` and can be used directly from the
//! values returned by the Soroban RPC simulation endpoint.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Attestation ──────────────────────────────────────────────────────────────

/// A single attestation record as stored on-chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attestation {
    /// Deterministic hash-based ID.
    pub id: String,
    /// Address of the issuer that created this attestation.
    pub issuer: String,
    /// Address of the subject this attestation describes.
    pub subject: String,
    /// Claim type identifier (e.g. `"KYC_PASSED"`).
    pub claim_type: String,
    /// Unix timestamp (seconds) when the attestation was created.
    pub timestamp: u64,
    /// Optional expiration unix timestamp.
    pub expiration: Option<u64>,
    /// Whether the attestation has been revoked.
    pub revoked: bool,
    /// Optional issuer-supplied metadata string.
    pub metadata: Option<String>,
    /// `true` when migrated from an external source via `import_attestation`.
    pub imported: bool,
    /// `true` when created by a trusted bridge contract.
    pub bridged: bool,
    /// Source chain identifier for bridged attestations.
    pub source_chain: Option<String>,
    /// Source transaction reference for bridged attestations.
    pub source_tx: Option<String>,
}

// ─── AttestationStatus ────────────────────────────────────────────────────────

/// The current lifecycle status of an attestation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AttestationStatus {
    /// The attestation is valid and not expired.
    Valid,
    /// The attestation has passed its expiration timestamp.
    Expired,
    /// The attestation has been revoked by the issuer.
    Revoked,
}

// ─── ClaimTypeInfo ────────────────────────────────────────────────────────────

/// Metadata for a registered claim type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimTypeInfo {
    /// The claim type identifier (e.g. `"KYC_PASSED"`).
    pub claim_type: String,
    /// Human-readable description of what this claim type signifies.
    pub description: String,
}

// ─── GlobalStats ─────────────────────────────────────────────────────────────

/// Contract-wide counters returned by `get_global_stats`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalStats {
    /// Cumulative count of all attestations ever created.
    pub total_attestations: u64,
    /// Cumulative count of all revocations ever performed.
    pub total_revocations: u64,
    /// Current number of registered issuers (live count, not cumulative).
    pub total_issuers: u64,
}

// ─── IssuerStats ─────────────────────────────────────────────────────────────

/// Per-issuer statistics returned by the contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IssuerStats {
    /// Total number of attestations ever issued by this address.
    pub total_issued: u64,
}

// ─── MultiSigProposal ─────────────────────────────────────────────────────────

/// A pending or finalized multi-sig attestation proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiSigProposal {
    /// Unique proposal identifier.
    pub id: String,
    /// Addresses of all required signers.
    pub required_signers: Vec<String>,
    /// Addresses that have already signed.
    pub signers: Vec<String>,
    /// Number of signatures needed to activate the attestation.
    pub threshold: u32,
    /// `true` once the threshold is reached and the attestation is active.
    pub finalized: bool,
    /// Unix timestamp after which new co-signs are rejected.
    pub expires_at: u64,
}

// ─── Errors ───────────────────────────────────────────────────────────────────

// `ContractErrorCode` is generated from `src/errors.rs` so the binding cannot
// drift from the contract's error table. Re-exported here to keep
// `trustlink::types::ContractErrorCode` working for existing callers.
pub use crate::generated::error_codes::ContractErrorCode;

/// Client-level error wrapping both transport and contract errors.
#[derive(Debug, thiserror::Error)]
pub enum TrustLinkError {
    /// HTTP or JSON-RPC transport error.
    #[error("RPC transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// The RPC server returned an error response.
    #[error("RPC error: {0}")]
    Rpc(String),

    /// The contract returned a typed error code.
    #[error("Contract error #{code:?}: {message}")]
    Contract {
        code: ContractErrorCode,
        message: String,
    },

    /// An XDR encoding or decoding failure.
    #[error("XDR codec error: {0}")]
    Xdr(String),

    /// A required field was missing in the response.
    #[error("Missing field in response: {0}")]
    MissingField(String),

    /// The returned SCVal could not be decoded into the expected Rust type.
    #[error("Type conversion error: {0}")]
    Conversion(String),
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, TrustLinkError>;

// ─── RPC response shapes (internal) ──────────────────────────────────────────

/// Wraps the raw JSON returned by `simulateTransaction`.
#[derive(Debug, Deserialize)]
pub(crate) struct SimulateResponse {
    pub id: Option<serde_json::Value>,
    pub result: Option<SimulateResult>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SimulateResult {
    pub results: Option<Vec<SimulateResultEntry>>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SimulateResultEntry {
    pub xdr: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
