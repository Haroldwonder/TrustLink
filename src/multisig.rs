//! Multi-signature attestation proposal logic.
//!
//! All multisig entrypoints delegate here so that `lib.rs` stays a thin
//! façade.  The canonical flow is:
//!
//! 1. An issuer calls [`propose_attestation`] — creates a [`MultiSigProposal`]
//!    and auto-signs it as proposer.
//! 2. Each required co-signer calls [`cosign_attestation`].
//! 3. Once `signers.len() >= threshold`, the proposal is finalised and the
//!    attestation is written automatically.
//! 4. The proposer may call [`cancel_multisig_proposal`] at any time before
//!    finalisation.
//! 5. Any caller may call [`list_open_proposals`] to browse unfinished proposals.
//! 6. Admins may reconfigure the TTL via [`set_multisig_ttl`].

use soroban_sdk::{Address, Env, String, Vec};

use crate::attestation::store_attestation;
use crate::constants::SECS_PER_DAY;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    Attestation, AttestationOrigin, AuditAction, AuditEntry, Error, MultiSigProposal,
};
use crate::validation::Validation;

/// Create a new multi-sig attestation proposal.
///
/// The proposer is automatically added as the first signer.  All
/// `required_signers` must be registered issuers.  Every signer must be
/// distinct and the list must not exceed `MAX_MULTISIG_SIGNERS` entries.
/// The `claim_type` is validated before the proposal is stored.
///
/// The proposal TTL is read from the on-chain configurable setting
/// (defaults to 7 days); use [`set_multisig_ttl`] to change it.
///
/// # Errors
/// - [`Error::Unauthorized`] — proposer is not a registered issuer.
/// - [`Error::ContractPaused`] — contract is paused.
/// - [`Error::InvalidClaimType`] — `claim_type` is empty, too long, or not
///   registered when claim-type registration is required.
/// - [`Error::InvalidThreshold`] — `threshold` is 0, exceeds the number of
///   required signers, or `required_signers` contains duplicates or exceeds
///   `MAX_MULTISIG_SIGNERS`.
/// - [`Error::LimitExceeded`] — `required_signers` exceeds `MAX_MULTISIG_SIGNERS`.
pub fn propose_attestation(
    env: &Env,
    proposer: Address,
    subject: Address,
    claim_type: String,
    required_signers: Vec<Address>,
    threshold: u32,
) -> Result<String, Error> {
    proposer.require_auth();
    Validation::require_issuer(env, &proposer)?;
    Validation::require_not_paused(env)?;
    crate::attestation::validate_new_attestation_common(env, &proposer, &subject, &claim_type)?;

    // Validate claim type format and registration.
    Validation::validate_claim_type(&claim_type)?;
    Validation::require_registered_claim_type(env, &claim_type)?;

    // Enforce upper bound on the signer list to prevent entry-size and
    // compute-cost issues in cosign_attestation.
    let signer_count = required_signers.len();
    if signer_count > crate::constants::MAX_MULTISIG_SIGNERS {
        return Err(Error::LimitExceeded);
    }

    // Reject duplicate signers — a proposal with duplicates can never reach
    // its threshold via distinct co-signers and inflates signer_count.
    for i in 0..signer_count {
        let a = required_signers.get(i).unwrap();
        for j in (i + 1)..signer_count {
            let b = required_signers.get(j).unwrap();
            if a == b {
                return Err(Error::InvalidThreshold);
            }
        }
    }

    // Validate all required signers are registered issuers.
    for signer in required_signers.iter() {
        Validation::require_issuer(env, &signer)?;
    }

    if threshold == 0 || threshold > signer_count {
        return Err(Error::InvalidThreshold);
    }

    let timestamp = env.ledger().timestamp();
    let proposal_id =
        MultiSigProposal::generate_id(env, &proposer, &subject, &claim_type, timestamp);

    // Proposer auto-signs on creation.
    let mut signers = Vec::new(env);
    signers.push_back(proposer.clone());

    // TTL comes from the on-chain configurable setting.
    let ttl_days = Storage::get_multisig_ttl(env);
    let ttl_secs = (ttl_days as u64) * SECS_PER_DAY;

    let proposal = MultiSigProposal {
        id: proposal_id.clone(),
        proposer: proposer.clone(),
        subject: subject.clone(),
        claim_type,
        required_signers,
        threshold,
        signers,
        created_at: timestamp,
        expires_at: timestamp + ttl_secs,
        finalized: false,
        cancelled: false,
    };

    Storage::set_multisig_proposal(env, &proposal);
    Storage::add_to_proposal_index(env, &subject, &proposal_id);
    Events::multisig_proposed(env, &proposal_id, &proposer, &subject, threshold);
    Ok(proposal_id)
}

/// Add a co-signature to an existing proposal.
///
/// When the signature count reaches `threshold`, the proposal is finalised
/// and the attestation is created automatically.
///
/// # Errors
/// - [`Error::Unauthorized`] — `issuer` is not a registered issuer.
/// - [`Error::NotFound`] — proposal does not exist.
/// - [`Error::ProposalFinalized`] — proposal is already finalised.
/// - [`Error::ProposalExpired`] — proposal has expired.
/// - [`Error::NotRequiredSigner`] — `issuer` is not in `required_signers`.
/// - [`Error::AlreadySigned`] — `issuer` has already signed.
pub fn cosign_attestation(env: &Env, issuer: Address, proposal_id: String) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;

    let mut proposal = Storage::get_multisig_proposal(env, &proposal_id)?;
    if proposal.cancelled {
        return Err(Error::ProposalCancelled);
    }
    if proposal.finalized {
        return Err(Error::ProposalFinalized);
    }
    let current_time = env.ledger().timestamp();
    if current_time >= proposal.expires_at {
        return Err(Error::ProposalExpired);
    }

    let mut is_required = false;
    for signer in proposal.required_signers.iter() {
        if signer == issuer {
            is_required = true;
            break;
        }
    }
    if !is_required {
        return Err(Error::NotRequiredSigner);
    }

    for signer in proposal.signers.iter() {
        if signer == issuer {
            return Err(Error::AlreadySigned);
        }
    }

    proposal.signers.push_back(issuer.clone());
    let sig_count = proposal.signers.len();
    Events::multisig_cosigned(env, &proposal_id, &issuer, sig_count, proposal.threshold);

    if sig_count >= proposal.threshold {
        // Re-check that the proposer is still a registered issuer at finalization
        // time; they may have been deregistered between proposal creation and now.
        Validation::require_issuer(env, &proposal.proposer)?;

        proposal.finalized = true;
        Storage::set_multisig_proposal(env, &proposal);

        // Use the current ledger timestamp so the attestation is not backdated
        // to proposal creation (which could be days earlier).
        let attestation_id = Attestation::generate_id(
            env,
            &proposal.proposer,
            &proposal.subject,
            &proposal.claim_type,
            current_time,
        );

        // Guard against overwriting a direct attestation whose ID collides.
        if Storage::has_attestation(env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: proposal.proposer.clone(),
            subject: proposal.subject.clone(),
            claim_type: proposal.claim_type.clone(),
            timestamp: current_time,
            expiration: None,
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

        store_attestation(env, &attestation);
        // Keep the attestation and its indexes live until expiration.
        Storage::extend_attestation_ttl_for_expiry(env, &attestation);

        // Write audit entry (matches direct-creation path).
        Storage::append_audit_entry(env, &attestation_id, &AuditEntry {
            action: AuditAction::Created,
            actor: proposal.proposer.clone(),
            timestamp: current_time,
            details: None,
        });

        Events::attestation_created(env, &attestation);
        Events::multisig_activated(env, &proposal_id, &attestation_id);
    } else {
        Storage::set_multisig_proposal(env, &proposal);
    }
    Ok(())
}

/// Fetch a proposal by ID.
///
/// # Errors
/// - [`Error::NotFound`] — proposal does not exist.
pub fn get_multisig_proposal(env: &Env, proposal_id: String) -> Result<MultiSigProposal, Error> {
    Storage::get_multisig_proposal(env, &proposal_id)
}

/// Return the current multisig TTL in days.
#[must_use]
pub fn get_multisig_ttl(env: &Env) -> u32 {
    Storage::get_multisig_ttl(env)
}

/// Set the multisig TTL in days.  Admin-only.
///
/// # Errors
/// - [`Error::Unauthorized`] — `admin` is not a registered admin.
pub fn set_multisig_ttl(env: &Env, admin: Address, days: u32) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(env, &admin)?;
    Storage::set_multisig_ttl(env, days);
    Ok(())
}

/// Cancel an unfinalized multisig proposal.
///
/// Only the original proposer may cancel.  Cancelled proposals are excluded
/// from future [`cosign_attestation`] calls (returns [`Error::ProposalExpired`]).
///
/// # Errors
/// - [`Error::NotFound`] — proposal does not exist.
/// - [`Error::Unauthorized`] — caller is not the original proposer.
/// - [`Error::ProposalFinalized`] — proposal has already been finalised.
/// - [`Error::ProposalCancelled`] — proposal is already cancelled.
pub fn cancel_multisig_proposal(
    env: &Env,
    proposer: Address,
    proposal_id: String,
) -> Result<(), Error> {
    proposer.require_auth();
    Validation::require_not_paused(env)?;

    let mut proposal = Storage::get_multisig_proposal(env, &proposal_id)?;

    if proposal.proposer != proposer {
        return Err(Error::Unauthorized);
    }
    if proposal.finalized {
        return Err(Error::ProposalFinalized);
    }
    if proposal.cancelled {
        return Err(Error::ProposalCancelled);
    }

    proposal.cancelled = true;
    Storage::set_multisig_proposal(env, &proposal);
    Events::multisig_cancelled(env, &proposal_id, &proposer);
    Ok(())
}

/// List open (unfinalized, unexpired, uncancelled) proposals for a subject.
///
/// Returns a paginated slice of proposals where `finalized == false`,
/// `cancelled == false`, and `expires_at > now`.
///
/// # Parameters
/// - `subject` — the subject address whose proposals to query.
/// - `start` — zero-based offset into the filtered list.
/// - `limit` — maximum number of proposals to return.
#[must_use]
pub fn list_open_proposals(
    env: &Env,
    subject: Address,
    start: u32,
    limit: u32,
) -> Vec<MultiSigProposal> {
    let current_time = env.ledger().timestamp();
    let index = Storage::get_proposal_index(env, &subject);
    let mut open: Vec<MultiSigProposal> = Vec::new(env);

    for proposal_id in index.iter() {
        if let Ok(proposal) = Storage::get_multisig_proposal(env, &proposal_id) {
            if !proposal.finalized && !proposal.cancelled && current_time < proposal.expires_at {
                open.push_back(proposal);
            }
        }
    }

    let total = open.len();
    if start >= total {
        return Vec::new(env);
    }
    let end = (start + limit).min(total);
    let mut result: Vec<MultiSigProposal> = Vec::new(env);
    for i in start..end {
        if let Some(p) = open.get(i) {
            result.push_back(p);
        }
    }
    result
}
