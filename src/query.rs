use soroban_sdk::{Address, Env, String, Vec};

use crate::attestation::maybe_trigger_expiration_hook;
use crate::constants::{SECS_PER_DAY, MAX_EXPIRING_WINDOW_DAYS};
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    Attestation, AttestationStatus, AttestationVersionSnapshot, AuditEntry, Delegation,
    DisputeRecord, DisputeStatus, Error, GlobalStats, RevocationList, RevocationListFormat,
};
use crate::validation::Validation;

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
    // Use the pre-filtered valid-attestations index (non-revoked, non-deleted)
    // to avoid reading records that can never produce a true result.
    let attestation_ids = Storage::get_valid_attestations(env, &subject);
    let current_time = env.ledger().timestamp();

    for attestation_id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            if attestation.claim_type != claim_type {
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
    let attestation_ids = Storage::get_valid_attestations(env, &subject);
    let current_time = env.ledger().timestamp();
    for attestation_id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            if attestation.claim_type == claim_type && attestation.issuer == issuer {
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
    }
    false
}

pub fn has_any_claim(env: &Env, subject: Address, claim_types: Vec<String>) -> bool {
    if claim_types.is_empty() {
        return false;
    }
    let attestation_ids = Storage::get_valid_attestations(env, &subject);
    let current_time = env.ledger().timestamp();
    for claim_type in claim_types.iter() {
        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
                if attestation.claim_type == claim_type
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
    let attestation_ids = Storage::get_valid_attestations(env, &subject);
    let current_time = env.ledger().timestamp();
    'claims: for claim_type in claim_types.iter() {
        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
                if attestation.claim_type == claim_type
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

/// Issue #1567: Check if a subject has a valid attestation of a specific claim type that is NOT under dispute.
/// Verifiers can use this to enforce "valid and not under dispute" semantics.
pub fn has_valid_undisputed_claim(env: &Env, subject: Address, claim_type: String) -> bool {
    let attestation_ids = Storage::get_valid_attestations(env, &subject);
    let current_time = env.ledger().timestamp();

    for attestation_id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            if attestation.claim_type != claim_type {
                continue;
            }
            if attestation.get_status(current_time) == AttestationStatus::Valid {
                // Check that there is no open dispute on this attestation
                if let Some(dispute) = Storage::get_dispute(env, &attestation_id) {
                    // Skip this attestation if there's an open dispute
                    if dispute.status == DisputeStatus::Open {
                        continue;
                    }
                }
                return true;
            }
        }
    }
    false
}

/// Issue #1567: Check if a subject has a valid attestation from a specific issuer of a given claim type
/// that is NOT under dispute.
pub fn has_valid_undisputed_claim_from_issuer(
    env: &Env,
    subject: Address,
    claim_type: String,
    issuer: Address,
) -> bool {
    let attestation_ids = Storage::get_valid_attestations(env, &subject);
    let current_time = env.ledger().timestamp();
    for attestation_id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            if attestation.claim_type == claim_type && attestation.issuer == issuer {
                if attestation.get_status(current_time) == AttestationStatus::Valid {
                    // Check that there is no open dispute on this attestation
                    if let Some(dispute) = Storage::get_dispute(env, &attestation_id) {
                        if dispute.status == DisputeStatus::Open {
                            continue;
                        }
                    }
                    return true;
                }
            }
        }
    }
    false
}

/// Check if multiple subjects all have a valid attestation for the given claim type.
/// Returns a Vec<bool> where each element corresponds to whether that subject has the claim.
/// This is more efficient than making individual has_valid_claim calls.
pub fn has_valid_claim_batch(env: &Env, subjects: Vec<Address>, claim_type: String) -> Vec<bool> {
    let mut results = Vec::new(env);
    for subject in subjects.iter() {
        let has_claim = has_valid_claim(env, subject.clone(), claim_type.clone());
        results.push_back(has_claim);
    }
    results
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

pub fn get_delegation(
    env: &Env,
    delegator: Address,
    delegate: Address,
    claim_type: String,
) -> Option<Delegation> {
    Storage::get_delegation(env, &delegator, &delegate, &claim_type)
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

/// Fetch up to `limit` non-deleted attestation IDs for `subject`, returning the
/// slice `filtered[start..start+limit]` of the non-deleted index.
///
/// **Page-full guarantee:** the returned page contains exactly `limit` items
/// unless the end of the (non-deleted) index has been reached. Deleted IDs (and
/// any IDs whose record can't be loaded) are excluded from the index entirely,
/// so they neither shrink the page nor cause duplicates across pages. This makes
/// the function safe for cursor pagination where `start` advances by the number
/// of items returned (as the SDK's [`iterateSubjectAttestations`] does).
///
/// `start` is an offset into the *filtered* result set (deleted IDs excluded),
/// not into the underlying chunked index.
fn subject_page_filtered(env: &Env, subject: &Address, start: u32, limit: u32) -> Vec<String> {
    let all_ids = crate::storage::ChunkedIndex::get_subject_all(env, subject);
    let mut filtered = Vec::new(env);
    // Only need to collect up to `start + limit` live IDs to satisfy the page.
    let needed = start.saturating_add(limit);
    for id in all_ids.iter() {
        if filtered.len() >= needed as usize {
            break;
        }
        if let Ok(a) = Storage::get_attestation(env, &id) {
            if !a.deleted {
                filtered.push_back(id);
            }
        }
    }
    crate::storage::paginate(env, &filtered, start, limit)
}

pub fn get_subject_attestations(env: &Env, subject: Address, start: u32, limit: u32) -> Vec<String> {
    subject_page_filtered(env, &subject, start, limit)
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
    let attestation_ids = crate::storage::ChunkedIndex::get_subject_all(env, &subject);
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

    let attestation_ids = crate::storage::ChunkedIndex::get_subject_all(env, &subject);
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
                        || (attestation.timestamp == cursor_timestamp && attestation.id > cursor_id)
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
    // Use saturating_add to prevent overflow when limit is very large (e.g. u32::MAX).
    let end = start_index.saturating_add(limit).min(filtered.len());
    for i in start_index..end {
        if let Some(attestation) = filtered.get(i) {
            result.push_back(attestation.clone());
        }
    }
    result
}

pub fn get_attestations_by_tag(env: &Env, subject: Address, tag: String) -> Vec<String> {
    let attestation_ids = crate::storage::ChunkedIndex::get_subject_all(env, &subject);
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
    let attestation_ids = crate::storage::ChunkedIndex::get_subject_all(env, &subject);
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

/// Fetch up to `limit` non-deleted attestation IDs for `issuer`, returning the
/// slice `filtered[start..start+limit]` of the non-deleted index.
///
/// **Page-full guarantee:** the returned page contains exactly `limit` items
/// unless the end of the (non-deleted) index has been reached. Deleted IDs are
/// excluded from the index entirely, so they neither shrink the page nor cause
/// duplicates across pages.
///
/// `start` is an offset into the *filtered* result set (deleted IDs excluded),
/// not into the underlying chunked index.
fn issuer_page_filtered(env: &Env, issuer: &Address, start: u32, limit: u32) -> Vec<String> {
    let all_ids = crate::storage::ChunkedIndex::get_issuer_all(env, issuer);
    let mut filtered = Vec::new(env);
    let needed = start.saturating_add(limit);
    for id in all_ids.iter() {
        if filtered.len() >= needed as usize {
            break;
        }
        if let Ok(a) = Storage::get_attestation(env, &id) {
            if !a.deleted {
                filtered.push_back(id);
            }
        }
    }
    crate::storage::paginate(env, &filtered, start, limit)
}

pub fn get_issuer_attestations(env: &Env, issuer: Address, start: u32, limit: u32) -> Vec<String> {
    issuer_page_filtered(env, &issuer, start, limit)
}

pub fn get_issuer_attestation_count(env: &Env, issuer: Address) -> u32 {
    // Use the chunked index so revoked/deleted attestations that have been
    // removed from it are not counted (fixes the stale flat-list bug).
    crate::storage::ChunkedIndex::issuer_count(env, &issuer)
}

pub fn get_valid_claims(env: &Env, subject: Address) -> Vec<String> {
    let current_time = env.ledger().timestamp();
    let mut result = Vec::new(env);
    for attestation_id in crate::storage::ChunkedIndex::get_subject_all(env, &subject).iter() {
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
    let attestation_ids = crate::storage::ChunkedIndex::get_subject_all(env, &subject);
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
    // Issue #1570: Fixed to return distinct claim types (matching get_valid_claims().len())
    // Previously this counted total attestations, creating a mismatch with get_valid_claims
    let valid_claims = get_valid_claims(env, subject);
    valid_claims.len() as u32
}

pub fn get_global_stats(env: &Env) -> GlobalStats {
    Storage::get_global_stats(env)
}

/// Return all prior versions of an attestation's metadata, in the order they
/// were superseded. The current (latest) metadata lives on the attestation
/// itself and is NOT included here.
pub fn get_attestation_history(
    env: &Env,
    attestation_id: String,
) -> Vec<AttestationVersionSnapshot> {
    Storage::get_attestation_history(env, &attestation_id)
}

/// Return the active dispute record for an attestation, or `None` if there
/// is no open dispute.
pub fn get_dispute(env: &Env, attestation_id: String) -> Option<DisputeRecord> {
    Storage::get_dispute(env, &attestation_id)
}

/// Returns subject's attestations expiring within `within_days` days, sorted by expiration ascending.
///
/// Excludes already-revoked, already-expired, and None-expiration attestations.
/// Paginated via `start`/`limit`.
///
/// Returns an error if `within_days` exceeds `MAX_EXPIRING_WINDOW_DAYS`.
pub fn get_expiring_attestations(
    env: &Env,
    subject: Address,
    within_days: u32,
    start: u32,
    limit: u32,
) -> Result<Vec<Attestation>, Error> {
    if within_days > MAX_EXPIRING_WINDOW_DAYS {
        return Err(Error::LimitExceeded);
    }

    let current_time = env.ledger().timestamp();
    let window_end = current_time + (within_days as u64) * SECS_PER_DAY;

    let attestation_ids = crate::storage::ChunkedIndex::get_subject_all(env, &subject);
    let mut filtered = Vec::new(env);

    for id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &id) {
            if attestation.deleted || attestation.revoked {
                continue;
            }
            if let Some(exp) = attestation.expiration {
                if exp > current_time && exp <= window_end {
                    filtered.push_back(attestation);
                }
            }
        }
    }

    // Sort by expiration ascending using insertion sort (O(n²) worst case, efficient in practice)
    let len = filtered.len();
    for i in 1..len {
        // Invariant: i is in [1, len), so it is always a valid index into `filtered`.
        let key = filtered.get(i).expect("i is bounded by filtered.len()");
        let key_exp = key.expiration.unwrap_or(u64::MAX);
        let mut j = i;
        while j > 0 {
            // Invariant: j > 0 and j <= i < len, so j - 1 is always a valid index.
            let prev = filtered.get(j - 1).expect("j - 1 is bounded by filtered.len()");
            let prev_exp = prev.expiration.unwrap_or(u64::MAX);
            if prev_exp <= key_exp {
                break;
            }
            filtered.set(j, prev);
            j -= 1;
        }
        filtered.set(j, key);
    }

    let mut result = Vec::new(env);
    let len = filtered.len();
    if start < len {
        let end = (start + limit).min(len);
        for i in start..end {
            if let Some(attestation) = filtered.get(i) {
                result.push_back(attestation);
            }
        }
    }
    Ok(result)
}

/// Returns issuer's attestations expiring within `days_window` days, sorted by expiration ascending.
///
/// Excludes already-revoked, already-expired, and None-expiration attestations.
/// Paginated via `start`/`limit`.
///
/// Returns an error if `days_window` exceeds `MAX_EXPIRING_WINDOW_DAYS`.
pub fn get_issuer_expiring_attestations(
    env: &Env,
    issuer: Address,
    days_window: u32,
    start: u32,
    limit: u32,
) -> Result<Vec<Attestation>, Error> {
    if days_window > MAX_EXPIRING_WINDOW_DAYS {
        return Err(Error::LimitExceeded);
    }

    let current_time = env.ledger().timestamp();
    let window_end = current_time + (days_window as u64) * SECS_PER_DAY;

    // Use the chunked index so revoked attestations removed from it are excluded.
    let attestation_ids = crate::storage::ChunkedIndex::get_issuer_all(env, &issuer);
    let mut filtered = Vec::new(env);

    for id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &id) {
            if attestation.deleted || attestation.revoked {
                continue;
            }
            if let Some(exp) = attestation.expiration {
                if exp > current_time && exp <= window_end {
                    filtered.push_back(attestation);
                }
            }
        }
    }

    // Sort by expiration ascending using insertion sort (O(n²) worst case, efficient in practice)
    let len = filtered.len();
    for i in 1..len {
        // Invariant: i is in [1, len), so it is always a valid index into `filtered`.
        let key = filtered.get(i).expect("i is bounded by filtered.len()");
        let key_exp = key.expiration.unwrap_or(u64::MAX);
        let mut j = i;
        while j > 0 {
            // Invariant: j > 0 and j <= i < len, so j - 1 is always a valid index.
            let prev = filtered.get(j - 1).expect("j - 1 is bounded by filtered.len()");
            let prev_exp = prev.expiration.unwrap_or(u64::MAX);
            if prev_exp <= key_exp {
                break;
            }
            filtered.set(j, prev);
            j -= 1;
        }
        filtered.set(j, key);
    }

    let mut result = Vec::new(env);
    let len = filtered.len();
    if start < len {
        let end = (start + limit).min(len);
        for i in start..end {
            if let Some(attestation) = filtered.get(i) {
                result.push_back(attestation);
            }
        }
    }
    Ok(result)
}

/// Submit a dispute against an attestation. Only the subject of the
/// attestation may call this. The disputed flag is informational and does
/// not affect validity queries.
pub fn dispute_attestation(
    env: &Env,
    subject: Address,
    attestation_id: String,
    reason: String,
) -> Result<(), Error> {
    use crate::errors::Error as E;
    use crate::events::Events;
    use crate::types::{DisputeRecord, DisputeStatus, AuditAction, AuditEntry};

    subject.require_auth();

    let attestation = Storage::get_attestation(env, &attestation_id)?;
    if attestation.subject != subject {
        return Err(E::Unauthorized);
    }
    if attestation.deleted {
        return Err(E::NotFound);
    }
    
    // Issue #1567: Check if there's already an open dispute
    if let Some(existing) = Storage::get_dispute(env, &attestation_id) {
        if existing.status == DisputeStatus::Open {
            return Err(E::AlreadyDisputed);
        }
    }
    
    // Issue #1567: Reject disputes on revoked or expired attestations
    let current_time = env.ledger().timestamp();
    let status = attestation.get_status(current_time);
    if status == AttestationStatus::Revoked || status == AttestationStatus::Expired {
        return Err(E::InvalidAttestation); // or new error: CannotDisputeRevoked/Expired
    }
    
    if reason.len() > 256 {
        return Err(E::MetadataTooLong);
    }

    Validation::require_not_paused(env)?;

    let timestamp = env.ledger().timestamp();
    let record = DisputeRecord {
        attestation_id: attestation_id.clone(),
        subject: subject.clone(),
        reason: reason.clone(),
        disputed_at: timestamp,
        status: DisputeStatus::Open,
        resolved_by: None,
        resolved_at: None,
        resolution_note: None,
    };
    Storage::set_dispute(env, &attestation_id, &record);
    
    // Issue #1566: Add dispute to audit log
    let audit_entry = AuditEntry {
        action: AuditAction::Disputed,
        actor: subject.clone(),
        timestamp,
        details: Some(reason.clone()),
    };
    Storage::append_audit_entry(env, &attestation_id, &audit_entry);
    
    Events::dispute_raised(env, &attestation_id, &subject, &reason, timestamp);
    Ok(())
}

/// Export a revocation list for a given issuer and optional claim type.
///
/// This provides a compact, standards-adjacent format for external verifiers
/// to check revocation status for many attestations at once without individually
/// querying each one. Supports two formats:
/// - `RevocationListFormat::SimpleList`: Simple list of revoked attestation IDs
/// - `RevocationListFormat::Bitstring`: Reserved for bitstring encoding (returns `None` for now)
///
/// # Parameters
/// - `issuer` — the issuer address whose revocations to export
/// - `claim_type` — optional claim type filter (None = all claim types)
/// - `format` — the desired output format
/// - `start` — zero-based offset into the issuer's attestation history
/// - `limit` — maximum number of history entries to inspect
///
/// # Returns
/// A `RevocationList` containing:
/// - The issuer address
/// - The claim type filter (None if exporting all)
/// - Unix timestamp when this list was generated
/// - List of revoked attestation IDs
/// - Optional bitstring encoding when format is Bitstring
/// - Attestation count included in this page
/// - Revoked count
///
/// This is a public read and accepts both registered and former issuers.
///
pub fn export_revocation_list(
    env: &Env,
    issuer: Address,
    claim_type: Option<String>,
    format: RevocationListFormat,
    start: u32,
    limit: u32,
) -> Result<RevocationList, Error> {
    const MAX_PAGE_SIZE: u32 = 100;
    if limit == 0 || limit > MAX_PAGE_SIZE {
        return Err(Error::LimitExceeded);
    }

    let current_time = env.ledger().timestamp();

    // Use the chunked index so revoked attestations removed from it are not
    // double-counted in the total (fixes stale flat-list divergence).
    let attestation_ids = crate::storage::ChunkedIndex::get_issuer_all(env, &issuer);
    let mut revoked_ids: Vec<String> = Vec::new(env);
    let mut total_count: u64 = 0;

    for attestation_id in attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            if attestation.issuer != issuer {
                continue;
            }
            // Skip deleted records
            if attestation.deleted {
                continue;
            }

            // Apply claim type filter
            if let Some(ref ct) = claim_type {
                if attestation.claim_type != *ct {
                    continue;
                }
            }

            total_count += 1;

            // Check if revoked
            if attestation.revoked {
                revoked_ids.push_back(attestation_id.clone());
            }
        }
    }

    // Generate bitstring if requested
    let bitstring = match format {
        RevocationListFormat::Bitstring => {
            // Bitstring encoding not yet implemented
            return Err(Error::NotImplemented);
        }
        RevocationListFormat::SimpleList => None,
    };

    let revoked_count = revoked_ids.len() as u64;
    let revocation_list = RevocationList {
        issuer,
        claim_type,
        generated_at: current_time,
        revoked_attestation_ids: revoked_ids,
        bitstring,
        total_attestation_count: total_count,
        revoked_count,
    };

    Ok(revocation_list)
}
