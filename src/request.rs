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

    let timestamp = env.ledger().timestamp();
    let request_id = AttestationRequest::generate_id(env, &subject, &issuer, &claim_type, timestamp);

    if Storage::get_request(env, &request_id).is_ok() {
        return Err(Error::DuplicateRequest);
    }

    let expires_at = timestamp + ATTESTATION_REQUEST_TTL_SECS;
    let request = AttestationRequest {
        id: request_id.clone(),
        subject: subject.clone(),
        issuer: issuer.clone(),
        claim_type: claim_type.clone(),
        timestamp,
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
        RequestStatus::Fulfilled | RequestStatus::Rejected => return Err(Error::RequestAlreadyProcessed),
        RequestStatus::Pending => {}
    }

    let current_time = env.ledger().timestamp();
    if current_time >= request.expires_at {
        return Err(Error::RequestExpired);
    }

    validate_native_expiration(env, expiration)?;

    let attestation_id = Attestation::generate_id(env, &issuer, &request.subject, &request.claim_type, current_time);
    if Storage::has_attestation(env, &attestation_id) {
        return Err(Error::DuplicateAttestation);
    }

    let limits = Storage::get_limits(env);
    if Storage::get_issuer_attestations(env, &issuer).len() >= limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }
    if Storage::get_subject_attestations(env, &request.subject).len() >= limits.max_attestations_per_subject {
        return Err(Error::LimitExceeded);
    }

    let attestation = Attestation {
        id: attestation_id.clone(),
        issuer: issuer.clone(),
        subject: request.subject.clone(),
        claim_type: request.claim_type.clone(),
        timestamp: current_time,
        expiration,
        revoked: false,
        metadata: None,
        jurisdiction: None,
        valid_from: None,
        origin: AttestationOrigin::Native,
        source_chain: None,
        source_tx: None,
        tags: None,
        revocation_reason: None,
        deleted: false,
    };

    store_attestation(env, &attestation);
    Events::attestation_created(env, &attestation);

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
        RequestStatus::Fulfilled | RequestStatus::Rejected => return Err(Error::RequestAlreadyProcessed),
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
    Storage::get_request(env, &request_id)
}
