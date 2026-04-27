use soroban_sdk::{token::TokenClient, Address, Env, String, Vec};

use crate::constants::SECS_PER_DAY;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    Attestation, AttestationOrigin, AuditAction, AuditEntry, Endorsement, Error, FeeConfig,
};
use crate::validation::Validation;

pub const MAX_SOURCE_CHAIN_LEN: u32 = 32;
pub const MAX_SOURCE_TX_LEN: u32 = 128;

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
        if code.len() != 2 {
            return Err(Error::InvalidJurisdiction);
        }
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

pub fn check_rate_limit(env: &Env, issuer: &Address) -> Result<(), Error> {
    if let Some(config) = Storage::get_rate_limit_config(env) {
        if config.min_issuance_interval == 0 {
            return Ok(());
        }
        let current_time = env.ledger().timestamp();
        if let Some(last) = Storage::get_last_issuance_time(env, issuer) {
            if current_time.saturating_sub(last) < config.min_issuance_interval {
                return Err(Error::RateLimited);
            }
        }
    }
    Ok(())
}

pub fn load_fee_config(env: &Env) -> Result<FeeConfig, Error> {
    Storage::get_fee_config(env).ok_or(Error::NotInitialized)
}

pub fn charge_attestation_fee(env: &Env, issuer: &Address) -> Result<(), Error> {
    let fee_config = load_fee_config(env)?;
    if fee_config.attestation_fee == 0 {
        return Ok(());
    }
    let fee_token = fee_config.fee_token.ok_or(Error::FeeTokenRequired)?;
    TokenClient::new(env, &fee_token).transfer(issuer, &fee_config.fee_collector, &fee_config.attestation_fee);
    Ok(())
}

pub fn store_attestation(env: &Env, attestation: &Attestation) {
    Storage::set_attestation(env, attestation);
    Storage::add_subject_attestation(env, &attestation.subject, &attestation.id);
    Storage::add_issuer_attestation(env, &attestation.issuer, &attestation.id);
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
    Validation::validate_claim_type(&claim_type)?;
    Validation::validate_metadata(env, &metadata)?;
    validate_jurisdiction(env, &jurisdiction)?;
    validate_tags(&tags)?;
    validate_native_expiration(env, expiration)?;
    validate_valid_from(env, valid_from)?;

    if issuer == subject {
        return Err(Error::Unauthorized);
    }

    if Storage::is_whitelist_mode(env, &issuer) && !Storage::is_whitelisted(env, &issuer, &subject) {
        return Err(Error::SubjectNotWhitelisted);
    }

    check_rate_limit(env, &issuer)?;

    let limits = Storage::get_limits(env);
    let issuer_count = Storage::get_issuer_attestations(env, &issuer).len();
    if issuer_count >= limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }
    let subject_count = Storage::get_subject_attestations(env, &subject).len();
    if subject_count >= limits.max_attestations_per_subject {
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

    charge_attestation_fee(env, &issuer)?;

    Events::attestation_created(env, &attestation);
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
    validate_import_timestamps(env, timestamp, expiration)?;

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
    Ok(attestation_id)
}

pub fn bridge_attestation(
    env: &Env,
    bridge: Address,
    subject: Address,
    claim_type: String,
    source_chain: String,
    source_tx: String,
) -> Result<String, Error> {
    bridge.require_auth();
    Validation::require_bridge(env, &bridge)?;
    Validation::require_not_paused(env)?;
    validate_source_reference(&source_chain, &source_tx)?;

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
        expiration: None,
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
    };

    store_attestation(env, &attestation);
    Events::attestation_bridged(env, &attestation);
    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Created,
        actor: attestation.issuer.clone(),
        timestamp,
        details: None,
    });
    Ok(attestation_id)
}

pub fn create_attestations_batch(
    env: &Env,
    issuer: Address,
    subjects: Vec<Address>,
    claim_type: String,
    expiration: Option<u64>,
) -> Result<Vec<String>, Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;
    Validation::validate_claim_type(&claim_type)?;
    validate_native_expiration(env, expiration)?;
    check_rate_limit(env, &issuer)?;

    let timestamp = env.ledger().timestamp();
    let limits = Storage::get_limits(env);
    let issuer_count = Storage::get_issuer_attestations(env, &issuer).len();
    if issuer_count.saturating_add(subjects.len()) > limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }

    let mut ids: Vec<String> = Vec::new(env);
    for subject in subjects.iter() {
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
        store_attestation(env, &attestation);
        Events::attestation_created(env, &attestation);
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
        ids.push_back(attestation_id);
    }

    Storage::set_last_issuance_time(env, &issuer, timestamp);
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
    Validation::require_issuer(env, &issuer)?;
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
    Storage::remove_issuer_attestation(env, &issuer, &attestation_id);

    Events::attestation_revoked(env, &attestation_id, &issuer, &reason);
    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Revoked,
        actor: issuer.clone(),
        timestamp: env.ledger().timestamp(),
        details: reason.clone(),
    });
    Storage::increment_total_revocations(env, 1);
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
    if attestation.revoked {
        return Err(Error::AlreadyRevoked);
    }

    attestation.expiration = new_expiration;
    Storage::set_attestation(env, &attestation);
    Events::attestation_renewed(env, &attestation_id, &issuer, new_expiration);
    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Renewed,
        actor: issuer.clone(),
        timestamp: env.ledger().timestamp(),
        details: None,
    });
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
    Validation::require_issuer(env, &issuer)?;
    validate_reason(&reason)?;

    if attestation_ids.len() > MAX_BATCH {
        return Err(Error::LimitExceeded);
    }

    for id in attestation_ids.iter() {
        let attestation = Storage::get_attestation(env, &id)?;
        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }
    }

    let mut count: u32 = 0;
    for id in attestation_ids.iter() {
        let mut attestation = Storage::get_attestation(env, &id)?;
        attestation.revoked = true;
        attestation.revocation_reason = reason.clone();
        Storage::set_attestation(env, &attestation);
        Storage::remove_subject_attestation(env, &attestation.subject, &id);
        Storage::remove_issuer_attestation(env, &issuer, &id);
        Events::attestation_revoked_with_reason(env, &id, &issuer, &reason);
        Storage::append_audit_entry(
            env,
            &id,
            &AuditEntry {
                action: AuditAction::Revoked,
                actor: issuer.clone(),
                timestamp: env.ledger().timestamp(),
                details: reason.clone(),
            },
        );
        count += 1;
    }

    Storage::increment_total_revocations(env, count as u64);
    Ok(count)
}

pub fn update_expiration(
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
    if attestation.revoked {
        return Err(Error::AlreadyRevoked);
    }

    attestation.expiration = new_expiration;
    Storage::set_attestation(env, &attestation);
    Events::attestation_renewed(env, &attestation_id, &issuer, new_expiration);
    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Updated,
        actor: issuer.clone(),
        timestamp: env.ledger().timestamp(),
        details: None,
    });
    Ok(())
}

pub fn transfer_attestation(
    env: &Env,
    admin: Address,
    attestation_id: String,
    new_issuer: Address,
) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Validation::require_issuer(env, &new_issuer)?;

    let mut attestation = Storage::get_attestation(env, &attestation_id)?;
    let old_issuer = attestation.issuer.clone();

    if old_issuer == new_issuer {
        return Ok(());
    }

    Storage::remove_issuer_attestation(env, &old_issuer, &attestation_id);
    Storage::add_issuer_attestation(env, &new_issuer, &attestation_id);

    let mut old_stats = Storage::get_issuer_stats(env, &old_issuer);
    old_stats.total_issued = old_stats.total_issued.saturating_sub(1);
    Storage::set_issuer_stats(env, &old_issuer, &old_stats);

    let mut new_stats = Storage::get_issuer_stats(env, &new_issuer);
    new_stats.total_issued = new_stats.total_issued.saturating_add(1);
    Storage::set_issuer_stats(env, &new_issuer, &new_stats);

    attestation.issuer = new_issuer.clone();
    Storage::set_attestation(env, &attestation);

    Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
        action: AuditAction::Transferred,
        actor: admin.clone(),
        timestamp: env.ledger().timestamp(),
        details: Some(new_issuer.to_string()),
    });

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

    let timestamp = env.ledger().timestamp();
    Events::deletion_requested(env, &subject, &attestation_id, timestamp);
    Ok(())
}

// -----------------------------------------------------------------------
// Endorsement
// -----------------------------------------------------------------------

pub fn endorse_attestation(env: &Env, endorser: Address, attestation_id: String) -> Result<(), Error> {
    endorser.require_auth();
    Validation::require_issuer(env, &endorser)?;
    Validation::require_not_paused(env)?;

    let attestation = Storage::get_attestation(env, &attestation_id)?;
    if attestation.issuer == endorser {
        return Err(Error::CannotEndorseOwn);
    }
    if attestation.revoked {
        return Err(Error::AlreadyRevoked);
    }

    let existing = Storage::get_endorsements(env, &attestation_id);
    for e in existing.iter() {
        if e.endorser == endorser {
            return Err(Error::AlreadyEndorsed);
        }
    }

    let endorsement = Endorsement {
        attestation_id: attestation_id.clone(),
        endorser: endorser.clone(),
        timestamp: env.ledger().timestamp(),
    };
    Storage::add_endorsement(env, &endorsement);
    Ok(())
}

pub fn get_endorsement_count(env: &Env, attestation_id: String) -> u32 {
    Storage::get_endorsements(env, &attestation_id).len()
}
