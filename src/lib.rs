#![no_std]

mod errors;
mod events;
mod storage;
pub mod types;
mod validation;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env, String, Vec};

use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    Attestation, AttestationStatus, AuditAction, AuditEntry, ClaimTypeInfo, ContractConfig,
    ContractMetadata, Endorsement, Error, FeeConfig, GlobalStats, HealthStatus, IssuerMetadata,
    IssuerStats, IssuerTier, MultiSigProposal, RateLimitConfig, TtlConfig, MULTISIG_PROPOSAL_TTL_SECS,
};
use crate::validation::Validation;

// Seconds in one day.
const SECS_PER_DAY: u64 = 86_400;

/// Minimal interface expected on a registered callback contract.
/// The callback receives the subject, attestation ID, and expiration timestamp.
mod callback {
    use soroban_sdk::{contractclient, Address, Env, String};

    #[contractclient(name = "ExpirationCallbackClient")]
    #[allow(dead_code)]
    pub trait ExpirationCallback {
        fn notify_expiring(env: Env, subject: Address, attestation_id: String, expiration: u64);
    }
}

use callback::ExpirationCallbackClient;

fn validate_metadata(metadata: &Option<String>) -> Result<(), Error> {
    if let Some(value) = metadata {
        if value.len() > 256 {
            return Err(Error::MetadataTooLong);
        }
    }
    Ok(())
}

fn validate_reason(reason: &Option<String>) -> Result<(), Error> {
    if let Some(r) = reason {
        if r.len() > 128 {
            return Err(Error::ReasonTooLong);
        }
    }
    Ok(())
}

fn validate_tags(tags: &Option<Vec<String>>) -> Result<(), Error> {
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

fn validate_native_expiration(env: &Env, expiration: Option<u64>) -> Result<(), Error> {
    if let Some(value) = expiration {
        if value <= env.ledger().timestamp() {
            return Err(Error::InvalidExpiration);
        }
    }
    Ok(())
}

fn validate_import_timestamps(
    env: &Env,
    timestamp: u64,
    expiration: Option<u64>,
) -> Result<(), Error> {
    if timestamp > env.ledger().timestamp() {
        return Err(Error::InvalidTimestamp);
    }

    if let Some(value) = expiration {
        if value <= timestamp {
            return Err(Error::InvalidExpiration);
        }
    }

    Ok(())
}

fn validate_fee_config(fee: i128, fee_token: &Option<Address>) -> Result<(), Error> {
    if fee < 0 {
        return Err(Error::InvalidFee);
    }

    if fee > 0 && fee_token.is_none() {
        return Err(Error::FeeTokenRequired);
    }

    Ok(())
}

fn check_rate_limit(env: &Env, issuer: &Address) -> Result<(), Error> {
    if let Some(config) = Storage::get_rate_limit_config(env) {
        if config.min_issuance_interval == 0 {
            return Ok(());
        }

        let current_time = env.ledger().timestamp();
        if let Some(last_issuance) = Storage::get_last_issuance_time(env, issuer) {
            let elapsed = current_time.saturating_sub(last_issuance);
            if elapsed < config.min_issuance_interval {
                return Err(Error::RateLimited);
            }
        }
    }

    Ok(())
}

fn default_fee_config(admin: &Address) -> FeeConfig {
    FeeConfig {
        attestation_fee: 0,
        fee_collector: admin.clone(),
        fee_token: None,
    }
}

fn load_fee_config(env: &Env) -> Result<FeeConfig, Error> {
    Storage::get_fee_config(env).ok_or(Error::NotInitialized)
}

fn charge_attestation_fee(env: &Env, issuer: &Address) -> Result<(), Error> {
    let fee_config = load_fee_config(env)?;

    if fee_config.attestation_fee < 0 {
        return Err(Error::InvalidFee);
    }

    if fee_config.attestation_fee == 0 {
        return Ok(());
    }

    let fee_token = fee_config.fee_token.ok_or(Error::FeeTokenRequired)?;
    TokenClient::new(env, &fee_token).transfer(
        issuer,
        &fee_config.fee_collector,
        &fee_config.attestation_fee,
    );

    Ok(())
}

fn store_attestation(env: &Env, attestation: &Attestation) {
    Storage::set_attestation(env, attestation);
    Storage::add_subject_attestation(env, &attestation.subject, &attestation.id);
    Storage::add_issuer_attestation(env, &attestation.issuer, &attestation.id);

    // Increment total_issued counter atomically with the attestation write.
    let mut stats = Storage::get_issuer_stats(env, &attestation.issuer);
    stats.total_issued += 1;
    Storage::set_issuer_stats(env, &attestation.issuer, &stats);
}

/// Fire the expiration hook for `subject` if one is registered and the
/// attestation is inside the notification window. Failures are silently
/// swallowed so the main flow is never interrupted.
fn maybe_trigger_expiration_hook(
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
        // Best-effort cross-contract call — ignore any panic/error.
        let client = ExpirationCallbackClient::new(env, &hook.callback_contract);
        let _ = client.try_notify_expiring(subject, attestation_id, &expiration);
    }
}

#[contract]
pub struct TrustLinkContract;

#[contractimpl]
impl TrustLinkContract {
    pub fn initialize(env: Env, admin: Address, ttl_days: Option<u32>) -> Result<(), Error> {
        admin.require_auth();

        if Storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        Storage::set_admin(&env, &admin);
        Storage::set_version(&env, &String::from_str(&env, "1.0.0"));
        Storage::set_fee_config(&env, &default_fee_config(&admin));

        // Set TTL configuration if provided
        if let Some(days) = ttl_days {
            Storage::set_ttl_config(&env, &TtlConfig { ttl_days: days });
        } else {
            Storage::set_ttl_config(&env, &TtlConfig { ttl_days: 30 });
        }

        Events::admin_initialized(&env, &admin, env.ledger().timestamp());
        Ok(())
    }

    pub fn transfer_admin(
        env: Env,
        current_admin: Address,
        new_admin: Address,
    ) -> Result<(), Error> {
        current_admin.require_auth();
        Validation::require_admin(&env, &current_admin)?;
        Storage::set_admin(&env, &new_admin);
        Events::admin_transferred(&env, &current_admin, &new_admin);
        Ok(())
    }

    pub fn register_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::add_issuer(&env, &issuer);
        Storage::increment_total_issuers(&env);
        Events::issuer_registered(&env, &issuer, &admin, env.ledger().timestamp());
        Ok(())
    }

    pub fn remove_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::remove_issuer(&env, &issuer);
        Storage::decrement_total_issuers(&env);
        Events::issuer_removed(&env, &issuer, &admin, env.ledger().timestamp());
        Ok(())
    }

    /// Update the trust tier of an already-registered issuer.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — `issuer` is not registered.
    pub fn update_issuer_tier(
        env: Env,
        admin: Address,
        issuer: Address,
        tier: IssuerTier,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Validation::require_issuer(&env, &issuer)?;
        Storage::set_issuer_tier(&env, &issuer, &tier);
        Events::issuer_tier_updated(&env, &issuer, &tier);
        Ok(())
    }

    /// Return the trust tier of `issuer`, or `None` if not registered.
    pub fn get_issuer_tier(env: Env, issuer: Address) -> Option<IssuerTier> {
        Storage::get_issuer_tier(&env, &issuer)
    }

    /// Return `true` if `subject` holds a valid `claim_type` attestation issued
    /// by an issuer whose tier is >= `min_tier`.
    pub fn has_valid_claim_from_tier(
        env: Env,
        subject: Address,
        claim_type: String,
        min_tier: IssuerTier,
    ) -> bool {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        let min_rank = min_tier.rank();

        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if attestation.deleted || attestation.claim_type != claim_type {
                    continue;
                }
                if attestation.get_status(current_time) != AttestationStatus::Valid {
                    continue;
                }
                if let Some(tier) = Storage::get_issuer_tier(&env, &attestation.issuer) {
                    if tier.rank() >= min_rank {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn register_bridge(
        env: Env,
        admin: Address,
        bridge_contract: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::add_bridge(&env, &bridge_contract);
        Ok(())
    }

    pub fn set_fee(
        env: Env,
        admin: Address,
        fee: i128,
        collector: Address,
        fee_token: Option<Address>,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        validate_fee_config(fee, &fee_token)?;

        Storage::set_fee_config(
            &env,
            &FeeConfig {
                attestation_fee: fee,
                fee_collector: collector,
                fee_token,
            },
        );

        Ok(())
    }

    /// Configure the minimum issuance interval (rate limit) for attestation creation.
    ///
    /// When `min_issuance_interval` is 0 (default), rate limiting is disabled.
    /// When > 0, issuers must wait at least that many seconds between attestation creations.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — caller is not the admin.
    pub fn set_rate_limit(
        env: Env,
        admin: Address,
        min_issuance_interval: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;

        Storage::set_rate_limit_config(
            &env,
            &RateLimitConfig {
                min_issuance_interval,
            },
        );

        Ok(())
    }

    /// Retrieve the current rate limit configuration, or `None` if not set.
    pub fn get_rate_limit(env: Env) -> Option<RateLimitConfig> {
        Storage::get_rate_limit_config(&env)
    }

    /// Pause the contract, disabling all attestation write operations.
    ///
    /// Read-only functions (`has_valid_claim`, `get_attestation`, etc.) remain
    /// available while paused so that integrators can still verify existing
    /// attestations during an incident.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — caller is not the admin.
    pub fn pause(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::set_paused(&env, true);
        Events::contract_paused(&env, &admin, env.ledger().timestamp());
        Ok(())
    }

    /// Unpause the contract, re-enabling attestation write operations.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — caller is not the admin.
    pub fn unpause(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::set_paused(&env, false);
        Events::contract_unpaused(&env, &admin, env.ledger().timestamp());
        Ok(())
    }

    /// Return `true` if the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        Storage::is_paused(&env)
    }

    /// Creates a native attestation from a registered issuer about a subject.
    ///
    /// `issuer` and `subject` must be different addresses; self-attestation is
    /// rejected with [`Error::Unauthorized`] to prevent self-certification.
    pub fn create_attestation(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<String, Error> {
        issuer.require_auth();
        Validation::require_not_paused(&env)?;
        Validation::require_issuer(&env, &issuer)?;
        validate_metadata(&metadata)?;
        validate_tags(&tags)?;
        validate_native_expiration(&env, expiration)?;
        check_rate_limit(&env, &issuer)?;

        if issuer == subject {
            return Err(Error::Unauthorized);
        }

        let timestamp = env.ledger().timestamp();
        let attestation_id =
            Attestation::generate_id(&env, &issuer, &subject, &claim_type, timestamp);

        if Storage::has_attestation(&env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }

        charge_attestation_fee(&env, &issuer)?;

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: issuer.clone(),
            subject,
            claim_type,
            timestamp,
            expiration,
            revoked: false,
            metadata,
            valid_from: None,
            imported: false,
            bridged: false,
            source_chain: None,
            source_tx: None,
            tags,
            revocation_reason: None,
            deleted: false,
        };

        store_attestation(&env, &attestation);
        Storage::increment_total_attestations(&env, 1);
        Storage::set_last_issuance_time(&env, &issuer, timestamp);
        Events::attestation_created(&env, &attestation);
        Storage::append_audit_entry(
            &env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Created,
                actor: attestation.issuer.clone(),
                timestamp,
                details: None,
            },
        );
        Ok(attestation_id)
    }

    pub fn import_attestation(
        env: Env,
        admin: Address,
        issuer: Address,
        subject: Address,
        claim_type: String,
        timestamp: u64,
        expiration: Option<u64>,
    ) -> Result<String, Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Validation::require_issuer(&env, &issuer)?;
        validate_import_timestamps(&env, timestamp, expiration)?;

        let attestation_id =
            Attestation::generate_id(&env, &issuer, &subject, &claim_type, timestamp);

        if Storage::has_attestation(&env, &attestation_id) {
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
            metadata: None,
            valid_from: None,
            imported: true,
            bridged: false,
            source_chain: None,
            source_tx: None,
            tags: None,
            revocation_reason: None,
            deleted: false,
        };

        store_attestation(&env, &attestation);
        Storage::increment_total_attestations(&env, 1);
        Events::attestation_imported(&env, &attestation);
        Storage::append_audit_entry(
            &env,
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
        env: Env,
        bridge: Address,
        subject: Address,
        claim_type: String,
        source_chain: String,
        source_tx: String,
    ) -> Result<String, Error> {
        bridge.require_auth();
        Validation::require_bridge(&env, &bridge)?;

        let timestamp = env.ledger().timestamp();
        let attestation_id = Attestation::generate_bridge_id(
            &env,
            &bridge,
            &subject,
            &claim_type,
            &source_chain,
            &source_tx,
            timestamp,
        );

        if Storage::has_attestation(&env, &attestation_id) {
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
            metadata: None,
            valid_from: None,
            imported: false,
            bridged: true,
            source_chain: Some(source_chain),
            source_tx: Some(source_tx),
            tags: None,
            revocation_reason: None,
            deleted: false,
        };

        store_attestation(&env, &attestation);
        Storage::increment_total_attestations(&env, 1);
        Events::attestation_bridged(&env, &attestation);
        Storage::append_audit_entry(
            &env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Created,
                actor: attestation.issuer.clone(),
                timestamp,
                details: None,
            },
        );
        Ok(attestation_id)
    }

    pub fn create_attestations_batch(
        env: Env,
        issuer: Address,
        subjects: Vec<Address>,
        claim_type: String,
        expiration: Option<u64>,
    ) -> Result<Vec<String>, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        validate_native_expiration(&env, expiration)?;
        check_rate_limit(&env, &issuer)?;

        let timestamp = env.ledger().timestamp();
        let mut ids = Vec::new(&env);

        for subject in subjects.iter() {
            let attestation_id =
                Attestation::generate_id(&env, &issuer, &subject, &claim_type, timestamp);

            if Storage::has_attestation(&env, &attestation_id) {
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
                metadata: None,
                valid_from: None,
                imported: false,
                bridged: false,
                source_chain: None,
                source_tx: None,
                tags: None,
                revocation_reason: None,
                deleted: false,
            };

            store_attestation(&env, &attestation);
            Events::attestation_created(&env, &attestation);
            Storage::append_audit_entry(
                &env,
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

        Storage::increment_total_attestations(&env, ids.len() as u64);
        Storage::set_last_issuance_time(&env, &issuer, timestamp);
        Ok(ids)
    }

    pub fn revoke_attestation(
        env: Env,
        issuer: Address,
        attestation_id: String,
        reason: Option<String>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_not_paused(&env)?;
        Validation::require_issuer(&env, &issuer)?;
        validate_reason(&reason)?;
        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;

        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }

        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        attestation.revoked = true;
        attestation.revocation_reason = reason.clone();
        Storage::set_attestation(&env, &attestation);
        Events::attestation_revoked(&env, &attestation_id, &issuer, &reason);
        Storage::append_audit_entry(
            &env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Revoked,
                actor: issuer.clone(),
                timestamp: env.ledger().timestamp(),
                details: reason.clone(),
            },
        );
        Storage::increment_total_revocations(&env, 1);
        Ok(())
    }

    pub fn revoke_attestations_batch(
        env: Env,
        issuer: Address,
        attestation_ids: Vec<String>,
        reason: Option<String>,
    ) -> Result<u32, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        validate_reason(&reason)?;

        let mut count = 0;
        for attestation_id in attestation_ids.iter() {
            let mut attestation = Storage::get_attestation(&env, &attestation_id)?;

            if attestation.issuer != issuer {
                return Err(Error::Unauthorized);
            }

            if attestation.revoked {
                return Err(Error::AlreadyRevoked);
            }

            attestation.revoked = true;
            attestation.revocation_reason = reason.clone();
            Storage::set_attestation(&env, &attestation);
            Events::attestation_revoked(&env, &attestation_id, &issuer, &reason);
            Storage::append_audit_entry(
                &env,
                &attestation_id,
                &AuditEntry {
                    action: AuditAction::Revoked,
                    actor: issuer.clone(),
                    timestamp: env.ledger().timestamp(),
                    details: reason.clone(),
                },
            );
            count += 1;
        }

        Storage::increment_total_revocations(&env, count as u64);
        Ok(count)
    }

    pub fn renew_attestation(
        env: Env,
        issuer: Address,
        attestation_id: String,
        new_expiration: Option<u64>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        validate_native_expiration(&env, new_expiration)?;

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;
        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        attestation.expiration = new_expiration;
        Storage::set_attestation(&env, &attestation);
        Events::attestation_renewed(&env, &attestation_id, &issuer, new_expiration);
        Storage::append_audit_entry(
            &env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Renewed,
                actor: issuer.clone(),
                timestamp: env.ledger().timestamp(),
                details: None,
            },
        );
        Ok(())
    }

    pub fn update_expiration(
        env: Env,
        issuer: Address,
        attestation_id: String,
        new_expiration: Option<u64>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;

        if let Some(value) = new_expiration {
            if value <= env.ledger().timestamp() {
                return Err(Error::InvalidExpiration);
            }
        }

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;
        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        attestation.expiration = new_expiration;
        Storage::set_attestation(&env, &attestation);
        Events::attestation_updated(&env, &attestation_id, &issuer, new_expiration);
        Storage::append_audit_entry(
            &env,
            &attestation_id,
            &AuditEntry {
                action: AuditAction::Updated,
                actor: issuer.clone(),
                timestamp: env.ledger().timestamp(),
                details: None,
            },
        );
        Ok(())
    }

    pub fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();

        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if attestation.deleted || attestation.claim_type != claim_type {
                    continue;
                }
                match attestation.get_status(current_time) {
                    AttestationStatus::Valid => {
                        // Fire expiration hook if the attestation has an
                        // expiration and is inside the notification window.
                        if let Some(exp) = attestation.expiration {
                            maybe_trigger_expiration_hook(
                                &env,
                                &subject,
                                &attestation_id,
                                exp,
                                current_time,
                            );
                        }
                        return true;
                    }
                    AttestationStatus::Expired => {
                        Events::attestation_expired(&env, &attestation_id, &subject);
                    }
                    AttestationStatus::Revoked | AttestationStatus::Pending => {}
                }
            }
        }

        false
    }

    pub fn has_valid_claim_from_issuer(
        env: Env,
        subject: Address,
        claim_type: String,
        issuer: Address,
    ) -> bool {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();

        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if attestation.deleted {
                    continue;
                }
                if attestation.claim_type == claim_type && attestation.issuer == issuer {
                    match attestation.get_status(current_time) {
                        AttestationStatus::Valid => return true,
                        AttestationStatus::Expired => {
                            Events::attestation_expired(&env, &attestation_id, &subject);
                        }
                        AttestationStatus::Revoked | AttestationStatus::Pending => {}
                    }
                }
            }
        }

        false
    }

    pub fn has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool {
        if claim_types.is_empty() {
            return false;
        }

        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();

        for claim_type in claim_types.iter() {
            for attestation_id in attestation_ids.iter() {
                if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
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

    pub fn has_all_claims(env: Env, subject: Address, claim_types: Vec<String>) -> bool {
        if claim_types.is_empty() {
            return true;
        }

        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();

        'claims: for claim_type in claim_types.iter() {
            for attestation_id in attestation_ids.iter() {
                if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
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

    pub fn get_attestation(env: Env, attestation_id: String) -> Result<Attestation, Error> {
        Storage::get_attestation(&env, &attestation_id)
    }

    /// Request GDPR deletion of an attestation.
    ///
    /// Only the subject of the attestation may call this. The attestation is
    /// marked as `deleted` (soft-delete) and removed from the subject index so
    /// it no longer appears in any query result. The record itself is retained
    /// in storage for audit purposes, but is invisible to all public queries.
    ///
    /// A `DeletionRequested` event is emitted for off-chain compliance audit trails.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — attestation does not exist.
    /// - [`Error::Unauthorized`] — caller is not the subject of the attestation.
    pub fn request_deletion(
        env: Env,
        subject: Address,
        attestation_id: String,
    ) -> Result<(), Error> {
        subject.require_auth();

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;

        if attestation.subject != subject {
            return Err(Error::Unauthorized);
        }

        attestation.deleted = true;
        Storage::set_attestation(&env, &attestation);
        Storage::remove_subject_attestation(&env, &subject, &attestation_id);

        let timestamp = env.ledger().timestamp();
        Events::deletion_requested(&env, &subject, &attestation_id, timestamp);
        Ok(())
    }

    /// Return the full audit log for `attestation_id`.
    ///
    /// The log is append-only and contains one entry per state change
    /// (create, revoke, renew, update). Returns an empty list if the
    /// attestation has no recorded history.
    pub fn get_audit_log(env: Env, attestation_id: String) -> Vec<AuditEntry> {
        Storage::get_audit_log(&env, &attestation_id)
    }

    pub fn get_attestation_status(
        env: Env,
        attestation_id: String,
    ) -> Result<AttestationStatus, Error> {
        let attestation = Storage::get_attestation(&env, &attestation_id)?;
        let status = attestation.get_status(env.ledger().timestamp());

        if status == AttestationStatus::Expired {
            Events::attestation_expired(&env, &attestation_id, &attestation.subject);
        }

        Ok(status)
    }

    pub fn get_subject_attestations(
        env: Env,
        subject: Address,
        start: u32,
        limit: u32,
    ) -> Vec<String> {
        crate::storage::paginate(
            &env,
            Storage::get_subject_attestations(&env, &subject),
            start,
            limit,
        )
    }

    pub fn get_attestations_by_tag(env: Env, subject: Address, tag: String) -> Vec<String> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let mut result = Vec::new(&env);

        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if attestation.deleted {
                    continue;
                }
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

    pub fn get_issuer_attestations(
        env: Env,
        issuer: Address,
        start: u32,
        limit: u32,
    ) -> Vec<String> {
        crate::storage::paginate(
            &env,
            Storage::get_issuer_attestations(&env, &issuer),
            start,
            limit,
        )
    }

    pub fn get_valid_claims(env: Env, subject: Address) -> Vec<String> {
        let current_time = env.ledger().timestamp();
        let mut result = Vec::new(&env);

        for attestation_id in Storage::get_subject_attestations(&env, &subject).iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
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

    pub fn get_attestation_by_type(
        env: Env,
        subject: Address,
        claim_type: String,
    ) -> Result<Attestation, Error> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        let mut index = attestation_ids.len();

        while index > 0 {
            index -= 1;
            if let Some(attestation_id) = attestation_ids.get(index) {
                let attestation = Storage::get_attestation(&env, &attestation_id)?;
                if !attestation.deleted
                    && attestation.claim_type == claim_type
                    && attestation.get_status(current_time) == AttestationStatus::Valid
                {
                    return Ok(attestation);
                }
            }
        }

        Err(Error::NotFound)
    }

    pub fn is_issuer(env: Env, address: Address) -> bool {
        Storage::is_issuer(&env, &address)
    }

    pub fn get_issuer_stats(env: Env, issuer: Address) -> IssuerStats {
        Storage::get_issuer_stats(&env, &issuer)
    }

    pub fn is_bridge(env: Env, address: Address) -> bool {
        Storage::is_bridge(&env, &address)
    }

    pub fn set_issuer_metadata(
        env: Env,
        issuer: Address,
        metadata: IssuerMetadata,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::set_issuer_metadata(&env, &issuer, &metadata);
        Ok(())
    }

    pub fn get_issuer_metadata(env: Env, issuer: Address) -> Option<IssuerMetadata> {
        Storage::get_issuer_metadata(&env, &issuer)
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        Storage::get_admin(&env)
    }

    pub fn get_fee_config(env: Env) -> Result<FeeConfig, Error> {
        load_fee_config(&env)
    }

    pub fn register_claim_type(
        env: Env,
        admin: Address,
        claim_type: String,
        description: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;

        let info = ClaimTypeInfo {
            claim_type: claim_type.clone(),
            description: description.clone(),
        };
        Storage::set_claim_type(&env, &info);
        Events::claim_type_registered(&env, &claim_type, &description);
        Ok(())
    }

    pub fn get_claim_type_description(env: Env, claim_type: String) -> Option<String> {
        Storage::get_claim_type(&env, &claim_type).map(|info| info.description)
    }

    pub fn list_claim_types(env: Env, start: u32, limit: u32) -> Vec<String> {
        crate::storage::paginate(&env, Storage::get_claim_type_list(&env), start, limit)
    }

    /// Create a multi-sig attestation proposal.
    ///
    /// The proposer automatically counts as the first signer. The proposal
    /// expires after `MULTISIG_PROPOSAL_TTL_SECS` seconds if not completed.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — proposer is not a registered issuer, or any
    ///   address in `required_signers` is not a registered issuer.
    /// - [`Error::InvalidThreshold`] — threshold is 0 or exceeds signer count.
    pub fn propose_attestation(
        env: Env,
        proposer: Address,
        subject: Address,
        claim_type: String,
        required_signers: Vec<Address>,
        threshold: u32,
    ) -> Result<String, Error> {
        proposer.require_auth();
        Validation::require_issuer(&env, &proposer)?;

        // Validate all required signers are registered issuers.
        for signer in required_signers.iter() {
            Validation::require_issuer(&env, &signer)?;
        }

        let signer_count = required_signers.len();
        if threshold == 0 || threshold > signer_count {
            return Err(Error::InvalidThreshold);
        }

        let timestamp = env.ledger().timestamp();
        let proposal_id =
            MultiSigProposal::generate_id(&env, &proposer, &subject, &claim_type, timestamp);

        // Proposer auto-signs on creation.
        let mut signers = Vec::new(&env);
        signers.push_back(proposer.clone());

        let proposal = MultiSigProposal {
            id: proposal_id.clone(),
            proposer: proposer.clone(),
            subject: subject.clone(),
            claim_type,
            required_signers,
            threshold,
            signers,
            created_at: timestamp,
            expires_at: timestamp + MULTISIG_PROPOSAL_TTL_SECS,
            finalized: false,
        };

        Storage::set_multisig_proposal(&env, &proposal);
        Events::multisig_proposed(&env, &proposal_id, &proposer, &subject, threshold);
        Ok(proposal_id)
    }

    /// Co-sign an existing multi-sig proposal.
    ///
    /// When the number of signatures reaches `threshold`, the attestation is
    /// automatically finalized and stored as an active attestation.
    ///
    /// # Errors
    /// - [`Error::NotFound`] — proposal does not exist.
    /// - [`Error::ProposalExpired`] — proposal window has passed.
    /// - [`Error::ProposalFinalized`] — proposal already activated.
    /// - [`Error::NotRequiredSigner`] — issuer is not in the required signers list.
    /// - [`Error::AlreadySigned`] — issuer has already co-signed.
    pub fn cosign_attestation(env: Env, issuer: Address, proposal_id: String) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;

        let mut proposal = Storage::get_multisig_proposal(&env, &proposal_id)?;

        if proposal.finalized {
            return Err(Error::ProposalFinalized);
        }

        let current_time = env.ledger().timestamp();
        if current_time >= proposal.expires_at {
            return Err(Error::ProposalExpired);
        }

        // Verify issuer is in the required signers list.
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

        // Check for duplicate signature.
        for signer in proposal.signers.iter() {
            if signer == issuer {
                return Err(Error::AlreadySigned);
            }
        }

        proposal.signers.push_back(issuer.clone());
        let sig_count = proposal.signers.len();

        Events::multisig_cosigned(&env, &proposal_id, &issuer, sig_count, proposal.threshold);

        if sig_count >= proposal.threshold {
            // Threshold reached — finalize into an active attestation.
            proposal.finalized = true;
            Storage::set_multisig_proposal(&env, &proposal);

            let attestation_id = Attestation::generate_id(
                &env,
                &proposal.proposer,
                &proposal.subject,
                &proposal.claim_type,
                proposal.created_at,
            );

            let attestation = Attestation {
                id: attestation_id.clone(),
                issuer: proposal.proposer.clone(),
                subject: proposal.subject.clone(),
                claim_type: proposal.claim_type.clone(),
                timestamp: proposal.created_at,
                expiration: None,
                revoked: false,
                metadata: None,
                valid_from: None,
                imported: false,
                bridged: false,
                source_chain: None,
                source_tx: None,
                tags: None,
                revocation_reason: None,
                deleted: false,
            };

            store_attestation(&env, &attestation);
            Storage::increment_total_attestations(&env, 1);
            Events::attestation_created(&env, &attestation);
            Events::multisig_activated(&env, &proposal_id, &attestation_id);
        } else {
            Storage::set_multisig_proposal(&env, &proposal);
        }

        Ok(())
    }

    /// Retrieve a multi-sig proposal by ID.
    pub fn get_multisig_proposal(env: Env, proposal_id: String) -> Result<MultiSigProposal, Error> {
        Storage::get_multisig_proposal(&env, &proposal_id)
    }

    /// Endorse an existing attestation, adding a layer of social proof.
    ///
    /// Only registered issuers may endorse. An issuer cannot endorse their own
    /// attestation, and cannot endorse a revoked attestation. Each issuer may
    /// endorse a given attestation at most once.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] — endorser is not a registered issuer.
    /// - [`Error::NotFound`] — attestation does not exist.
    /// - [`Error::CannotEndorseOwn`] — endorser is the attestation's issuer.
    /// - [`Error::AlreadyRevoked`] — attestation has been revoked.
    /// - [`Error::AlreadyEndorsed`] — endorser has already endorsed this attestation.
    pub fn endorse_attestation(
        env: Env,
        endorser: Address,
        attestation_id: String,
    ) -> Result<(), Error> {
        endorser.require_auth();
        Validation::require_issuer(&env, &endorser)?;

        let attestation = Storage::get_attestation(&env, &attestation_id)?;

        if attestation.issuer == endorser {
            return Err(Error::CannotEndorseOwn);
        }

        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        // Prevent duplicate endorsements from the same issuer.
        for existing in Storage::get_endorsements(&env, &attestation_id).iter() {
            if existing.endorser == endorser {
                return Err(Error::AlreadyEndorsed);
            }
        }

        let timestamp = env.ledger().timestamp();
        let endorsement = Endorsement {
            attestation_id: attestation_id.clone(),
            endorser: endorser.clone(),
            timestamp,
        };

        Storage::add_endorsement(&env, &endorsement);
        Events::attestation_endorsed(&env, &attestation_id, &endorser, timestamp);
        Ok(())
    }

    /// Return all endorsements for the given attestation.
    pub fn get_endorsements(env: Env, attestation_id: String) -> Vec<Endorsement> {
        Storage::get_endorsements(&env, &attestation_id)
    }

    /// Return the number of endorsements for the given attestation.
    pub fn get_endorsement_count(env: Env, attestation_id: String) -> u32 {
        Storage::get_endorsements(&env, &attestation_id).len()
    }

    pub fn get_version(env: Env) -> Result<String, Error> {
        Storage::get_version(&env).ok_or(Error::NotInitialized)
    }

    /// Return global contract statistics.
    ///
    /// No authentication required — safe to call from dashboards and analytics tools.
    pub fn get_global_stats(env: Env) -> GlobalStats {
        Storage::get_global_stats(&env)
    }

    /// Lightweight health probe for monitoring dashboards and uptime checks.
    ///
    /// No authentication required. Returns `initialized: false` before
    /// `initialize` has been called.
    pub fn health_check(env: Env) -> HealthStatus {
        let initialized = Storage::has_admin(&env);
        let stats = Storage::get_global_stats(&env);
        HealthStatus {
            initialized,
            admin_set: initialized,
            issuer_count: stats.total_issuers,
            total_attestations: stats.total_attestations,
        }
    }

    pub fn get_contract_metadata(env: Env) -> Result<ContractMetadata, Error> {
        let version = Storage::get_version(&env).ok_or(Error::NotInitialized)?;
        Ok(ContractMetadata {
            name: String::from_str(&env, "TrustLink"),
            version,
            description: String::from_str(
                &env,
                "On-chain attestation and verification system for the Stellar blockchain.",
            ),
        })
    }

    pub fn get_config(env: Env) -> ContractConfig {
        let ttl_config = Storage::get_ttl_config(&env).unwrap_or(TtlConfig { ttl_days: 30 });

        let fee_config = Storage::get_fee_config(&env).unwrap_or_else(|| FeeConfig {
            attestation_fee: 0,
            fee_collector: env.current_contract_address(),
            fee_token: None,
        });

        let version = Storage::get_version(&env).unwrap_or_else(|| String::from_str(&env, ""));

        ContractConfig {
            ttl_config,
            fee_config,
            contract_name: String::from_str(&env, "TrustLink"),
            contract_version: version,
            contract_description: String::from_str(
                &env,
                "On-chain attestation and verification system for the Stellar blockchain.",
            ),
        }
    }
}
