use soroban_sdk::{Address, Env, String, Vec};

use crate::attestation::{store_attestation, validate_native_expiration};
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    Attestation, AttestationOrigin, AttestationRequest, Error, RequestStatus,
    ATTESTATION_REQUEST_TTL_SECS,
};
use crate::validation::Validation;

pub fn request_attestation(
    env: &Env,
    subject: Address,
    issuer: Address,
    claim_type: String,
) -> Result<String, Error> {
    subject.require_auth();
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &issuer)?;
    Validation::validate_claim_type(&claim_type)?;

    let current_time = env.ledger().timestamp();

    // Check if there's already a pending, unexpired request for the same (subject, issuer, claim_type)
    let pending_ids = Storage::get_pending_request_ids(env, &issuer);
    for existing_id in pending_ids.iter() {
        if let Ok(existing_req) = Storage::get_request(env, &existing_id) {
            if existing_req.subject == subject
                && existing_req.claim_type == claim_type
                && existing_req.status == RequestStatus::Pending
                && current_time < existing_req.expires_at {
                return Err(Error::DuplicateRequest);
            }
        }
    }

    let request_id = AttestationRequest::generate_id(env, &subject, &issuer, &claim_type, current_time);
    let expires_at = current_time + ATTESTATION_REQUEST_TTL_SECS;
    let request = AttestationRequest {
        id: request_id.clone(),
        subject: subject.clone(),
        issuer: issuer.clone(),
        claim_type: claim_type.clone(),
        timestamp: current_time,
        expires_at,
        status: RequestStatus::Pending,
        rejection_reason: None,
    };

    Storage::set_request(env, &request);
    Storage::add_pending_request(env, &issuer, &request_id);
    Events::attestation_requested(env, &request_id, &subject, &issuer, &claim_type, expires_at);

    Ok(request_id)
}

pub fn fulfill_request(
    env: &Env,
    issuer: Address,
    request_id: String,
    expiration: Option<u64>,
) -> Result<String, Error> {
    issuer.require_auth();
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &issuer)?;

    let mut request = Storage::get_request(env, &request_id)?;

    if request.issuer != issuer {
        return Err(Error::Unauthorized);
    }

    match request.status {
        RequestStatus::Fulfilled | RequestStatus::Rejected | RequestStatus::Cancelled | RequestStatus::Expired => return Err(Error::RequestAlreadyProcessed),
        RequestStatus::Pending => {}
    }

    let current_time = env.ledger().timestamp();
    if current_time >= request.expires_at {
        return Err(Error::RequestExpired);
    }

    crate::attestation::validate_new_attestation_common(env, &issuer, &request.subject, &request.claim_type)?;
    validate_native_expiration(env, expiration)?;

    let attestation_id = crate::attestation::create_attestation_internal(
        env,
        issuer.clone(),
        request.subject.clone(),
        request.claim_type.clone(),
        expiration,
        None,
        None,
        None,
        None,
    )?;

    Storage::append_audit_entry(
        env,
        &attestation_id,
        &crate::types::AuditEntry {
            action: crate::types::AuditAction::Created,
            actor: issuer.clone(),
            timestamp: current_time,
            details: Some(request_id.clone()),
        },
    );

    request.status = RequestStatus::Fulfilled;
    Storage::set_request(env, &request);
    Storage::remove_pending_request(env, &issuer, &request_id);

    Events::request_fulfilled(env, &request_id, &issuer, &attestation_id);

    Ok(attestation_id)
}

pub fn reject_request(
    env: &Env,
    issuer: Address,
    request_id: String,
    reason: Option<String>,
) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_not_paused(env)?;
    Validation::require_issuer(env, &issuer)?;
    crate::attestation::validate_reason(&reason)?;

    let mut request = Storage::get_request(env, &request_id)?;

    if request.issuer != issuer {
        return Err(Error::Unauthorized);
    }

    match request.status {
        RequestStatus::Fulfilled | RequestStatus::Rejected | RequestStatus::Cancelled | RequestStatus::Expired => {
            return Err(Error::RequestAlreadyProcessed)
        }
        RequestStatus::Pending => {}
    }

    let current_time = env.ledger().timestamp();
    if current_time >= request.expires_at {
        return Err(Error::RequestExpired);
    }

    request.status = RequestStatus::Rejected;
    request.rejection_reason = reason.clone();
    Storage::set_request(env, &request);
    Storage::remove_pending_request(env, &issuer, &request_id);

    Events::request_rejected(env, &request_id, &issuer, &reason);

    Ok(())
}

pub fn cancel_request(env: &Env, subject: Address, request_id: String) -> Result<(), Error> {
    subject.require_auth();
    Validation::require_not_paused(env)?;

    let mut request = Storage::get_request(env, &request_id)?;

    if request.subject != subject {
        return Err(Error::Unauthorized);
    }

    match request.status {
        RequestStatus::Fulfilled | RequestStatus::Rejected | RequestStatus::Cancelled | RequestStatus::Expired => {
            return Err(Error::RequestAlreadyProcessed)
        }
        RequestStatus::Pending => {}
    }

    request.status = RequestStatus::Cancelled;
    Storage::set_request(env, &request);
    Storage::remove_pending_request(env, &request.issuer, &request_id);

    Events::request_cancelled(env, &request_id, &subject);

    Ok(())
}

pub fn get_pending_requests(env: &Env, issuer: Address, start: u32, limit: u32) -> Vec<AttestationRequest> {
    let current_time = env.ledger().timestamp();
    let all_ids = Storage::get_pending_request_ids(env, &issuer);
    let mut pending = Vec::new(env);

    for id in all_ids.iter() {
        if let Ok(req) = Storage::get_request(env, &id) {
            if req.status == RequestStatus::Pending && current_time < req.expires_at {
                pending.push_back(req);
            }
        }
    }

    let total = pending.len();
    let start = start.min(total);
    let end = (start + limit).min(total);
    let mut result = Vec::new(env);
    let mut i = start;
    while i < end {
        if let Some(req) = pending.get(i) {
            result.push_back(req);
        }
        i += 1;
    }
    result
}

pub fn get_request(env: &Env, request_id: String) -> Result<AttestationRequest, Error> {
    let mut req = Storage::get_request(env, &request_id)?;
    // Compute expiry lazily on read: a Pending request whose deadline has
    // passed is surfaced as Expired without requiring cleanup to have run.
    if req.status == RequestStatus::Pending && env.ledger().timestamp() >= req.expires_at {
        req.status = RequestStatus::Expired;
    }
    Ok(req)
}

/// Remove expired pending requests from an issuer's pending list, persist
/// `RequestStatus::Expired` on each expired record, and emit a `req_exp`
/// event per expired request.
///
/// # Errors
/// - [`Error::Unauthorized`] — caller is not a registered issuer.
pub fn cleanup_expired_requests(env: &Env, issuer: Address) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Storage::cleanup_expired_requests(env, &issuer);
    Ok(())
}
