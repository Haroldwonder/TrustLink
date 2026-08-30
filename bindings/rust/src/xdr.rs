//! Helpers for building and decoding Soroban `SCVal` XDR values.
//!
//! Soroban contract calls are encoded as XDR.  The helpers here produce the
//! wire representation expected by the RPC `simulateTransaction` endpoint and
//! decode the raw XDR returned in the `result.results[0].xdr` field back to
//! Rust values.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use stellar_xdr::curr::{
    AccountId, Hash, HostFunction, InvokeContractArgs, LimitedVec, PublicKey, ScAddress,
    ScString, ScSymbol, ScVal, ScVec, StringM, Uint256, WriteXdr,
};

use crate::types::{Attestation, AttestationStatus, TrustLinkError, Result};

// ─── Encoding helpers ─────────────────────────────────────────────────────────

/// Encode an `SCVal::Symbol` (used for method names and short string args).
pub fn scval_symbol(s: &str) -> ScVal {
    ScVal::Symbol(ScSymbol(
        StringM::try_from(s.as_bytes().to_vec()).expect("symbol fits"),
    ))
}

/// Encode an `SCVal::String` (used for longer string arguments).
pub fn scval_string(s: &str) -> ScVal {
    ScVal::String(ScString(
        StringM::try_from(s.as_bytes().to_vec()).expect("string fits"),
    ))
}

/// Encode an `SCVal::Address` for an account (G… Stellar public key).
pub fn scval_address(address: &str) -> Result<ScVal> {
    let keypair = stellar_strkey::ed25519::PublicKey::from_string(address)
        .map_err(|e| TrustLinkError::Conversion(format!("invalid address {address}: {e}")))?;
    let pk = PublicKey::PublicKeyTypeEd25519(Uint256(keypair.0));
    let account_id = AccountId(pk);
    Ok(ScVal::Address(ScAddress::Account(account_id)))
}

/// Encode an `SCVal::U32`.
pub fn scval_u32(n: u32) -> ScVal {
    ScVal::U32(n)
}

/// Encode `Some(u64)` as `SCVal::Vec([U64])`, `None` as `SCVal::Vec([])`.
pub fn scval_option_u64(opt: Option<u64>) -> ScVal {
    match opt {
        None => ScVal::Vec(Some(ScVec(LimitedVec::new()))),
        Some(n) => {
            let mut v = LimitedVec::new();
            v.push(ScVal::U64(n)).expect("vec capacity");
            ScVal::Vec(Some(ScVec(v)))
        }
    }
}

/// Encode a `Vec<String>` as `SCVal::Vec([Symbol, …])`.
pub fn scval_vec_symbol(items: &[&str]) -> ScVal {
    let mut v = LimitedVec::new();
    for s in items {
        v.push(scval_symbol(s)).expect("vec capacity");
    }
    ScVal::Vec(Some(ScVec(v)))
}

/// Build the full `HostFunction` for a read-only Soroban invocation.
///
/// `contract_id_hex` must be the 32-byte contract address in hex (the raw
/// bytes, not a strkey C… address).
pub fn build_invoke_function(
    contract_id_hex: &str,
    function_name: &str,
    args: Vec<ScVal>,
) -> Result<HostFunction> {
    let id_bytes =
        hex::decode(contract_id_hex).map_err(|e| TrustLinkError::Xdr(e.to_string()))?;
    if id_bytes.len() != 32 {
        return Err(TrustLinkError::Xdr(format!(
            "contract id must be 32 bytes, got {}",
            id_bytes.len()
        )));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&id_bytes);

    let mut args_limited = LimitedVec::new();
    for arg in args {
        args_limited.push(arg).expect("arg capacity");
    }

    Ok(HostFunction::InvokeContract(InvokeContractArgs {
        contract_address: ScAddress::Contract(Hash(arr)),
        function_name: ScSymbol(
            StringM::try_from(function_name.as_bytes().to_vec()).expect("fn name fits"),
        ),
        args: args_limited,
    }))
}

// ─── Encoding a HostFunction to base64 XDR ────────────────────────────────────

/// Serialize a `HostFunction` to base64-encoded XDR suitable for inclusion in
/// the `simulateTransaction` JSON-RPC request body.
pub fn host_function_to_xdr_base64(hf: &HostFunction) -> Result<String> {
    let mut buf = Vec::new();
    hf.write_xdr(&mut buf)
        .map_err(|e| TrustLinkError::Xdr(e.to_string()))?;
    Ok(BASE64.encode(&buf))
}

// ─── Decoding SCVal from base64 XDR ──────────────────────────────────────────

use stellar_xdr::curr::ReadXdr;

/// Decode the base64-XDR `retval` string returned by `simulateTransaction` to
/// a typed `ScVal`.
pub fn decode_scval(b64: &str) -> Result<ScVal> {
    let bytes = BASE64
        .decode(b64)
        .map_err(|e| TrustLinkError::Xdr(format!("base64 decode: {e}")))?;
    ScVal::from_xdr(bytes, stellar_xdr::curr::Limits::none())
        .map_err(|e| TrustLinkError::Xdr(format!("xdr decode: {e}")))
}

// ─── ScVal → Rust type conversions ────────────────────────────────────────────

/// Extract a `bool` from an `ScVal::Bool`.
pub fn scval_to_bool(val: &ScVal) -> Result<bool> {
    match val {
        ScVal::Bool(b) => Ok(*b),
        other => Err(TrustLinkError::Conversion(format!(
            "expected Bool, got {other:?}"
        ))),
    }
}

/// Extract a `String` from `ScVal::String` or `ScVal::Symbol`.
pub fn scval_to_string(val: &ScVal) -> Result<String> {
    match val {
        ScVal::String(s) => Ok(String::from_utf8_lossy(s.as_slice()).into_owned()),
        ScVal::Symbol(s) => Ok(String::from_utf8_lossy(s.as_slice()).into_owned()),
        other => Err(TrustLinkError::Conversion(format!(
            "expected String/Symbol, got {other:?}"
        ))),
    }
}

/// Extract a `u64` from `ScVal::U64`.
pub fn scval_to_u64(val: &ScVal) -> Result<u64> {
    match val {
        ScVal::U64(n) => Ok(*n),
        other => Err(TrustLinkError::Conversion(format!(
            "expected U64, got {other:?}"
        ))),
    }
}

/// Extract `Option<u64>` from `ScVal::Vec([U64])` or `ScVal::Vec([])`.
pub fn scval_to_option_u64(val: &ScVal) -> Result<Option<u64>> {
    match val {
        ScVal::Vec(None) => Ok(None),
        ScVal::Vec(Some(v)) if v.is_empty() => Ok(None),
        ScVal::Vec(Some(v)) => {
            let inner = v.first().ok_or_else(|| {
                TrustLinkError::Conversion("empty option vec".into())
            })?;
            Ok(Some(scval_to_u64(inner)?))
        }
        other => Err(TrustLinkError::Conversion(format!(
            "expected Vec for Option<u64>, got {other:?}"
        ))),
    }
}

/// Extract `Option<String>` from `ScVal::Vec([String])` or `ScVal::Vec([])`.
pub fn scval_to_option_string(val: &ScVal) -> Result<Option<String>> {
    match val {
        ScVal::Vec(None) => Ok(None),
        ScVal::Vec(Some(v)) if v.is_empty() => Ok(None),
        ScVal::Vec(Some(v)) => {
            let inner = v.first().ok_or_else(|| {
                TrustLinkError::Conversion("empty option vec".into())
            })?;
            Ok(Some(scval_to_string(inner)?))
        }
        other => Err(TrustLinkError::Conversion(format!(
            "expected Vec for Option<String>, got {other:?}"
        ))),
    }
}

/// Extract a Stellar account address (`G…`) from `ScVal::Address`.
pub fn scval_to_address(val: &ScVal) -> Result<String> {
    match val {
        ScVal::Address(ScAddress::Account(AccountId(
            PublicKey::PublicKeyTypeEd25519(Uint256(bytes)),
        ))) => {
            let strkey = stellar_strkey::ed25519::PublicKey(*bytes);
            Ok(strkey.to_string())
        }
        other => Err(TrustLinkError::Conversion(format!(
            "expected Address(Account), got {other:?}"
        ))),
    }
}

// ─── Attestation decoding ─────────────────────────────────────────────────────

/// Decode a `ScVal::Map` returned by `get_attestation` into an [`Attestation`].
pub fn scval_to_attestation(val: &ScVal) -> Result<Attestation> {
    let map = match val {
        ScVal::Map(Some(m)) => m,
        other => {
            return Err(TrustLinkError::Conversion(format!(
                "expected Map for Attestation, got {other:?}"
            )))
        }
    };

    let mut fields: std::collections::HashMap<String, &ScVal> = std::collections::HashMap::new();
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

    Ok(Attestation {
        id: scval_to_string(get("id")?)?,
        issuer: scval_to_address(get("issuer")?)?,
        subject: scval_to_address(get("subject")?)?,
        claim_type: scval_to_string(get("claim_type")?)?,
        timestamp: scval_to_u64(get("timestamp")?)?,
        expiration: scval_to_option_u64(get("expiration")?)?,
        revoked: scval_to_bool(get("revoked")?)?,
        metadata: scval_to_option_string(get("metadata")?)?,
        imported: scval_to_bool(get("imported")?)?,
        bridged: scval_to_bool(get("bridged")?)?,
        source_chain: scval_to_option_string(get("source_chain")?)?,
        source_tx: scval_to_option_string(get("source_tx")?)?,
    })
}

/// Decode a `ScVal::Vec` of attestation maps into a `Vec<Attestation>`.
pub fn scval_to_attestation_vec(val: &ScVal) -> Result<Vec<Attestation>> {
    let vec = match val {
        ScVal::Vec(Some(v)) => v,
        ScVal::Vec(None) => return Ok(vec![]),
        other => {
            return Err(TrustLinkError::Conversion(format!(
                "expected Vec for attestation list, got {other:?}"
            )))
        }
    };

    vec.iter().map(scval_to_attestation).collect()
}

/// Decode a `ScVal` representing `AttestationStatus`.
pub fn scval_to_status(val: &ScVal) -> Result<AttestationStatus> {
    let s = scval_to_string(val)?;
    match s.as_str() {
        "Valid" => Ok(AttestationStatus::Valid),
        "Expired" => Ok(AttestationStatus::Expired),
        "Revoked" => Ok(AttestationStatus::Revoked),
        other => Err(TrustLinkError::Conversion(format!(
            "unknown AttestationStatus: {other}"
        ))),
    }
}
