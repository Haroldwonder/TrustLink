//! Attestation bundle creation and management.
//!
//! Bundles allow issuing multiple related attestations atomically with a shared bundle ID,
//! enabling verifiers to confirm a set of claims were issued as one coherent unit.

use soroban_sdk::{Env, Address, String, Vec, Bytes};

use crate::attestation::{store_attestation, charge_attestation_fee, check_rate_limit};
use crate::storage::Storage;
use crate::types::{
    Attestation, AttestationBundle, AttestationOrigin, AuditAction, AuditEntry, Error, Validation,
};
use crate::validation::{validate_native_expiration, validate_tags, validate_jurisdiction};

/// Maximum number of attestations allowed in a single bundle.
const MAX_BUNDLE_SIZE: u32 = 50;

/// Generate a deterministic bundle ID from issuer, subject, claim_types, and timestamp.
pub fn generate_bundle_id(
    env: &Env,
    issuer: &Address,
    subject: &Address,
    claim_types: &Vec<String>,
    timestamp: u64,
) -> String {
    let mut payload = Bytes::new(env);
    payload.append(&Bytes::from_slice(env, b"bundle:"));
    payload.append(&issuer.clone().to_xdr(env));
    payload.append(&subject.clone().to_xdr(env));
    
    // Ensure deterministic ordering by iterating claim_types in order
    for claim_type in claim_types.iter() {
        payload.append(&claim_type.clone().to_xdr(env));
    }
    
    payload.append(&timestamp.to_xdr(env));
    Attestation::hash_payload(env, &payload)
}

/// Create multiple attestations atomically as a bundle.
///
/// All attestations are created in a single transaction with Soroban's atomicity guarantee.
/// Each attestation is tagged with the same bundle_id, allowing verifiers to confirm they
/// were issued together. If any claim_type fails validation, the entire bundle fails.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `issuer` - Address of the issuer (must be authorized)
/// * `subject` - Address receiving the attestations
/// * `claim_types` - Ordered list of claim types to issue (determines bundle ID)
/// * `expiration` - Optional expiration time (applied to all attestations in bundle)
/// * `metadata` - Optional metadata (can differ per claim type via indexer if needed, or shared)
/// * `tags` - Optional tags (applied to all attestations)
///
/// # Returns
/// * `Ok(bundle_id)` - The bundle ID on success
/// * `Err(Error)` - Validation or storage error; entire bundle fails atomically
pub fn create_attestation_bundle(
    env: &Env,
    issuer: Address,
    subject: Address,
    claim_types: Vec<String>,
    expiration: Option<u64>,
    metadata: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<String, Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;
    
    // Validate bundle size
    if claim_types.is_empty() || claim_types.len() > MAX_BUNDLE_SIZE as usize {
        return Err(Error::LimitExceeded);
    }

    // Validate all claim types are registered before starting
    for claim_type in claim_types.iter() {
        Validation::validate_claim_type(claim_type)?;
        Validation::require_registered_claim_type(env, claim_type)?;
    }

    // Validate metadata and tags (once, shared for all in bundle)
    Validation::validate_metadata(env, &metadata)?;
    Validation::validate_metadata_hash_only(env, &metadata)?;
    validate_tags(&tags)?;
    validate_native_expiration(env, expiration)?;

    // Cannot create attestations from issuer to self
    if issuer == subject {
        return Err(Error::Unauthorized);
    }

    // Check whitelist mode
    if Storage::is_whitelist_mode(env, &issuer) && !Storage::is_whitelisted(env, &issuer, &subject) {
        return Err(Error::SubjectNotWhitelisted);
    }

    let timestamp = env.ledger().timestamp();
    let bundle_id = generate_bundle_id(env, &issuer, &subject, &claim_types, timestamp);

    // Check if bundle already exists (prevent duplicates)
    if Storage::has_bundle(env, &bundle_id) {
        return Err(Error::DuplicateAttestation);
    }

    let limits = Storage::get_limits(env);
    let issuer_count = Storage::get_issuer_attestations(env, &issuer).len();
    let subject_count = Storage::get_subject_attestations(env, &subject).len();

    // Verify storage limits can accommodate entire bundle
    if issuer_count.saturating_add(claim_types.len()) > limits.max_attestations_per_issuer {
        return Err(Error::LimitExceeded);
    }
    
    // Check optional per-subject limit from ContractConfig if configured
    let max_subject_limit = if let Some(config) = Storage::get_contract_config(env) {
        if let Some(max_per_subject) = config.max_attestations_per_subject {
            max_per_subject as usize
        } else {
            limits.max_attestations_per_subject as usize
        }
    } else {
        limits.max_attestations_per_subject as usize
    };
    
    if subject_count.saturating_add(claim_types.len()) > max_subject_limit {
        return Err(Error::LimitExceeded);
    }

    // Check rate limits for each claim type before creating any attestations
    for claim_type in claim_types.iter() {
        check_rate_limit(env, &issuer, claim_type)?;
    }

    // Build attestation list for the bundle
    let mut attestation_ids: Vec<String> = Vec::new(env);
    let mut attestations: Vec<Attestation> = Vec::new(env);

    // Create attestation record for each claim type
    for claim_type in claim_types.iter() {
        let attestation_id = Attestation::generate_id(env, &issuer, &subject, claim_type, timestamp);

        // Check for duplicate attestations
        if Storage::has_attestation(env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
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
            metadata: metadata.clone(),
            jurisdiction: None,
            valid_from: None,
            origin: AttestationOrigin::Native,
            source_chain: None,
            source_tx: None,
            tags: tags.clone(),
            revocation_reason: None,
            bundle_id: Some(bundle_id.clone()),
        };

        attestations.push_back(attestation);
        attestation_ids.push_back(attestation_id);
    }

    // Store all attestations and create bundle metadata
    let mut new_issuer_ids: Vec<String> = Vec::new(env);

    for attestation in attestations.iter() {
        // Store attestation with bundle_id
        store_attestation(env, &attestation);
        
        // Add to subject's attestation index
        Storage::add_subject_attestation(env, &attestation.subject, &attestation.id);
        Storage::add_valid_attestation(env, &attestation.subject, &attestation.id);
        crate::storage::ChunkedIndex::add_subject(env, &attestation.subject, &attestation.id);

        // Queue for issuer index batch write
        new_issuer_ids.push_back(attestation.id.clone());

        // Audit log
        Storage::append_audit_entry(
            env,
            &attestation.id,
            &AuditEntry {
                action: AuditAction::Created,
                actor: issuer.clone(),
                timestamp,
                details: Some(format_bundle_audit_detail(env, &bundle_id)),
            },
        );

        // Emit individual attestation_created event (for backward compatibility)
        crate::events::Events::attestation_created(env, &attestation);
    }

    // Batch write issuer index
    Storage::add_issuer_attestations_bulk(env, &issuer, &new_issuer_ids);
    crate::storage::ChunkedIndex::add_issuer_bulk(env, &issuer, &new_issuer_ids);

    // Update stats
    let bundle_size = attestation_ids.len() as u64;
    Storage::increment_issuer_stats(env, &issuer, bundle_size);
    Storage::increment_total_attestations(env, bundle_size);

    // Set issuance timestamps
    Storage::set_last_issuance_time(env, &issuer, timestamp);
    for claim_type in claim_types.iter() {
        if Storage::get_claim_type_rate_limit(env, claim_type).is_some() {
            Storage::set_last_issuance_time_by_claim_type(env, &issuer, claim_type, timestamp);
        }
    }

    // Create bundle metadata record
    let bundle = AttestationBundle {
        id: bundle_id.clone(),
        issuer: issuer.clone(),
        subject: subject.clone(),
        claim_types: claim_types.clone(),
        timestamp,
        attestation_ids: attestation_ids.clone(),
        all_valid: true,
    };

    Storage::set_bundle(env, &bundle);
    Storage::add_issuer_bundle(env, &issuer, &bundle_id);
    Storage::add_subject_bundle(env, &subject, &bundle_id);

    // Charge fees for all attestations in the bundle
    for _ in 0..bundle_size {
        charge_attestation_fee(env, &issuer)?;
    }

    // Emit bundle_created event
    crate::events::Events::bundle_created(env, &bundle);

    Ok(bundle_id)
}

/// Get all attestations belonging to a bundle.
pub fn get_bundle_attestations(
    env: &Env,
    bundle_id: &String,
) -> Result<Vec<Attestation>, Error> {
    let bundle = Storage::get_bundle(env, bundle_id)?;

    let mut attestations: Vec<Attestation> = Vec::new(env);
    for attestation_id in bundle.attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            attestations.push_back(attestation);
        }
    }

    Ok(attestations)
}

/// Check if all attestations in a bundle are still valid (not revoked).
pub fn is_bundle_valid(env: &Env, bundle_id: &String) -> Result<bool, Error> {
    let mut bundle = Storage::get_bundle(env, bundle_id)?;

    for attestation_id in bundle.attestation_ids.iter() {
        if let Ok(attestation) = Storage::get_attestation(env, &attestation_id) {
            if attestation.revoked || attestation.deleted {
                bundle.all_valid = false;
                Storage::set_bundle(env, &bundle);
                return Ok(false);
            }
        }
    }

    Ok(bundle.all_valid)
}

/// Format audit entry details to note bundle membership.
fn format_bundle_audit_detail(env: &Env, bundle_id: &String) -> String {
    let mut detail = String::from_str(env, "Created as part of bundle: ");
    detail.append(bundle_id);
    detail
}
