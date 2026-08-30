//! High-level [`TrustLinkClient`] — the primary entry point for Rust
//! applications that want to query TrustLink over the Soroban RPC.
//!
//! # Quick start
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
//!     let has_kyc = client
//!         .has_valid_claim("GABC...SUBJECT", "KYC_PASSED")
//!         .await?;
//!
//!     println!("Has valid KYC: {has_kyc}");
//!     Ok(())
//! }
//! ```

use stellar_strkey::contract::Contract as ContractStrkey;

use crate::rpc::RpcClient;
use crate::types::{Attestation, AttestationStatus, GlobalStats, Result, TrustLinkError};
use crate::xdr::{
    scval_address, scval_symbol, scval_to_attestation, scval_to_attestation_vec, scval_to_bool,
    scval_to_status, scval_u32, scval_vec_symbol,
};

// ─── Well-known network passphrases ───────────────────────────────────────────

/// Common Stellar network configurations.
pub struct Networks;

impl Networks {
    /// Stellar public testnet.
    pub const TESTNET: &'static str = "https://soroban-testnet.stellar.org";
    /// Stellar mainnet (Pubnet).
    pub const MAINNET: &'static str = "https://mainnet.stellar.validationcloud.io/v1/XDM6i7eJ44LWFRVOwGfMqHoT0r9M";
    /// Local node (Quickstart / standalone).
    pub const LOCAL: &'static str = "http://localhost:8000/soroban/rpc";
}

// ─── TrustLinkClient ──────────────────────────────────────────────────────────

/// Read-only Rust client for the TrustLink contract.
///
/// All methods perform a simulated transaction (no fees, no signing) over the
/// Soroban JSON-RPC protocol.
#[derive(Debug, Clone)]
pub struct TrustLinkClient {
    /// 32-byte contract ID as hex (derived from the C… strkey on construction).
    contract_id_hex: String,
    rpc: RpcClient,
}

impl TrustLinkClient {
    /// Create a new client.
    ///
    /// # Parameters
    ///
    /// - `contract_id` — The deployed TrustLink contract address (C… strkey).
    /// - `rpc_url` — Soroban RPC endpoint, e.g. [`Networks::TESTNET`].
    ///
    /// # Errors
    ///
    /// Returns [`TrustLinkError::Conversion`] if `contract_id` is not a valid
    /// Stellar contract strkey.
    pub fn new(contract_id: &str, rpc_url: &str) -> Result<Self> {
        let strkey = ContractStrkey::from_string(contract_id).map_err(|e| {
            TrustLinkError::Conversion(format!("invalid contract id '{contract_id}': {e}"))
        })?;
        let contract_id_hex = hex::encode(strkey.0);
        Ok(Self {
            contract_id_hex,
            rpc: RpcClient::new(rpc_url),
        })
    }

    // ─── Claim verification ────────────────────────────────────────────────────

    /// Return `true` if `subject` currently holds a valid (non-revoked,
    /// non-expired) attestation of type `claim_type`.
    ///
    /// Uses OR-logic across all issuers — returns `true` on the first valid
    /// match.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use trustlink_client::{TrustLinkClient, Networks};
    /// # #[tokio::main] async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = TrustLinkClient::new("C...", Networks::TESTNET)?;
    /// let ok = client.has_valid_claim("GSUBJECT...", "KYC_PASSED").await?;
    /// # Ok(()) }
    /// ```
    pub async fn has_valid_claim(&self, subject: &str, claim_type: &str) -> Result<bool> {
        let args = vec![scval_address(subject)?, scval_symbol(claim_type)];
        let val = self
            .rpc
            .simulate(&self.contract_id_hex, "has_valid_claim", args)
            .await?;
        scval_to_bool(&val)
    }

    /// Return `true` if `subject` holds a valid attestation of `claim_type`
    /// issued specifically by `issuer`.
    pub async fn has_valid_claim_from_issuer(
        &self,
        subject: &str,
        claim_type: &str,
        issuer: &str,
    ) -> Result<bool> {
        let args = vec![
            scval_address(subject)?,
            scval_symbol(claim_type),
            scval_address(issuer)?,
        ];
        let val = self
            .rpc
            .simulate(
                &self.contract_id_hex,
                "has_valid_claim_from_issuer",
                args,
            )
            .await?;
        scval_to_bool(&val)
    }

    /// Return `true` if `subject` holds a valid attestation for **any** of the
    /// listed claim types (OR-logic, short-circuits on first match).
    ///
    /// An empty `claim_types` slice always returns `false`.
    pub async fn has_any_claim(&self, subject: &str, claim_types: &[&str]) -> Result<bool> {
        let args = vec![scval_address(subject)?, scval_vec_symbol(claim_types)];
        let val = self
            .rpc
            .simulate(&self.contract_id_hex, "has_any_claim", args)
            .await?;
        scval_to_bool(&val)
    }

    /// Return `true` if `subject` holds a valid attestation for **all** of the
    /// listed claim types (AND-logic, short-circuits on first missing claim).
    ///
    /// An empty `claim_types` slice always returns `true`.
    pub async fn has_all_claims(&self, subject: &str, claim_types: &[&str]) -> Result<bool> {
        let args = vec![scval_address(subject)?, scval_vec_symbol(claim_types)];
        let val = self
            .rpc
            .simulate(&self.contract_id_hex, "has_all_claims", args)
            .await?;
        scval_to_bool(&val)
    }

    // ─── Attestation queries ───────────────────────────────────────────────────

    /// Fetch a single attestation by its deterministic ID.
    ///
    /// # Errors
    ///
    /// Returns [`TrustLinkError::Contract`] with code
    /// [`ContractErrorCode::NotFound`] if the attestation does not exist.
    pub async fn get_attestation(&self, attestation_id: &str) -> Result<Attestation> {
        let args = vec![scval_symbol(attestation_id)];
        let val = self
            .rpc
            .simulate(&self.contract_id_hex, "get_attestation", args)
            .await?;
        scval_to_attestation(&val)
    }

    /// Return the current [`AttestationStatus`] (`Valid`, `Expired`, or
    /// `Revoked`) for the given attestation ID.
    pub async fn get_attestation_status(
        &self,
        attestation_id: &str,
    ) -> Result<AttestationStatus> {
        let args = vec![scval_symbol(attestation_id)];
        let val = self
            .rpc
            .simulate(&self.contract_id_hex, "get_attestation_status", args)
            .await?;
        scval_to_status(&val)
    }

    /// Return a paginated list of attestations held by `subject`.
    ///
    /// - `offset` — zero-based start index.
    /// - `limit` — maximum number of results to return.
    pub async fn get_subject_attestations(
        &self,
        subject: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Attestation>> {
        let args = vec![
            scval_address(subject)?,
            scval_u32(offset),
            scval_u32(limit),
        ];
        let val = self
            .rpc
            .simulate(&self.contract_id_hex, "get_subject_attestations", args)
            .await?;
        scval_to_attestation_vec(&val)
    }

    /// Return a paginated list of attestations issued by `issuer`.
    pub async fn get_issuer_attestations(
        &self,
        issuer: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Attestation>> {
        let args = vec![
            scval_address(issuer)?,
            scval_u32(offset),
            scval_u32(limit),
        ];
        let val = self
            .rpc
            .simulate(&self.contract_id_hex, "get_issuer_attestations", args)
            .await?;
        scval_to_attestation_vec(&val)
    }

    /// Check whether `address` is a currently registered issuer.
    pub async fn is_issuer(&self, address: &str) -> Result<bool> {
        let args = vec![scval_address(address)?];
        let val = self
            .rpc
            .simulate(&self.contract_id_hex, "is_issuer", args)
            .await?;
        scval_to_bool(&val)
    }

    /// Fetch contract-wide counters (total attestations, revocations, and
    /// registered issuers).
    pub async fn get_global_stats(&self) -> Result<GlobalStats> {
        use crate::xdr::{scval_to_string, scval_to_u64};
        use stellar_xdr::curr::ScVal;

        let val = self
            .rpc
            .simulate(&self.contract_id_hex, "get_global_stats", vec![])
            .await?;

        // The contract returns a Map with keys total_attestations,
        // total_revocations, total_issuers.
        let map = match &val {
            ScVal::Map(Some(m)) => m,
            other => {
                return Err(TrustLinkError::Conversion(format!(
                    "expected Map for GlobalStats, got {other:?}"
                )))
            }
        };

        let mut fields: std::collections::HashMap<String, &ScVal> =
            std::collections::HashMap::new();
        for entry in map.iter() {
            let key = scval_to_string(&entry.key)?;
            fields.insert(key, &entry.val);
        }

        let get = |name: &str| -> Result<&ScVal> {
            fields
                .get(name)
                .copied()
                .ok_or_else(|| TrustLinkError::MissingField(name.to_owned()))
        };

        Ok(GlobalStats {
            total_attestations: scval_to_u64(get("total_attestations")?)?,
            total_revocations: scval_to_u64(get("total_revocations")?)?,
            total_issuers: scval_to_u64(get("total_issuers")?)?,
        })
    }
}
