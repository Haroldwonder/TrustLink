//! # trustlink-client
//!
//! A native Rust RPC client for the [TrustLink](https://github.com/afurious/TrustLink)
//! on-chain attestation contract on Stellar/Soroban.
//!
//! This crate is **distinct** from the on-chain contract crate — it is a
//! thin async HTTP client that talks to a Soroban RPC node and lets Rust
//! backend services, CLIs, and indexers query TrustLink without needing a
//! TypeScript or Python runtime.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use trustlink_client::{TrustLinkClient, Networks};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = TrustLinkClient::new(
//!         "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8",
//!         Networks::TESTNET,
//!     )?;
//!
//!     // Check a wallet's KYC status
//!     let has_kyc = client
//!         .has_valid_claim("GABC...SUBJECT_ADDRESS", "KYC_PASSED")
//!         .await?;
//!     println!("Has valid KYC: {has_kyc}");
//!
//!     // Fetch a full attestation record
//!     let att = client.get_attestation("att_id_here").await?;
//!     println!("Claim type: {}", att.claim_type);
//!
//!     // List the first 20 attestations for a subject
//!     let page = client
//!         .get_subject_attestations("GABC...SUBJECT_ADDRESS", 0, 20)
//!         .await?;
//!     println!("Found {} attestations", page.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Method coverage
//!
//! | Method | Contract function |
//! |---|---|
//! | [`TrustLinkClient::has_valid_claim`] | `has_valid_claim` |
//! | [`TrustLinkClient::has_valid_claim_from_issuer`] | `has_valid_claim_from_issuer` |
//! | [`TrustLinkClient::has_any_claim`] | `has_any_claim` |
//! | [`TrustLinkClient::has_all_claims`] | `has_all_claims` |
//! | [`TrustLinkClient::get_attestation`] | `get_attestation` |
//! | [`TrustLinkClient::get_attestation_status`] | `get_attestation_status` |
//! | [`TrustLinkClient::get_subject_attestations`] | `get_subject_attestations` |
//! | [`TrustLinkClient::get_issuer_attestations`] | `get_issuer_attestations` |
//! | [`TrustLinkClient::is_issuer`] | `is_issuer` |
//! | [`TrustLinkClient::get_global_stats`] | `get_global_stats` |
//!
//! ## Error handling
//!
//! All methods return `Result<T, `[`TrustLinkError`]`>`.
//! Contract-level traps (e.g. `NotFound`, `Unauthorized`) surface as
//! [`TrustLinkError::Contract`] with a typed [`ContractErrorCode`].

pub mod client;
pub mod rpc;
pub mod types;
pub mod xdr;

pub use client::{Networks, TrustLinkClient};
pub use types::{
    Attestation, AttestationStatus, ClaimTypeInfo, ContractErrorCode, GlobalStats, IssuerStats,
    MultiSigProposal, TrustLinkError,
};
