//! Low-level JSON-RPC transport for the Soroban RPC `simulateTransaction`
//! endpoint.
//!
//! All reads against TrustLink are performed as simulated (read-only)
//! transactions — no ledger state is modified and no signing is required.

use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use stellar_xdr::curr::{HostFunction, InvokeContractArgs, Limits, ReadXdr, ScVal, WriteXdr};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use crate::types::{
    RpcError, SimulateResponse, TrustLinkError, Result,
};
use crate::xdr::{build_invoke_function, decode_scval};

// ─── RPC client ───────────────────────────────────────────────────────────────

/// Thin wrapper around an HTTP client pointing at a Soroban RPC server.
#[derive(Debug, Clone)]
pub(crate) struct RpcClient {
    http: HttpClient,
    rpc_url: String,
}

impl RpcClient {
    /// Create a new `RpcClient` targeting `rpc_url`.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            http: HttpClient::new(),
            rpc_url: rpc_url.to_owned(),
        }
    }

    /// Invoke a contract function in simulation mode and return the decoded
    /// `ScVal` result.
    ///
    /// `contract_id_hex` is the 32-byte contract address as hex.
    pub async fn simulate(
        &self,
        contract_id_hex: &str,
        function_name: &str,
        args: Vec<ScVal>,
    ) -> Result<ScVal> {
        let hf = build_invoke_function(contract_id_hex, function_name, args)?;

        // Serialize the HostFunction to base64 XDR
        let mut hf_xdr = Vec::new();
        hf.write_xdr(&mut hf_xdr)
            .map_err(|e| TrustLinkError::Xdr(e.to_string()))?;
        let hf_b64 = BASE64.encode(&hf_xdr);

        // Build a minimal Transaction envelope that wraps just the InvokeHostFunction op.
        // Soroban RPC `simulateTransaction` accepts the full envelope XDR.
        // We use a pre-built minimal envelope template (no signatures required for reads).
        let tx_b64 = self.build_tx_envelope(hf_b64)?;

        // Call simulateTransaction
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "simulateTransaction",
            "params": {
                "transaction": tx_b64
            }
        });

        let resp = self
            .http
            .post(&self.rpc_url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(TrustLinkError::Transport)?;

        let sim: SimulateResponse = resp
            .json()
            .await
            .map_err(TrustLinkError::Transport)?;

        // Check for JSON-RPC error
        if let Some(err) = sim.error {
            return Err(TrustLinkError::Rpc(format!(
                "code={}, message={}",
                err.code, err.message
            )));
        }

        let result = sim
            .result
            .ok_or_else(|| TrustLinkError::MissingField("result".into()))?;

        // Check for simulation-level error (contract trap)
        if let Some(err_str) = result.error {
            // Try to parse a contract error code from the error string
            return Err(parse_contract_error(&err_str));
        }

        let entries = result
            .results
            .ok_or_else(|| TrustLinkError::MissingField("result.results".into()))?;

        let first = entries
            .into_iter()
            .next()
            .ok_or_else(|| TrustLinkError::MissingField("result.results[0]".into()))?;

        decode_scval(&first.xdr)
    }

    // ─── Minimal transaction envelope builder ─────────────────────────────────

    /// Build a base64-encoded `TransactionEnvelope` with a single
    /// `InvokeHostFunction` operation suitable for simulation.
    ///
    /// No source account fees or signatures are required for `simulateTransaction`.
    fn build_tx_envelope(&self, _hf_b64: String) -> Result<String> {
        // Soroban RPC accepts a full TransactionEnvelope XDR.
        // We construct one with a zero fee and a dummy account, which is valid
        // for read-only simulation.
        use stellar_xdr::curr::{
            AccountId, DecoratedSignature, EnvelopeType, FeeBumpTransaction,
            FeeBumpTransactionEnvelope, FeeBumpTransactionInnerTx, Hash, InvokeHostFunctionOp,
            Memo, MuxedAccount, Operation, OperationBody, Preconditions, PublicKey,
            SequenceNumber, Transaction, TransactionEnvelope, TransactionExt,
            TransactionV1Envelope, Uint256,
        };

        // Dummy source account (all zeros — valid for simulation)
        let src = MuxedAccount::Ed25519(Uint256([0u8; 32]));

        // Re-decode the HostFunction from b64
        let hf_bytes = BASE64
            .decode(&_hf_b64)
            .map_err(|e| TrustLinkError::Xdr(e.to_string()))?;
        let hf = HostFunction::from_xdr(hf_bytes, Limits::none())
            .map_err(|e| TrustLinkError::Xdr(e.to_string()))?;

        let op = Operation {
            source_account: None,
            body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
                host_function: hf,
                auth: stellar_xdr::curr::VecM::default(),
            }),
        };

        let mut ops = stellar_xdr::curr::VecM::default();
        ops.push(op).ok();

        let tx = Transaction {
            source_account: src,
            fee: 0,
            seq_num: SequenceNumber(0),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: ops,
            ext: TransactionExt::V0,
        };

        let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
            tx,
            signatures: stellar_xdr::curr::VecM::default(),
        });

        let mut buf = Vec::new();
        envelope
            .write_xdr(&mut buf)
            .map_err(|e| TrustLinkError::Xdr(e.to_string()))?;
        Ok(BASE64.encode(&buf))
    }
}

// ─── Error parsing ─────────────────────────────────────────────────────────────

/// Parse a Soroban simulation error string into a [`TrustLinkError`].
///
/// Contract panics surface as strings like `"Error(Contract, #4)"`.
fn parse_contract_error(msg: &str) -> TrustLinkError {
    use crate::types::ContractErrorCode;
    // Match patterns like "Error(Contract, #4)"
    if let Some(code_str) = msg
        .split('#')
        .nth(1)
        .and_then(|s| s.split(')').next())
        .and_then(|s| s.trim().parse::<u32>().ok())
    {
        let code = ContractErrorCode::from(code_str);
        TrustLinkError::Contract {
            code,
            message: msg.to_owned(),
        }
    } else {
        TrustLinkError::Rpc(msg.to_owned())
    }
}
