//! Attestation template management.
//!
//! Templates let an issuer pre-configure a named set of attestation defaults
//! (`claim_type`, optional expiration window, optional metadata).  A template
//! can then be instantiated via [`create_attestation_from_template`] with
//! optional per-call overrides, delegating final creation to the shared
//! internal attestation path.

use soroban_sdk::{Address, Env, String, Vec};

use crate::attestation::create_attestation_internal;
use crate::constants::SECS_PER_DAY;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{AttestationTemplate, Error};
use crate::validation::Validation;

/// Create or overwrite a named attestation template for the calling issuer.
///
/// Templates capture default values for `claim_type`, optional expiration
/// window, and optional metadata.  They can be instantiated later via
/// [`create_attestation_from_template`].
///
/// # Errors
/// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
/// - [`Error::InvalidClaimType`] — `claim_type` is empty or invalid.
/// - [`Error::MetadataTooLong`] — `metadata_template` exceeds 256 bytes.
pub fn create_template(
    env: &Env,
    issuer: Address,
    template_id: String,
    template: AttestationTemplate,
) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::validate_claim_type(&template.claim_type)?;
    Validation::validate_metadata(env, &template.metadata_template)?;

    Storage::set_template(env, &issuer, &template_id, &template);
    Storage::add_to_template_registry(env, &issuer, &template_id);
    Events::template_created(env, &issuer, &template_id);
    Ok(())
}

/// Instantiate an attestation from a template, with optional field overrides.
///
/// Loads the template for `(issuer, template_id)`, resolves the final
/// expiration and metadata (override wins over template default), then
/// creates and stores the attestation using the same logic as
/// [`crate::attestation::create_attestation`].
///
/// # Errors
/// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
/// - [`Error::NotFound`] — `template_id` does not exist for this issuer.
/// - [`Error::MetadataTooLong`] — `metadata_override` exceeds 256 bytes.
/// - [`Error::InvalidExpiration`] — `expiration_override` ≤ current ledger timestamp.
pub fn create_attestation_from_template(
    env: &Env,
    issuer: Address,
    template_id: String,
    subject: Address,
    expiration_override: Option<u64>,
    metadata_override: Option<String>,
) -> Result<String, Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;

    let template =
        Storage::get_template(env, &issuer, &template_id).ok_or(Error::NotFound)?;

    // Validate overrides before resolving.
    Validation::validate_metadata(env, &metadata_override)?;

    let current_time = env.ledger().timestamp();
    if let Some(ts) = expiration_override {
        if ts <= current_time {
            return Err(Error::InvalidExpiration);
        }
    }

    // Resolve final expiration: override > template default > None.
    let expiration = if let Some(ts) = expiration_override {
        Some(ts)
    } else if let Some(days) = template.default_expiration_days {
        Some(current_time + (days as u64) * SECS_PER_DAY)
    } else {
        None
    };

    // Resolve final metadata: override > template default.
    let metadata = if metadata_override.is_some() {
        metadata_override
    } else {
        template.metadata_template.clone()
    };

    // Delegate to the shared internal creation path.
    create_attestation_internal(
        env,
        issuer,
        subject,
        template.claim_type.clone(),
        expiration,
        metadata,
        None,
        None,
        None,
    )
}

/// Return the ordered list of template IDs registered for `issuer`.
///
/// Returns an empty `Vec` if the issuer has no templates.  IDs are in
/// insertion order (first-created first).
#[must_use]
pub fn list_templates(env: &Env, issuer: Address) -> Vec<String> {
    Storage::get_template_registry(env, &issuer)
}

/// Retrieve a single template by issuer and template ID.
///
/// # Errors
/// - [`Error::NotFound`] — `template_id` does not exist for this issuer.
pub fn get_template(
    env: &Env,
    issuer: Address,
    template_id: String,
) -> Result<AttestationTemplate, Error> {
    Storage::get_template(env, &issuer, &template_id).ok_or(Error::NotFound)
}
