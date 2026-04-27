use soroban_sdk::{Address, Env, String, Vec};

use crate::attestation::maybe_trigger_expiration_hook;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{Attestation, AttestationStatus, AuditEntry, Error, GlobalStats};

pub fn has_valid_claim(env: &Env, subject: Address, claim_type: String) -> bool {
    let attestation_ids = Storage::get_subject_attestations(env, &subject);
    let current_time = env.ledger().timestamp();

    for attestation_id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            if attestation.deleted || attestation.claim_type != claim_type {
                continue;
            }
            if attestation.get_status(current_time) == AttestationStatus::Valid {
                maybe_trigger_expiration_hook(
                    env,
                    &subject,
                    &attestation_id,
                    attestation.expiration.unwrap_or(u64::MAX),
                    current_time,
                );
                return true;
            }
        }
    }
    false
}

pub fn has_valid_claim_from_issuer(env: &Env, subject: Address, claim_type: String, issuer: Address) -> bool {
    let attestation_ids = Storage::get_subject_attestations(env, &subject);
    let current_time = env.ledger().timestamp();
    for attestation_id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            if attestation.deleted { continue; }
            if attestation.claim_type == claim_type && attestation.issuer == issuer {
                match attestation.get_status(current_time) {
                    AttestationStatus::Valid => return true,
                    AttestationStatus::Expired => {
                        Events::attestation_expired(env, &attestation_id, &subject);
                    }
                    _ => {}
                }
            }
        }
    }
    false
}

pub fn has_any_claim(env: &Env, subject: Address, claim_types: Vec<String>) -> bool {
    if claim_types.is_empty() {
        return false;
    }
    let attestation_ids = Storage::get_subject_attestations(env, &subject);
    let current_time = env.ledger().timestamp();
    for claim_type in claim_types.iter() {
        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
                if !attestation.deleted
                    && attestation.claim_type == claim_type
                    && attestation.get_status(current_time) == AttestationStatus::Valid
                {
                    return true;
                }
            }
        }
    }
    false
}

pub fn has_all_claims(env: &Env, subject: Address, claim_types: Vec<String>) -> bool {
    if claim_types.is_empty() { return true; }
    let attestation_ids = Storage::get_subject_attestations(env, &subject);
    let current_time = env.ledger().timestamp();
    'claims: for claim_type in claim_types.iter() {
        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
                if !attestation.deleted
                    && attestation.claim_type == claim_type
                    && attestation.get_status(current_time) == AttestationStatus::Valid
                {
                    continue 'claims;
                }
            }
        }
        return false;
    }
    true
}

pub fn get_attestation(env: &Env, attestation_id: String) -> Result<Attestation, Error> {
    let attestation = Storage::get_attestation(env, &attestation_id)?;
    if attestation.deleted {
        return Err(Error::NotFound);
    }
    Ok(attestation)
}

pub fn get_audit_log(env: &Env, attestation_id: String) -> Vec<AuditEntry> {
    Storage::get_audit_log(env, &attestation_id)
}

pub fn get_attestation_status(env: &Env, attestation_id: String) -> Result<AttestationStatus, Error> {
    let attestation = Storage::get_attestation(env, &attestation_id)?;
    if attestation.deleted {
        return Err(Error::NotFound);
    }
    let status = attestation.get_status(env.ledger().timestamp());
    if status == AttestationStatus::Expired {
        Events::attestation_expired(env, &attestation_id, &attestation.subject);
    }
    Ok(status)
}

pub fn get_subject_attestations(env: &Env, subject: Address, start: u32, limit: u32) -> Vec<String> {
    let ids = Storage::get_subject_attestations(env, &subject);
    let mut filtered = Vec::new(env);
    for id in ids.iter() {
        if let Ok(a) = Storage::get_attestation(env, &id) {
            if !a.deleted {
                filtered.push_back(id);
            }
        }
    }
    crate::storage::paginate(env, &filtered, start, limit)
}

pub fn get_attestations_in_range(
    env: &Env,
    subject: Address,
    from_ts: u64,
    to_ts: u64,
    start: u32,
    limit: u32,
) -> Vec<Attestation> {
    let attestation_ids = Storage::get_subject_attestations(env, &subject);
    let mut filtered_ids = Vec::new(env);
    for id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &id) {
            if !attestation.deleted && attestation.timestamp >= from_ts && attestation.timestamp <= to_ts {
                filtered_ids.push_back(id);
            }
        }
    }
    let paginated_ids = crate::storage::paginate(env, &filtered_ids, start, limit);
    let mut result = Vec::new(env);
    for id in paginated_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &id) {
            result.push_back(attestation);
        }
    }
    result
}

pub fn get_attestations_by_tag(env: &Env, subject: Address, tag: String) -> Vec<String> {
    let attestation_ids = Storage::get_subject_attestations(env, &subject);
    let mut result = Vec::new(env);
    for id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &id) {
            if attestation.deleted { continue; }
            if let Some(tags) = attestation.tags {
                for t in tags.iter() {
                    if t == tag {
                        result.push_back(id.clone());
                        break;
                    }
                }
            }
        }
    }
    result
}

pub fn get_attestations_by_jurisdiction(
    env: &Env,
    subject: Address,
    jurisdiction: String,
    start: u32,
    limit: u32,
) -> Vec<String> {
    let attestation_ids = Storage::get_subject_attestations(env, &subject);
    let mut filtered = Vec::new(env);
    for id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &id) {
            if attestation.deleted { continue; }
            if let Some(att_jurisdiction) = attestation.jurisdiction {
                if att_jurisdiction == jurisdiction {
                    filtered.push_back(id.clone());
                }
            }
        }
    }
    crate::storage::paginate(env, &filtered, start, limit)
}

pub fn get_issuer_attestations(env: &Env, issuer: Address, start: u32, limit: u32) -> Vec<String> {
    let ids = Storage::get_issuer_attestations(env, &issuer);
    let mut filtered = Vec::new(env);
    for id in ids.iter() {
        if let Ok(a) = Storage::get_attestation(env, &id) {
            if !a.deleted {
                filtered.push_back(id);
            }
        }
    }
    crate::storage::paginate(env, &filtered, start, limit)
}

pub fn get_issuer_attestation_count(env: &Env, issuer: Address) -> u32 {
    Storage::get_issuer_attestations(env, &issuer).len()
}

pub fn get_valid_claims(env: &Env, subject: Address) -> Vec<String> {
    let current_time = env.ledger().timestamp();
    let mut result = Vec::new(env);
    for attestation_id in Storage::get_subject_attestations(env, &subject).iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            if !attestation.deleted && attestation.get_status(current_time) == AttestationStatus::Valid {
                let mut already_present = false;
                for existing in result.iter() {
                    if existing == attestation.claim_type {
                        already_present = true;
                        break;
                    }
                }
                if !already_present {
                    result.push_back(attestation.claim_type);
                }
            }
        }
    }
    result
}

pub fn get_attestation_by_type(env: &Env, subject: Address, claim_type: String) -> Option<Attestation> {
    let attestation_ids = Storage::get_subject_attestations(env, &subject);
    let current_time = env.ledger().timestamp();
    let mut index = attestation_ids.len();
    while index > 0 {
        index -= 1;
        if let Some(attestation_id) = attestation_ids.get(index) {
            if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
                if !attestation.deleted
                    && attestation.claim_type == claim_type
                    && attestation.get_status(current_time) == AttestationStatus::Valid
                {
                    return Some(attestation);
                }
            }
        }
    }
    None
}

pub fn get_subject_attestation_count(env: &Env, subject: Address) -> u32 {
    Storage::get_subject_attestations(env, &subject).len()
}

pub fn get_valid_claim_count(env: &Env, subject: Address) -> u32 {
    let current_time = env.ledger().timestamp();
    let mut count = 0u32;
    for attestation_id in Storage::get_subject_attestations(env, &subject).iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            if !attestation.deleted && attestation.get_status(current_time) == AttestationStatus::Valid {
                count += 1;
            }
        }
    }
    count
}

pub fn get_global_stats(env: &Env) -> GlobalStats {
    Storage::get_global_stats(env)
}
