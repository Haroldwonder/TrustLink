use soroban_sdk::{token::TokenClient, Address, Env, String, Vec};

use crate::constants::SECS_PER_DAY;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    Attestation, AttestationOrigin, AttestationStatus, AttestationVersionSnapshot, AuditAction,
    AuditEntry, Endorsement, Error, FeeConfig,
};
use crate::validation::Validation;

pub const MAX_SOURCE_CHAIN_LEN: u32 = 32;
pub const MAX_SOURCE_TX_LEN: u32 = 128;

/// Convert a `u64` to a Soroban `String` without `std` (no `format!`/`to_string`).
pub fn u64_to_string(env: &Env, n: u64) -> String {
    if n == 0 {
        return String::from_bytes(env, b"0");
    }
    let mut buf = [0u8; 20]; // u64::MAX has 20 decimal digits
    let mut pos = 20usize;
    let mut val = n;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    String::from_bytes(env, &buf[pos..20])
}

// -----------------------------------------------------------------------
// Shared helpers (pub so admin.rs / multisig.rs / request.rs can reuse)
// -----------------------------------------------------------------------

pub fn validate_native_expiration(env: &Env, expiration: Option<u64>) -> Result<(), Error> {
    if let Some(v) = expiration {
        if v <= env.ledger().timestamp() {
            return Err(Error::InvalidExpiration);
        }
    }
    Ok(())
}

pub fn validate_valid_from(env: &Env, valid_from: Option<u64>) -> Result<(), Error> {
    if let Some(vf) = valid_from {
        if vf <= env.ledger().timestamp() {
            return Err(Error::InvalidValidFrom);
        }
    }
    Ok(())
}

pub fn validate_import_timestamps(env: &Env, timestamp: u64, expiration: Option<u64>) -> Result<(), Error> {
    if timestamp > env.ledger().timestamp() {
        return Err(Error::InvalidTimestamp);
    }
    if let Some(v) = expiration {
        if v <= timestamp {
            return Err(Error::InvalidExpiration);
        }
    }
    Ok(())
}

pub fn validate_reason(reason: &Option<String>) -> Result<(), Error> {
    if let Some(r) = reason {
        if r.len() > 128 {
            return Err(Error::ReasonTooLong);
        }
    }
    Ok(())
}

pub fn validate_source_reference(source_chain: &String, source_tx: &String) -> Result<(), Error> {
    if source_chain.is_empty() || source_tx.is_empty() {
        return Err(Error::InvalidSourceReference);
    }
    if source_chain.len() > MAX_SOURCE_CHAIN_LEN || source_tx.len() > MAX_SOURCE_TX_LEN {
        return Err(Error::MetadataTooLong);
    }
    Ok(())
}

pub fn validate_tags(tags: &Option<Vec<String>>) -> Result<(), Error> {
    if let Some(t) = tags {
        if t.len() > 5 {
            return Err(Error::TooManyTags);
        }
        for tag in t.iter() {
            if tag.len() > 32 {
                return Err(Error::TagTooLong);
            }
        }
    }
    Ok(())
}

pub fn validate_jurisdiction(env: &Env, jurisdiction: &Option<String>) -> Result<(), Error> {
    if let Some(code) = jurisdiction {
        // Must be exactly 2 characters
        if code.len() != 2 {
            return Err(Error::InvalidJurisdiction);
        }

        
        // Must be exactly 2 uppercase ASCII letters (A-Z)
        let mut buf = [0u8; 2];
        code.copy_into_slice(&mut buf);
        if !buf.iter().all(|&b| b >= b'A' && b <= b'Z') {
            return Err(Error::InvalidJurisdiction);
        }
        
        // Must be a valid ISO 3166-1 alpha-2 code
        let valid_codes = [
            "AF","AX","AL","DZ","AS","AD","AO","AI","AQ","AG","AR","AM","AW","AU","AT","AZ",
            "BS","BH","BD","BB","BY","BE","BZ","BJ","BM","BT","BO","BQ","BA","BW","BV","BR",
            "IO","BN","BG","BF","BI","CV","KH","CM","CA","KY","CF","TD","CL","CN","CX","CC",
            "CO","KM","CG","CD","CK","CR","CI","HR","CU","CW","CY","CZ","DK","DJ","DM","DO",
            "EC","EG","SV","GQ","ER","EE","SZ","ET","FK","FO","FJ","FI","FR","GF","PF","TF",
            "GA","GM","GE","DE","GH","GI","GR","GL","GD","GP","GU","GT","GG","GN","GW","GY",
            "HT","HM","VA","HN","HK","HU","IS","IN","ID","IR","IQ","IE","IM","IL","IT","JM",
            "JP","JE","JO","KZ","KE","KI","KP","KR","KW","KG","LA","LV","LB","LS","LR","LY",
            "LI","LT","LU","MO","MK","MG","MW","MY","MV","ML","MT","MH","MQ","MR","MU","YT",
            "MX","FM","MD","MC","MN","ME","MS","MA","MZ","MM","NA","NR","NP","NL","NC","NZ",
            "NI","NE","NG","NU","NF","MP","NO","OM","PK","PW","PS","PA","PG","PY","PE","PH",
            "PN","PL","PT","PR","QA","RE","RO","RU","RW","BL","SH","KN","LC","MF","PM","VC",
            "WS","SM","ST","SA","SN","RS","SC","SL","SG","SX","SK","SI","SB","SO","ZA","GS",
            "SS","ES","LK","SD","SR","SJ","SE","CH","SY","TW","TJ","TZ","TH","TL","TG","TK",
            "TO","TT","TN","TR","TM","TC","TV","UG","UA","AE","GB","US","UM","UY","UZ","VU",
            "VE","VN","VG","VI","WF","EH","YE","ZM","ZW",
        ];
        let mut valid = false;
        for iso in valid_codes.iter() {
            if code == &String::from_str(env, iso) {
                valid = true;
                break;
            }
        }
        if !valid {
            return Err(Error::InvalidJurisdiction);
        }
    }
    Ok(())
}

pub fn check_rate_limit(env: &Env, issuer: &Address, claim_type: &String) -> Result<(), Error> {
    let current_time = env.ledger().timestamp();

    // Always enforce the global rate limit when configured.  The per-claim-type
    // limit is checked *in addition* to — not instead of — the global limit.
    if let Some(config) = Storage::get_rate_limit_config(env) {
        let interval = config.min_issuance_interval;
        if interval > 0 {
            if let Some(last) = Storage::get_last_issuance_time(env, issuer) {
                if current_time.saturating_sub(last) < interval {
                    return Err(Error::RateLimited);
                }
            }
        }
    }

    // Additionally enforce any per-claim-type rate limit, independently of the
    // global limit.  Both must pass — the per-claim-type limit does not replace
    // the global one.
    if let Some(claim_interval) = Storage::get_claim_type_rate_limit(env, claim_type) {
        if claim_interval > 0 {
            if let Some(last) =
                Storage::get_last_issuance_time_by_claim_type(env, issuer, claim_type)
            {
                if current_time.saturating_sub(last) < claim_interval {
                    return Err(Error::RateLimited);
                }
            }
        }
    }

    Ok(())
}

pub fn load_fee_config(env: &Env) -> Result<FeeConfig, Error> {
    Storage::get_fee_config(env).ok_or(Error::NotInitialized)
}

pub fn charge_attestation_fee(env: &Env, issuer: &Address) -> Result<(), Error> {
    charge_attestation_fees(env, issuer, 1)
}

/// Charge the fee for `count` attestations with a single token transfer.
///
/// # Errors
/// - [`Error::InvalidFee`] — `attestation_fee * count` overflows.
/// - [`Error::FeeTokenRequired`] — a fee is set but no fee token is configured.
pub fn charge_attestation_fees(env: &Env, issuer: &Address, count: u32) -> Result<(), Error> {
    let fee_config = load_fee_config(env)?;
    if fee_config.attestation_fee == 0 || count == 0 {
        return Ok(());
    }
    let total = fee_config
        .attestation_fee
        .checked_mul(count as i128)
        .ok_or(Error::InvalidFee)?;
    let fee_token = fee_config.fee_token.ok_or(Error::FeeTokenRequired)?;
    TokenClient::new(env, &fee_token).transfer(issuer, &fee_config.fee_collector, &total);
    Ok(())
}

pub fn store_attestation(env: &Env, attestation: &Attestation) {
    Storage::set_attestation(env, attestation);
    // Write the attestation ID to the two authoritative subject indexes:
    //   1. ChunkedIndex  — used for paginated listings
    //   2. ValidAttestations — pre-filtered index for has_valid_claim fast-path
    // The legacy flat SubjectAttestations key is NOT written here; it is only
    // kept as a read-path alias backed by ChunkedIndex (issue #1550 — three
    // parallel writes tripled write cost and created drift risk).
    Storage::add_valid_attestation(env, &attestation.subject, &attestation.id);
    crate::storage::ChunkedIndex::add_subject(env, &attestation.subject, &attestation.id);
    Storage::add_issuer_attestation(env, &attestation.issuer, &attestation.id);
    crate::storage::ChunkedIndex::add_issuer(env, &attestation.issuer, &attestation.id);
    let mut stats = Storage::get_issuer_stats(env, &attestation.issuer);
    stats.total_issued += 1;
    Storage::set_issuer_stats(env, &attestation.issuer, &stats);
    Storage::increment_total_attestations(env, 1);
}

pub fn maybe_trigger_expiration_hook(
    env: &Env,
    subject: &Address,
    attestation_id: &String,
    expiration: u64,
    current_time: u64,
) {
    let hook = match Storage::get_expiration_hook(env, subject) {
        Some(h) => h,
        None => return,
    };
    let notify_window = (hook.notify_days_before as u64) * SECS_PER_DAY;
    let notify_from = expiration.saturating_sub(notify_window);
    if current_time >= notify_from && current_time < expiration {
        Events::expiration_hook_triggered(env, subject, attestation_id, expiration);
        let client = crate::callback::ExpirationCallbackClient::new(env, &hook.callback_contract);
        let _ = client.try_notify_expiring(subject, attestation_id, &expiration);
    }
}

// -----------------------------------------------------------------------
// Attestation creation
// -----------------------------------------------------------------------

// -----------------------------------------------------------------------
// Shared validation for all attestation creation paths
// -----------------------------------------------------------------------

pub fn validate_new_attestation_common(
    env: &Env,
    issuer: &Address,
    subject: &Address,
    claim_type: &String,
) -> Result<(), Error> {
    Validation::validate_claim_type(claim_type)?;
    Validation::require_registered_claim_type(env, claim_type)?;
    
    if issuer == subject {
        return Err(Error::Unauthorized);
    }
    
    Ok(())
}

pub fn effective_max_attestations_per_subject(env: &Env) -> u32 {
    let limits = Storage::get_limits(env);
    Storage::get_contract_config(env)
        .and_then(|config| config.max_attestations_per_subject)
        .unwrap_or(limits.max_attestations_per_subject)
}

pub fn validate_attestation_metadata(
    env: &Env,
    claim_type: &String,
    metadata: &Option<String>,
) -> Result<(), Error> {
    Validation::validate_metadata(env, metadata)?;
    Validation::validate_claim_constraints(env, claim_type, metadata)?;
    Validation::validate_metadata_hash_only(env, metadata)
}

pub fn create_attestation_internal(
    env: &Env,
    issuer: Address,
    subject: Address,
    claim_type: String,
    expiration: Option<u64>,
    metadata: Option<String>,
    jurisdiction: Option<String>,
    tags: Option<Vec<String>>,
    valid_from: Option<u64>,
) -> Result<String, Error> {
    issuer.require_auth();
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &issuer)?;
    validate_new_attestation_common(env, &issuer, &subject, &claim_type)?;
    Validation::validate_metadata(env, &metadata)?;
    Validation::validate_claim_constraints(env, &claim_type, &metadata)?;
    Validation::validate_metadata_hash_only(env, &metadata)?;
    validate_jurisdiction(env, &jurisdiction)?;
    validate_tags(&tags)?;
    validate_native_expiration(env, expiration)?;
    validate_valid_from(env, valid_from)?;

    if Storage::is_whitelist_mode(env, &issuer) && !Storage::is_whitelisted(env, &issuer, &subject) {
        return Err(Error::SubjectNotWhitelisted);
    }

    check_rate_limit(env, &issuer, &claim_type)?;

    let limits = Storage::get_limits(env);
    // Use ChunkedIndex so revoked attestations no longer inflate the count.
    let issuer_count = crate::storage::ChunkedIndex::issuer_count(env, &issuer);
    if issuer_count >= limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }

    let subject_count = crate::storage::ChunkedIndex::subject_count(env, &subject);
    if subject_count >= effective_max_attestations_per_subject(env) {
        return Err(Error::LimitExceeded);
    }

    let timestamp = env.ledger().timestamp();
    let attestation_id = Attestation::generate_id(env, &issuer, &subject, &claim_type, timestamp);

    if Storage::has_attestation(env, &attestation_id) {
        return Err(Error::DuplicateAttestation);
    }

    let attestation = Attestation {
        id: attestation_id.clone(),
        issuer: issuer.clone(),
        subject,
        claim_type,
        timestamp,
        expiration,
        revoked: false,
        deleted: false,
        metadata,
        jurisdiction,
        valid_from,
        origin: AttestationOrigin::Native,
        source_chain: None,
        source_tx: None,
        tags,
        revocation_reason: None,
        bundle_id: None,
    };

    store_attestation(env, &attestation);
    Storage::append_audit_entry(
        env,
        &attestation_id,
        &AuditEntry {
            action: AuditAction::Created,
            actor: attestation.issuer.clone(),
            timestamp,
            details: None,
        },
    );
    Storage::set_last_issuance_time(env, &issuer, timestamp);
    if Storage::get_claim_type_rate_limit(env, &attestation.claim_type).is_some() {
        Storage::set_last_issuance_time_by_claim_type(env, &issuer, &attestation.claim_type, timestamp);
    }

    charge_attestation_fee(env, &issuer)?;

    Events::attestation_created(env, &attestation);
    // Keep the attestation and its indexes live until expiration so long-lived
    // attestations are not archived by the default TTL.
    Storage::extend_attestation_ttl_for_expiry(env, &attestation);
    Ok(attestation_id)
}

pub fn create_attestation(
    env: &Env,
    issuer: Address,
    subject: Address,
    claim_type: String,
    expiration: Option<u64>,
    metadata: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<String, Error> {
    create_attestation_internal(env, issuer, subject, claim_type, expiration, metadata, None, tags, None)
}

pub fn create_attestation_valid_from(
    env: &Env,
    issuer: Address,
    subject: Address,
    claim_type: String,
    expiration: Option<u64>,
    metadata: Option<String>,
    tags: Option<Vec<String>>,
    valid_from: u64,
) -> Result<String, Error> {
    create_attestation_internal(env, issuer, subject, claim_type, expiration, metadata, None, tags, Some(valid_from))
}

/// Same as [`create_attestation`], but demonstrates the version-guard
/// pattern from issue #952: if `expected_version` is `Some`, it must match
/// the contract's currently deployed version (as returned by `get_version()`)
/// or the call is rejected with `Error::VersionMismatch` before any other
/// validation runs. Passing `None` behaves exactly like `create_attestation`.
///
/// See [`crate::validation::Validation::require_version_match`] for the full
/// pattern documentation aimed at SDK authors.
pub fn create_attestation_versioned(
    env: &Env,
    issuer: Address,
    subject: Address,
    claim_type: String,
    expiration: Option<u64>,
    metadata: Option<String>,
    tags: Option<Vec<String>>,
    expected_version: Option<String>,
) -> Result<String, Error> {
    Validation::require_version_match(env, &expected_version)?;
    create_attestation_internal(env, issuer, subject, claim_type, expiration, metadata, None, tags, None)
}

pub fn create_attestation_jurisdiction(
    env: &Env,
    issuer: Address,
    subject: Address,
    claim_type: String,
    expiration: Option<u64>,
    metadata: Option<String>,
    jurisdiction: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<String, Error> {
    create_attestation_internal(env, issuer, subject, claim_type, expiration, metadata, jurisdiction, tags, None)
}

pub fn import_attestation(
    env: &Env,
    admin: Address,
    issuer: Address,
    subject: Address,
    claim_type: String,
    timestamp: u64,
    expiration: Option<u64>,
) -> Result<String, Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &issuer)?;
    validate_new_attestation_common(env, &issuer, &subject, &claim_type)?;
    validate_import_timestamps(env, timestamp, expiration)?;

    let limits = Storage::get_limits(env);
    let issuer_count = crate::storage::ChunkedIndex::issuer_count(env, &issuer);
    if issuer_count >= limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }

    // Check optional per-subject limit from ContractConfig if configured
    if let Some(config) = Storage::get_contract_config(env) {
        if let Some(max_per_subject) = config.max_attestations_per_subject {
            let subject_count = crate::storage::ChunkedIndex::subject_count(env, &subject);
            if subject_count >= max_per_subject {
                return Err(Error::LimitExceeded);
            }
        }
    } else {
        // Fallback to StorageLimits for backward compatibility if no config is set
        let subject_count = crate::storage::ChunkedIndex::subject_count(env, &subject);
        if subject_count >= limits.max_attestations_per_subject {
            return Err(Error::LimitExceeded);
        }
    }

    let attestation_id = Attestation::generate_id(env, &issuer, &subject, &claim_type, timestamp);
    if Storage::has_attestation(env, &attestation_id) {
        return Err(Error::DuplicateAttestation);
    }

    let attestation = Attestation {
        id: attestation_id.clone(),
        issuer,
        subject,
        claim_type,
        timestamp,
        expiration,
        revoked: false,
        deleted: false,
        metadata: None,
        jurisdiction: None,
        valid_from: None,
        origin: AttestationOrigin::Imported,
        source_chain: None,
        source_tx: None,
        tags: None,
        revocation_reason: None,
        bundle_id: None,
    };

    store_attestation(env, &attestation);
    Events::attestation_imported(env, &attestation);
    Storage::append_audit_entry(
        env,
        &attestation_id,
        &AuditEntry {
            action: AuditAction::Created,
            actor: admin.clone(),
            timestamp,
            details: None,
        },
    );
    // Keep the attestation and its indexes live until expiration.
    Storage::extend_attestation_ttl_for_expiry(env, &attestation);
    Ok(attestation_id)
}

pub fn bridge_attestation(
    env: &Env,
    bridge: Address,
    subject: Address,
    claim_type: String,
    source_chain: String,
    source_tx: String,
    expiration: Option<u64>,
) -> Result<String, Error> {
    bridge.require_auth();
    Validation::require_bridge(env, &bridge)?;
    Validation::require_not_paused(env)?;
    validate_new_attestation_common(env, &bridge, &subject, &claim_type)?;
    validate_source_reference(&source_chain, &source_tx)?;
    validate_native_expiration(env, expiration)?;

    // Check limits before creating attestation
    let limits = Storage::get_limits(env);
    let issuer_count = crate::storage::ChunkedIndex::issuer_count(env, &bridge);
    if issuer_count >= limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }
    
    // Check optional per-subject limit from ContractConfig if configured
    if let Some(config) = Storage::get_contract_config(env) {
        if let Some(max_per_subject) = config.max_attestations_per_subject {
            let subject_count = crate::storage::ChunkedIndex::subject_count(env, &subject);
            if subject_count >= max_per_subject {
                return Err(Error::LimitExceeded);
            }
        }
    } else {
        // Fallback to StorageLimits for backward compatibility if no config is set
        let subject_count = crate::storage::ChunkedIndex::subject_count(env, &subject);
        if subject_count >= limits.max_attestations_per_subject {
            return Err(Error::LimitExceeded);
        }
    }

    let timestamp = env.ledger().timestamp();
    let attestation_id = Attestation::generate_bridge_id(
        env, &bridge, &subject, &claim_type, &source_chain, &source_tx, timestamp,
    );
    if Storage::has_attestation(env, &attestation_id) {
        return Err(Error::DuplicateAttestation);
    }

    let attestation = Attestation {
        id: attestation_id.clone(),
        issuer: bridge,
        subject,
        claim_type,
        timestamp,
        expiration,
        revoked: false,
        deleted: false,
        metadata: None,
        jurisdiction: None,
        valid_from: None,
        origin: AttestationOrigin::Bridged,
        source_chain: Some(source_chain),
        source_tx: Some(source_tx),
        tags: None,
        revocation_reason: None,
        bundle_id: None,
    };

    store_attestation(env, &attestation);
    Events::attestation_bridged(env, &attestation);
    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Created,
        actor: attestation.issuer.clone(),
        timestamp,
        details: None,
    });
    // Keep the attestation and its indexes live until expiration.
    Storage::extend_attestation_ttl_for_expiry(env, &attestation);
    Ok(attestation_id)
}

pub fn create_attestations_batch(
    env: &Env,
    issuer: Address,
    subjects: Vec<Address>,
    claim_type: String,
    expiration: Option<u64>,
) -> Result<Vec<String>, Error> {
    const MAX_BATCH_SIZE: u32 = 50;

    if subjects.is_empty() || subjects.len() > MAX_BATCH_SIZE as usize {
        return Err(Error::LimitExceeded);
    }

    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;
    validate_attestation_metadata(env, &claim_type, &None)?;
    validate_native_expiration(env, expiration)?;

    for subject in subjects.iter() {
        validate_new_attestation_common(env, &issuer, subject, &claim_type)?;
        if Storage::is_whitelist_mode(env, &issuer) && !Storage::is_whitelisted(env, &issuer, subject) {
            return Err(Error::SubjectNotWhitelisted);
        }
    }
    // Rate-limit check is performed per-subject inside the loop so that each
    // individual issuance in a batch consumes its own rate-limit slot.  A single
    // check before the loop would allow a batch of N subjects to bypass the
    // configured issuance interval entirely.

    let timestamp = env.ledger().timestamp();
    let limits = Storage::get_limits(env);
    let issuer_count = crate::storage::ChunkedIndex::issuer_count(env, &issuer);
    if issuer_count.saturating_add(subjects.len()) > limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }

    let mut ids: Vec<String> = Vec::new(env);
    // Accumulate new IDs so the issuer index can be written once at the end
    // instead of once per subject (N reads + N writes → 1 read + 1 write).
    let mut new_issuer_ids: Vec<String> = Vec::new(env);

    for subject in subjects.iter() {
        // Enforce rate limits per individual issuance so the full batch cannot
        // bypass a configured issuance interval.  The last-issuance timestamp is
        // updated immediately after each successful check so the next iteration
        // (and the next transaction) both see the correct state.
        check_rate_limit(env, &issuer, &claim_type)?;
        // Record the issuance timestamp right away so subsequent loop iterations
        // are checked against it.
        Storage::set_last_issuance_time(env, &issuer, timestamp);
        if Storage::get_claim_type_rate_limit(env, &claim_type).is_some() {
            Storage::set_last_issuance_time_by_claim_type(env, &issuer, &claim_type, timestamp);
        }

        let attestation_id = Attestation::generate_id(env, &issuer, &subject, &claim_type, timestamp);
        if Storage::has_attestation(env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }
        let subject_count = crate::storage::ChunkedIndex::subject_count(env, &subject);
        if subject_count >= effective_max_attestations_per_subject(env) {
            return Err(Error::LimitExceeded);
        }
        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: issuer.clone(),
            subject: subject.clone(),
            claim_type: claim_type.clone(),
            timestamp,
            expiration,
            revoked: false,
            deleted: false,
            metadata: None,
            jurisdiction: None,
            valid_from: None,
            origin: AttestationOrigin::Native,
            source_chain: None,
            source_tx: None,
            tags: None,
            revocation_reason: None,
            bundle_id: None,
        };

        // Write attestation record and per-subject index — issuer index deferred.
        Storage::set_attestation(env, &attestation);
        // Only write to ChunkedIndex and ValidAttestations — not the legacy flat
        // SubjectAttestations key — to consolidate subject writes (issue #1550).
        Storage::add_valid_attestation(env, &subject, &attestation_id);
        crate::storage::ChunkedIndex::add_subject(env, &subject, &attestation_id);

        Storage::append_audit_entry(
            env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Created,
                actor: issuer.clone(),
                timestamp,
                details: None,
            },
        );
        // Keep this attestation and its subject-side indexes live until expiration.
        Storage::extend_attestation_ttl_for_expiry(env, &attestation);
        Events::attestation_created(env, &attestation);

        new_issuer_ids.push_back(attestation_id.clone());
        ids.push_back(attestation_id);
    }

    let batch_len = new_issuer_ids.len();

    // Single write: issuer index (replaces N add_issuer_attestation calls).
    Storage::add_issuer_attestations_bulk(env, &issuer, &new_issuer_ids);
    crate::storage::ChunkedIndex::add_issuer_bulk(env, &issuer, &new_issuer_ids);

    // The issuer chunks above were written after the per-attestation extensions,
    // so extend them now to cover the full expiration window.
    if let Some(first) = ids.first() {
        if let Ok(att) = Storage::get_attestation(env, first) {
            Storage::extend_attestation_ttl_for_expiry(env, &att);
        }
    }

    // Single write: issuer stats (replaces N set_issuer_stats calls).
    Storage::increment_issuer_stats(env, &issuer, batch_len);

    // Single write: global stats (replaces N increment_total_attestations calls).
    Storage::increment_total_attestations(env, batch_len as u64);
    charge_attestation_fees(env, &issuer, subjects.len() as u32)?;

    // Last-issuance timestamps are updated per-item inside the loop above so
    // that each subject issuance consumes its own rate-limit slot.
    Ok(ids)
}

pub fn revoke_attestation(
    env: &Env,
    issuer: Address,
    attestation_id: String,
    reason: Option<String>,
) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_not_paused(env)?;
    // Allow both registered issuers and registered bridges to revoke
    // attestations they created — bridges could not previously revoke their
    // own bridged attestations because require_issuer rejected them.
    Validation::require_authorized_creator(env, &issuer)?;
    validate_reason(&reason)?;

    let mut attestation = Storage::get_attestation(env, &attestation_id)?;
    if attestation.issuer != issuer {
        return Err(Error::Unauthorized);
    }
    if attestation.revoked {
        return Err(Error::AlreadyRevoked);
    }

    attestation.revoked = true;
    attestation.revocation_reason = reason.clone();
    Storage::set_attestation(env, &attestation);
    Storage::remove_subject_attestation(env, &attestation.subject, &attestation_id);
    Storage::remove_valid_attestation(env, &attestation.subject, &attestation_id);
    Storage::remove_issuer_attestation(env, &issuer, &attestation_id);
    crate::storage::ChunkedIndex::remove_subject(env, &attestation.subject, &attestation_id);
    crate::storage::ChunkedIndex::remove_issuer(env, &issuer, &attestation_id);

    Events::attestation_revoked(env, &attestation_id, &issuer, &reason);
    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Revoked,
        actor: issuer.clone(),
        timestamp: env.ledger().timestamp(),
        details: reason.clone(),
    });
    Storage::increment_total_revocations(env, 1);
    Storage::increment_issuer_revocations(env, &issuer, 1);

    // Update bundle validity if this attestation belongs to a bundle
    if let Some(bundle_id) = attestation.bundle_id {
        if let Some(mut bundle) = Storage::get_bundle(env, &bundle_id) {
            if bundle.all_valid {
                bundle.all_valid = false;
                Storage::set_bundle(env, &bundle);
            }
        }
    }

    Ok(())
}

pub fn renew_attestation(
    env: &Env,
    issuer: Address,
    attestation_id: String,
    new_expiration: Option<u64>,
) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;
    validate_native_expiration(env, new_expiration)?;

    let mut attestation = Storage::get_attestation(env, &attestation_id)?;
    if attestation.issuer != issuer {
        return Err(Error::Unauthorized);
    }
    if attestation.deleted {
        return Err(Error::InvalidStatus);
    }
    if attestation.revoked {
        return Err(Error::AlreadyRevoked);
    }
    if let (Some(expiration), Some(valid_from)) = (new_expiration, attestation.valid_from) {
        if expiration <= valid_from {
            return Err(Error::InvalidExpiration);
        }
    }

    attestation.expiration = new_expiration;
    Storage::set_attestation(env, &attestation);
    Events::attestation_renewed(env, &attestation_id, &issuer, new_expiration);
    let details = new_expiration.map(|ts| u64_to_string(env, ts));
    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Renewed,
        actor: issuer.clone(),
        timestamp: env.ledger().timestamp(),
        details,
    });
    // Keep the attestation and its indexes live until the new expiration.
    Storage::extend_attestation_ttl_for_expiry(env, &attestation);
    Ok(())
}

pub fn revoke_attestations_batch(
    env: &Env,
    issuer: Address,
    attestation_ids: Vec<String>,
    reason: Option<String>,
) -> Result<u32, Error> {
    const MAX_BATCH: u32 = 50;

    issuer.require_auth();
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &issuer)?;
    validate_reason(&reason)?;

    if attestation_ids.len() > MAX_BATCH {
        return Err(Error::LimitExceeded);
    }

    let mut seen_ids: Vec<String> = Vec::new(env);
    let mut attestations: Vec<Attestation> = Vec::new(env);

    for id in attestation_ids.iter() {
        for existing_id in seen_ids.iter() {
            if existing_id == id {
                return Err(Error::DuplicateAttestation);
            }
        }
        seen_ids.push_back(id.clone());

        let attestation = Storage::get_attestation(env, &id)?;
        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }
        attestations.push_back(attestation);
    }

    let mut count: u32 = 0;
    for attestation in attestations.iter() {
        let mut attestation = attestation.clone();
        attestation.revoked = true;
        attestation.revocation_reason = reason.clone();
        Storage::set_attestation(env, &attestation);
        Storage::remove_subject_attestation(env, &attestation.subject, &attestation.id);
        Storage::remove_valid_attestation(env, &attestation.subject, &attestation.id);
        Storage::remove_issuer_attestation(env, &issuer, &attestation.id);
        crate::storage::ChunkedIndex::remove_subject(env, &attestation.subject, &attestation.id);
        crate::storage::ChunkedIndex::remove_issuer(env, &issuer, &attestation.id);
        Events::attestation_revoked(env, &attestation.id, &issuer, &reason);
        Storage::append_audit_entry(
            env,
            &attestation.id,
            &AuditEntry {
                action: AuditAction::Revoked,
                actor: issuer.clone(),
                timestamp: env.ledger().timestamp(),
                details: reason.clone(),
            },
        );

        // Update bundle validity if this attestation belongs to a bundle
        if let Some(bundle_id) = attestation.bundle_id {
            if let Some(mut bundle) = Storage::get_bundle(env, &bundle_id) {
                if bundle.all_valid {
                    bundle.all_valid = false;
                    Storage::set_bundle(env, &bundle);
                }
            }
        }

        count += 1;
    }

    if count > 0 {
        Storage::increment_total_revocations(env, count as u64);
        Storage::increment_issuer_revocations(env, &issuer, count as u64);
    }
    Ok(count)
}

pub fn update_expiration(
    env: &Env,
    issuer: Address,
    attestation_id: String,
    new_expiration: Option<u64>,
) -> Result<(), Error> {
    renew_attestation(env, issuer, attestation_id, new_expiration)
}

pub fn transfer_attestation(
    env: &Env,
    admin: Address,
    attestation_id: String,
    new_issuer: Address,
) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &new_issuer)?;

    let mut attestation = Storage::get_attestation(env, &attestation_id)?;
    let old_issuer = attestation.issuer.clone();

    // Reject revoked or deleted attestations
    if attestation.revoked || attestation.deleted {
        return Err(Error::InvalidStatus);
    }

    // Reject self-attestation (new_issuer == subject)
    if new_issuer == attestation.subject {
        return Err(Error::Unauthorized);
    }

    if old_issuer == new_issuer {
        return Ok(());
    }

    // Clean both the legacy flat index and the new chunked index
    Storage::remove_issuer_attestation(env, &old_issuer, &attestation_id);
    Storage::add_issuer_attestation(env, &new_issuer, &attestation_id);
    crate::storage::ChunkedIndex::remove_issuer(env, &old_issuer, &attestation_id);
    crate::storage::ChunkedIndex::add_issuer(env, &new_issuer, &attestation_id);

    // Move issuer stats
    let mut old_stats = Storage::get_issuer_stats(env, &old_issuer);
    old_stats.total_issued = old_stats.total_issued.saturating_sub(1);
    Storage::set_issuer_stats(env, &old_issuer, &old_stats);

    let mut new_stats = Storage::get_issuer_stats(env, &new_issuer);
    new_stats.total_issued = new_stats.total_issued.saturating_add(1);
    Storage::set_issuer_stats(env, &new_issuer, &new_stats);

    // Update attestation with new issuer
    attestation.issuer = new_issuer.clone();
    Storage::set_attestation(env, &attestation);

    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Transferred,
        actor: admin.clone(),
        timestamp: env.ledger().timestamp(),
        details: Some(new_issuer.to_string()),
    });
    // `set_attestation` above reset the record's TTL to the default window;
    // re-apply the expiry-aware extension so the moved attestation stays live.
    Storage::extend_attestation_ttl_for_expiry(env, &attestation);
    Events::attestation_transferred(env, &attestation_id, &old_issuer, &new_issuer);
    Ok(())
}

pub fn request_deletion(env: &Env, subject: Address, attestation_id: String) -> Result<(), Error> {
    subject.require_auth();

    let mut attestation = Storage::get_attestation(env, &attestation_id)?;
    if attestation.subject != subject {
        return Err(Error::Unauthorized);
    }

    attestation.deleted = true;
    Storage::set_attestation(env, &attestation);
    Storage::remove_subject_attestation(env, &subject, &attestation_id);
    Storage::remove_valid_attestation(env, &subject, &attestation_id);
    crate::storage::ChunkedIndex::remove_subject(env, &subject, &attestation_id);
    // Also remove from the issuer index so the issuer index never retains
    // deleted IDs (the subject index already drops them on deletion).
    crate::storage::ChunkedIndex::remove_issuer(env, &attestation.issuer, &attestation_id);

    let timestamp = env.ledger().timestamp();
    Events::deletion_requested(env, &subject, &attestation_id, timestamp);
    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Deleted,
        actor: subject.clone(),
        timestamp,
        details: None,
    });
    Ok(())
}

// -----------------------------------------------------------------------
// Amendment
// -----------------------------------------------------------------------

pub fn amend_attestation(
    env: &Env,
    issuer: Address,
    attestation_id: String,
    new_metadata: Option<String>,
) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &issuer)?;

    let mut attestation = Storage::get_attestation(env, &attestation_id)?;
    validate_attestation_metadata(env, &attestation.claim_type, &new_metadata)?;
    if attestation.issuer != issuer {
        return Err(Error::Unauthorized);
    }
    if attestation.revoked {
        return Err(Error::AlreadyRevoked);
    }
    if attestation.deleted {
        return Err(Error::NotFound);
    }

    let timestamp = env.ledger().timestamp();

    // Snapshot the current state before applying the amendment.
    let version = Storage::get_attestation_version_count(env, &attestation_id);
    let snapshot = AttestationVersionSnapshot {
        version,
        metadata: attestation.metadata.clone(),
        amended_at: timestamp,
        amended_by: issuer.clone(),
    };
    Storage::push_attestation_version(env, &attestation_id, &snapshot);

    attestation.metadata = new_metadata.clone();
    Storage::set_attestation(env, &attestation);

    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Amended,
        actor: issuer.clone(),
        timestamp,
        details: new_metadata,
    });
    // `set_attestation`/`push_attestation_version` above reset TTLs to the
    // default window; re-apply the expiry-aware extension so an amended
    // long-lived attestation stays live.
    Storage::extend_attestation_ttl_for_expiry(env, &attestation);
    Events::attestation_amended(env, &attestation_id, &issuer, timestamp);
    Ok(())
}

// -----------------------------------------------------------------------
// Endorsement
// -----------------------------------------------------------------------

pub fn endorse_attestation(env: &Env, endorser: Address, attestation_id: String) -> Result<(), Error> {
    endorser.require_auth();
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &endorser)?;

    let attestation = Storage::get_attestation(env, &attestation_id)?;

    // Reject self-endorsement by the issuer
    if attestation.issuer == endorser {
        return Err(Error::CannotEndorseOwn);
    }

    // Reject self-endorsement by the subject
    if attestation.subject == endorser {
        return Err(Error::CannotEndorseOwn);
    }

    if attestation.revoked {
        return Err(Error::AlreadyRevoked);
    }

    // Reject expired, pending (valid_from in the future), or GDPR-deleted attestations
    let now = env.ledger().timestamp();
    if attestation.get_status(now) != AttestationStatus::Valid {
        return Err(Error::AttestationNotValid);
    }
    if attestation.deleted {
        return Err(Error::AttestationNotValid);
    }

    let existing = Storage::get_endorsements(env, &attestation_id);
    for e in existing.iter() {
        if e.endorser == endorser {
            return Err(Error::AlreadyEndorsed);
        }
    }

    let timestamp = env.ledger().timestamp();
    let endorsement = Endorsement {
        attestation_id: attestation_id.clone(),
        endorser: endorser.clone(),
        timestamp,
    };
    Storage::add_endorsement(env, &attestation_id, &endorsement);

    // Emit event for indexer and reputation dashboards
    Events::attestation_endorsed(env, &attestation_id, &endorser, timestamp);

    Ok(())
}

pub fn get_endorsement_count(env: &Env, attestation_id: String) -> u32 {
    Storage::get_endorsements(env, &attestation_id).len()
}

pub fn list_endorsements_by_endorser(env: &Env, endorser: Address, start: u32, limit: u32) -> Vec<Endorsement> {
    let all = Storage::get_endorsements_by_endorser(env, &endorser);
    let total = all.len();
    let start = start.min(total);
    let end = (start + limit).min(total);
    let mut result = Vec::new(env);
    for i in start..end {
        if let Some(item) = all.get(i) {
            result.push_back(item);
        }
    }
    result
}

// -----------------------------------------------------------------------
// Delegated attestation creation
// -----------------------------------------------------------------------

pub fn create_attestation_as_delegate(
    env: &Env,
    delegate: Address,
    delegator: Address,
    subject: Address,
    claim_type: String,
    expiration: Option<u64>,
    metadata: Option<String>,
) -> Result<String, Error> {
    delegate.require_auth();
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &delegator)?;
    validate_new_attestation_common(env, &delegator, &subject, &claim_type)?;
    validate_attestation_metadata(env, &claim_type, &metadata)?;
    validate_native_expiration(env, expiration)?;

    // Verify delegation exists and is not expired.
    let delegation = Storage::get_delegation(env, &delegator, &delegate, &claim_type)
        .ok_or(Error::Unauthorized)?;
    if let Some(exp) = delegation.expiration {
        if env.ledger().timestamp() >= exp {
            return Err(Error::Unauthorized);
        }
    }

    if delegate == subject {
        return Err(Error::Unauthorized);
    }
    if Storage::is_whitelist_mode(env, &delegator) && !Storage::is_whitelisted(env, &delegator, &subject) {
        return Err(Error::SubjectNotWhitelisted);
    }
    check_rate_limit(env, &delegator, &claim_type)?;

    let limits = Storage::get_limits(env);
    if crate::storage::ChunkedIndex::issuer_count(env, &delegator) >= limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }
    if crate::storage::ChunkedIndex::subject_count(env, &subject) >= effective_max_attestations_per_subject(env) {
        return Err(Error::LimitExceeded);
    }

    let timestamp = env.ledger().timestamp();
    let attestation_id = Attestation::generate_id(env, &delegator, &subject, &claim_type, timestamp);
    if Storage::has_attestation(env, &attestation_id) {
        return Err(Error::DuplicateAttestation);
    }

    let attestation = Attestation {
        id: attestation_id.clone(),
        issuer: delegator.clone(),
        subject,
        claim_type,
        timestamp,
        expiration,
        revoked: false,
        deleted: false,
        metadata,
        jurisdiction: None,
        valid_from: None,
        origin: AttestationOrigin::Native,
        source_chain: None,
        source_tx: None,
        tags: None,
        revocation_reason: None,
        bundle_id: None,
    };

    store_attestation(env, &attestation);
    Storage::append_audit_entry(
        env,
        &attestation_id,
        &AuditEntry {
            action: AuditAction::Created,
            actor: delegate.clone(),
            timestamp,
            details: None,
        },
    );
    Storage::set_last_issuance_time(env, &delegator, timestamp);
    if Storage::get_claim_type_rate_limit(env, &claim_type).is_some() {
        Storage::set_last_issuance_time_by_claim_type(env, &delegator, &claim_type, timestamp);
    }
    charge_attestation_fee(env, &delegator)?;
    // Keep the attestation and its indexes live until expiration.
    Storage::extend_attestation_ttl_for_expiry(env, &attestation);
    Events::attestation_created(env, &attestation);
    Ok(attestation_id)
}
/// Simulates creating an attestation without committing any state.
///
/// Returns the would-be attestation ID and fee amount that would result from
/// calling `create_attestation` with the same arguments.
///
/// This is a read-only query that can be used by UIs to show users the ID
/// and fee before they confirm the transaction, eliminating the need to
/// replicate ID-derivation and fee-calculation logic client-side.
///
/// # Parameters
/// - `issuer` — the issuer address (must authenticate via `require_auth()`)
/// - `subject` — the subject address
/// - `claim_type` — the claim type string
/// - `expiration` — optional expiration timestamp
/// - `metadata` — optional metadata string
/// - `tags` — optional tags vector
///
/// # Returns
/// A tuple of `(attestation_id, fee_amount)` where:
/// - `attestation_id` — the 64-character hex string that would be generated
/// - `fee_amount` — the fee that would be charged (0 if no fee is configured)
///
/// # Errors
/// Returns the same errors as `create_attestation`:
/// - `Error::NotInitialized` — contract not initialized
/// - `Error::Unauthorized` — issuer is not registered or issuer == subject
/// - `Error::InvalidClaimType` — claim type validation failed
/// - `Error::NotRegisteredClaimType` — claim type not registered when required
/// - `Error::InvalidMetadata` — metadata validation failed
/// - `Error::LimitExceeded` — attestation limits reached
/// - `Error::SubjectNotWhitelisted` — whitelist mode active and subject not whitelisted
/// - `Error::RateLimited` — rate limit exceeded
/// - `Error::FeeTokenRequired` — fee configured but no token address
pub fn simulate_create_attestation(
    env: &Env,
    issuer: Address,
    subject: Address,
    claim_type: String,
    expiration: Option<u64>,
    metadata: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<(String, i128), Error> {
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &issuer)?;
    Validation::validate_claim_type(&claim_type)?;
    Validation::require_registered_claim_type(env, &claim_type)?;
    Validation::validate_metadata(env, &metadata)?;
    Validation::validate_claim_constraints(env, &claim_type, &metadata)?;
    Validation::validate_metadata_hash_only(env, &metadata)?;
    validate_jurisdiction(env, &None)?;
    validate_tags(&tags)?;
    validate_native_expiration(env, expiration)?;
    validate_valid_from(env, None)?;

    if issuer == subject {
        return Err(Error::Unauthorized);
    }

    if Storage::is_whitelist_mode(env, &issuer) && !Storage::is_whitelisted(env, &issuer, &subject) {
        return Err(Error::SubjectNotWhitelisted);
    }

    check_rate_limit(env, &issuer, &claim_type)?;

    let limits = Storage::get_limits(env);
    let issuer_count = crate::storage::ChunkedIndex::issuer_count(env, &issuer);
    if issuer_count >= limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }
    let subject_count = crate::storage::ChunkedIndex::subject_count(env, &subject);
    if subject_count >= limits.max_attestations_per_subject {
        return Err(Error::LimitExceeded);
    }

    let timestamp = env.ledger().timestamp();
    let attestation_id = Attestation::generate_id(env, &issuer, &subject, &claim_type, timestamp);

    if Storage::has_attestation(env, &attestation_id) {
        return Err(Error::DuplicateAttestation);
    }

    // Calculate the fee without charging it
    let fee_config = load_fee_config(env)?;
    let fee_amount = if fee_config.attestation_fee == 0 {
        0
    } else {
        fee_config.attestation_fee
    };

    Ok((attestation_id, fee_amount))
}

// -----------------------------------------------------------------------
// Bundle operations
// -----------------------------------------------------------------------

/// Create a bundle of attestations for multiple subjects with the same claim type.
/// Each attestation is added to the bundle independently via store_attestation.
/// No redundant indexing or stats updates occur.
pub fn create_attestation_bundle(
    env: &Env,
    issuer: Address,
    subjects: Vec<Address>,
    claim_type: String,
    expiration: Option<u64>,
) -> Result<String, Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;
    validate_new_attestation_common(env, &issuer, &subjects.get(0).ok_or(Error::LimitExceeded)?, &claim_type)?;
    validate_native_expiration(env, expiration)?;
    check_rate_limit(env, &issuer, &claim_type)?;

    let timestamp = env.ledger().timestamp();
    let limits = Storage::get_limits(env);
    let issuer_count = crate::storage::ChunkedIndex::issuer_count(env, &issuer);
    if issuer_count.saturating_add(subjects.len()) > limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }

    let mut attestation_ids: Vec<String> = Vec::new(env);

    for subject in subjects.iter() {
        if issuer == subject {
            return Err(Error::Unauthorized);
        }

        let attestation_id = Attestation::generate_id(env, &issuer, &subject, &claim_type, timestamp);
        if Storage::has_attestation(env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }

        let subject_count = Storage::get_subject_attestations(env, &subject).len();
        if subject_count >= limits.max_attestations_per_subject {
            return Err(Error::LimitExceeded);
        }

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: issuer.clone(),
            subject: subject.clone(),
            claim_type: claim_type.clone(),
            timestamp,
            expiration,
            revoked: false,
            deleted: false,
            metadata: None,
            jurisdiction: None,
            valid_from: None,
            origin: AttestationOrigin::Native,
            source_chain: None,
            source_tx: None,
            tags: None,
            revocation_reason: None,
        };

        // Rely on store_attestation only - no redundant indexing or stats updates
        store_attestation(env, &attestation);
        Storage::append_audit_entry(
            env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Created,
                actor: issuer.clone(),
                timestamp,
                details: None,
            },
        );
        Events::attestation_created(env, &attestation);
        attestation_ids.push_back(attestation_id);
    }

    let bundle_id = crate::types::AttestationBundle::generate_id(env, &issuer, &claim_type, timestamp);
    let bundle = crate::types::AttestationBundle {
        id: bundle_id.clone(),
        issuer: issuer.clone(),
        claim_type,
        attestation_ids,
        created_at: timestamp,
        all_valid: true,
    };

    Storage::set_bundle(env, &bundle);
    Storage::set_last_issuance_time(env, &issuer, timestamp);
    if Storage::get_claim_type_rate_limit(env, &bundle.claim_type).is_some() {
        Storage::set_last_issuance_time_by_claim_type(env, &issuer, &bundle.claim_type, timestamp);
    }

    charge_attestation_fee(env, &issuer)?;

    Events::bundle_created(env, &bundle_id, &issuer, bundle.attestation_ids.len());
    Ok(bundle_id)
}

/// Query the validity of a bundle: checks expiration of each member and returns
/// whether all members are currently valid (not revoked, not deleted, not expired).
/// This is a pure getter function with no side effects.
pub fn is_bundle_valid(env: &Env, bundle_id: String) -> Result<bool, Error> {
    let bundle = Storage::get_bundle(env, &bundle_id)?;
    let current_time = env.ledger().timestamp();

    for attestation_id in bundle.attestation_ids.iter() {
        match Storage::get_attestation(env, &attestation_id) {
            Ok(attestation) => {
                // Check status properly: revoked, deleted, expired, or not-yet-valid members make bundle invalid
                let status = attestation.get_status(current_time);
                match status {
                    crate::types::AttestationStatus::Valid => {},
                    _ => return Ok(false),
                }
            }
            Err(_) => {
                // Missing attestations make bundle invalid
                return Ok(false);
            }
        }
    }

    Ok(true)
}
