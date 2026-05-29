use soroban_sdk::{Address, Env, String, Vec};

use crate::attestation::maybe_trigger_expiration_hook;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{Attestation, AttestationStatus, AuditEntry, Error, GlobalStats};

/// Returns `true` if the subject holds at least one valid attestation for `claim_type`.
///
/// Loads the subject's attestation index, then iterates entries in insertion order.
/// Returns immediately on the first matching attestation with status
/// [`AttestationStatus::Valid`] (short-circuit evaluation).
///
/// **Complexity:** O(n) in the worst case, where n is the number of attestations
/// indexed for the subject. Loading the subject index is O(n), and the attestation
/// scan is also O(n) when no valid match exists or when the only valid match is
/// the last indexed entry. The best case is O(1) attestation reads when the first
/// indexed entry is a valid match.
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
                    AttestationStatus::Valid => {
                        maybe_trigger_expiration_hook(
                            env,
                            &subject,
                            &attestation_id,
                            attestation.expiration.unwrap_or(u64::MAX),
                            current_time,
                        );
                        return true;
                    }
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
    // Use the chunked index: loads only the chunks that overlap [start, start+limit).
    let ids = crate::storage::ChunkedIndex::get_subject_page(env, &subject, start, limit);
    let mut result = Vec::new(env);
    for id in ids.iter() {
        if let Ok(a) = Storage::get_attestation(env, &id) {
            if !a.deleted {
                result.push_back(id);
            }
        }
    }
    result
}

/// Search the subject's attestations between `from_ts` and `to_ts`, excluding deleted records.
///
/// This implementation uses offset-based pagination over the current filtered result set.
/// Because GDPR deletions remove attestation IDs from the subject index, clients should
/// prefer `get_attestations_in_range_after` for page-to-page traversal when attestations
/// may be deleted between requests. The legacy `start`/`limit` API remains available for
/// simple pagination but can skip records if earlier pages are changed by concurrent deletions.
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

/// Search the subject's attestations between `from_ts` and `to_ts` using cursor pagination.
///
/// This function is the preferred pagination path for integrators when a subject's
/// attestations may be deleted between page fetches. The `after_attestation_id` cursor
/// is the last attestation ID returned by the previous page. When the cursor record has
/// been removed due to GDPR deletion, this implementation attempts to resume from the
/// next available attestation after the deleted cursor.
///
/// Note: if the provided cursor ID is invalid or does not point to a known attestation,
/// an empty result is returned in order to avoid unpredictable page recovery.
pub fn get_attestations_in_range_after(
    env: &Env,
    subject: Address,
    from_ts: u64,
    to_ts: u64,
    after_attestation_id: Option<String>,
    limit: u32,
) -> Vec<Attestation> {
    if from_ts > to_ts {
        return Vec::new(env);
    }

    let attestation_ids = Storage::get_subject_attestations(env, &subject);
    let mut filtered = Vec::new(env);
    for id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &id) {
            if !attestation.deleted && attestation.timestamp >= from_ts && attestation.timestamp <= to_ts {
                filtered.push_back(attestation);
            }
        }
    }

    let mut start_index: u32 = 0;
    if let Some(cursor_id) = after_attestation_id {
        let mut cursor_found = false;
        let mut cursor_timestamp: u64 = 0;
        let mut cursor_id_ref = cursor_id.clone();
        if let Ok(cursor_attestation) = Storage::get_attestation(env, &cursor_id) {
            cursor_timestamp = cursor_attestation.timestamp;
            for i in 0..filtered.len() {
                if let Some(attestation) = filtered.get(i) {
                    if attestation.id == cursor_attestation.id {
                        start_index = i + 1;
                        cursor_found = true;
                        break;
                    }
                }
            }
        } else {
            return Vec::new(env);
        }

        if !cursor_found {
            for i in 0..filtered.len() {
                if let Some(attestation) = filtered.get(i) {
                    if attestation.timestamp > cursor_timestamp
                        || (attestation.timestamp == cursor_timestamp && attestation.id > cursor_id_ref)
                    {
                        start_index = i;
                        cursor_found = true;
                        break;
                    }
                }
            }
        }

        if !cursor_found {
            return Vec::new(env);
        }
    }

    let mut result = Vec::new(env);
    let end = (start_index + limit).min(filtered.len());
    for i in start_index..end {
        if let Some(attestation) = filtered.get(i) {
            result.push_back(attestation.clone());
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
    // Use the chunked index: loads only the chunks that overlap [start, start+limit).
    let ids = crate::storage::ChunkedIndex::get_issuer_page(env, &issuer, start, limit);
    let mut result = Vec::new(env);
    for id in ids.iter() {
        if let Ok(a) = Storage::get_attestation(env, &id) {
            if !a.deleted {
                result.push_back(id);
            }
        }
    }
    result
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
    crate::storage::ChunkedIndex::subject_count(env, &subject)
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
