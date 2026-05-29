use soroban_sdk::{symbol_short, Address, Env, String, Symbol};

use crate::types::{Attestation, IssuerTier};

// Event topic constants (max 9 chars for symbol_short!)
const TOPIC_ADM_INIT: Symbol = symbol_short!("adm_init");
const TOPIC_CREATED: Symbol = symbol_short!("att_crtd");
const TOPIC_IMPORTED: Symbol = symbol_short!("att_imp");
const TOPIC_BRIDGED: Symbol = symbol_short!("att_brdg");
const TOPIC_REVOKED: Symbol = symbol_short!("att_rvkd");
const TOPIC_RENEWED: Symbol = symbol_short!("att_rnwd");
const TOPIC_UPDATED: Symbol = symbol_short!("att_upd");
const TOPIC_EXPIRED: Symbol = symbol_short!("att_exp");
const TOPIC_DEL_REQ: Symbol = symbol_short!("del_req");
const TOPIC_ISS_REG: Symbol = symbol_short!("iss_reg");
const TOPIC_ISS_TIER: Symbol = symbol_short!("iss_tier");
const TOPIC_ISS_REM: Symbol = symbol_short!("iss_rem");
const TOPIC_ADMIN_INIT: Symbol = symbol_short!("adm_init");
const TOPIC_CREATED: Symbol = symbol_short!("created");
const TOPIC_IMPORTED: Symbol = symbol_short!("imported");
const TOPIC_BRIDGED: Symbol = symbol_short!("bridged");
const TOPIC_REVOKED: Symbol = symbol_short!("revoked");
const TOPIC_RENEWED: Symbol = symbol_short!("renewed");
const TOPIC_UPDATED: Symbol = symbol_short!("updated");
const TOPIC_EXPIRED: Symbol = symbol_short!("expired");
const TOPIC_DEL_REQ: Symbol = symbol_short!("del_req");
const TOPIC_ISS_REG: Symbol = symbol_short!("iss_reg");
const TOPIC_ISS_REM: Symbol = symbol_short!("iss_rem");
const TOPIC_ISS_TIER: Symbol = symbol_short!("iss_tier");
const TOPIC_CLM_TYPE: Symbol = symbol_short!("clm_type");
const TOPIC_MS_PROP: Symbol = symbol_short!("ms_prop");
const TOPIC_MS_SIGN: Symbol = symbol_short!("ms_sign");
const TOPIC_MS_ACTV: Symbol = symbol_short!("ms_actv");
const TOPIC_ADM_XFER: Symbol = symbol_short!("adm_xfer");
const TOPIC_ADM_ADD: Symbol = symbol_short!("adm_add");
const TOPIC_ADM_REM: Symbol = symbol_short!("adm_rem");
const TOPIC_ENDORSED: Symbol = symbol_short!("endorsed");
const TOPIC_EXP_HOOK: Symbol = symbol_short!("exp_hook");
const TOPIC_PAUSED: Symbol = symbol_short!("paused");
const TOPIC_REQ: Symbol = symbol_short!("req");
const TOPIC_REQ_OK: Symbol = symbol_short!("req_ok");
const TOPIC_REQ_NO: Symbol = symbol_short!("req_no");
const TOPIC_WL_ON: Symbol = symbol_short!("wl_on");
const TOPIC_WL_ADD: Symbol = symbol_short!("wl_add");
const TOPIC_WL_REM: Symbol = symbol_short!("wl_rem");
const TOPIC_TPL_DEL: Symbol = symbol_short!("tpl_del");

const TOPIC_REQ: Symbol = symbol_short!("att_req");
const TOPIC_REQ_OK: Symbol = symbol_short!("req_ok");
const TOPIC_REQ_NO: Symbol = symbol_short!("req_no");
const TOPIC_REQ_CANCEL: Symbol = symbol_short!("req_cncl");

const TOPIC_WL_ADD: Symbol = symbol_short!("wl_add");
const TOPIC_WL_REM: Symbol = symbol_short!("wl_rem");
const TOPIC_WL_ON: Symbol = symbol_short!("wl_on");

pub struct Events;

impl Events {
    pub fn admin_initialized(env: &Env, admin: &Address, timestamp: u64) {
        env.events()
            .publish((TOPIC_ADM_INIT,), (admin.clone(), timestamp));
    }

    pub fn attestation_created(env: &Env, attestation: &Attestation) {
        env.events().publish(
            (TOPIC_CREATED, attestation.subject.clone()),
            (
                attestation.id.clone(),
                attestation.issuer.clone(),
                attestation.claim_type.clone(),
                attestation.timestamp,
                attestation.metadata.clone(),
            ),
        );
    }

    pub fn attestation_imported(env: &Env, attestation: &Attestation) {
        env.events().publish(
            (TOPIC_IMPORTED, attestation.subject.clone()),
            (
                attestation.id.clone(),
                attestation.issuer.clone(),
                attestation.claim_type.clone(),
                attestation.timestamp,
                attestation.expiration,
            ),
        );
    }

    pub fn attestation_bridged(env: &Env, attestation: &Attestation) {
        env.events().publish(
            (TOPIC_BRIDGED, attestation.subject.clone()),
            (
                attestation.id.clone(),
                attestation.issuer.clone(),
                attestation.claim_type.clone(),
                attestation
                    .source_chain
                    .clone()
                    .unwrap_or(String::from_str(env, "")),
                attestation
                    .source_tx
                    .clone()
                    .unwrap_or(String::from_str(env, "")),
            ),
        );
    }

    pub fn attestation_revoked(env: &Env, attestation_id: &String, issuer: &Address, reason: &Option<String>) {
        env.events().publish(
            (TOPIC_REVOKED, issuer.clone()),
            (attestation_id.clone(), reason.clone()),
        );
    }

    pub fn attestation_revoked_with_reason(
        env: &Env,
        attestation_id: &String,
        issuer: &Address,
        reason: &Option<String>,
    ) {
        env.events().publish(
            (TOPIC_REVOKED, issuer.clone()),
            (attestation_id.clone(), reason.clone()),
        );
    }

    pub fn attestation_renewed(
        env: &Env,
        attestation_id: &String,
        issuer: &Address,
        new_expiration: Option<u64>,
    ) {
        env.events().publish(
            (TOPIC_RENEWED, issuer.clone()),
            (attestation_id.clone(), new_expiration),
        );
    }

    pub fn attestation_updated(
        env: &Env,
        attestation_id: &String,
        issuer: &Address,
        new_expiration: Option<u64>,
    ) {
        env.events().publish(
            (TOPIC_UPDATED, issuer.clone()),
            (attestation_id.clone(), new_expiration),
        );
    }

    pub fn attestation_expired(env: &Env, attestation_id: &String, subject: &Address) {
        env.events().publish(
            (TOPIC_EXPIRED, subject.clone()),
            attestation_id.clone(),
        );
    }

    pub fn deletion_requested(
        env: &Env,
        subject: &Address,
        attestation_id: &String,
        timestamp: u64,
    ) {
        env.events().publish(
            (TOPIC_DEL_REQ, subject.clone()),
            (attestation_id.clone(), timestamp),
        );
    }

    pub fn issuer_registered(env: &Env, issuer: &Address, admin: &Address, timestamp: u64) {
        env.events().publish(
            (TOPIC_ISS_REG, issuer.clone()),
            (admin.clone(), timestamp),
        );
    }

    pub fn issuer_tier_updated(env: &Env, issuer: &Address, tier: &IssuerTier) {
        env.events()
            .publish((TOPIC_ISS_TIER, issuer.clone()), (old_tier, new_tier));
    }

    pub fn issuer_removed(env: &Env, issuer: &Address, admin: &Address, timestamp: u64) {
        env.events().publish(
            (TOPIC_ISS_REM, issuer.clone()),
            (admin.clone(), timestamp),
        );
    }

    pub fn claim_type_registered(env: &Env, claim_type: &String, description: &String) {
        env.events().publish(
            (TOPIC_CLM_TYPE, claim_type.clone()),
            description.clone(),
        );
    }

    pub fn multisig_proposed(
        env: &Env,
        proposal_id: &String,
        proposer: &Address,
        subject: &Address,
        threshold: u32,
    ) {
        env.events().publish(
            (TOPIC_MS_PROP, subject.clone()),
            (proposal_id.clone(), proposer.clone(), threshold),
        );
    }

    pub fn multisig_cosigned(
        env: &Env,
        proposal_id: &String,
        signer: &Address,
        signatures_so_far: u32,
        threshold: u32,
    ) {
        env.events().publish(
            (TOPIC_MS_SIGN, signer.clone()),
            (proposal_id.clone(), signatures_so_far, threshold),
        );
    }

    pub fn multisig_activated(env: &Env, proposal_id: &String, attestation_id: &String) {
        env.events().publish(
            (TOPIC_MS_ACTV,),
            (proposal_id.clone(), attestation_id.clone()),
        );
    }

    pub fn admin_transferred(env: &Env, old_admin: &Address, new_admin: &Address) {
        env.events().publish(
            (TOPIC_ADM_XFER,),
            (old_admin.clone(), new_admin.clone()),
        );
    }

    pub fn admin_transfer_proposed(env: &Env, current_admin: &Address, new_admin: &Address) {
        env.events().publish(
            (symbol_short!("adm_prop"), current_admin.clone()),
            new_admin.clone(),
        );
    }

    pub fn admin_added(env: &Env, by_admin: &Address, new_admin: &Address, timestamp: u64) {
        env.events().publish(
            (TOPIC_ADM_ADD, by_admin.clone()),
            (new_admin.clone(), timestamp),
        );
    }

    pub fn admin_removed(env: &Env, by_admin: &Address, removed_admin: &Address, timestamp: u64) {
        env.events().publish(
            (TOPIC_ADM_REM, by_admin.clone()),
            (removed_admin.clone(), timestamp),
        );
    }

    pub fn attestation_transferred(
        env: &Env,
        attestation_id: &String,
        old_issuer: &Address,
        new_issuer: &Address,
    ) {
        env.events().publish(
            (symbol_short!("att_xfer"), old_issuer.clone()),
            (attestation_id.clone(), new_issuer.clone()),
        );
    }

    /// Emitted when a registered issuer endorses an existing attestation.
    pub fn attestation_endorsed(
        env: &Env,
        attestation_id: &String,
        endorser: &Address,
        timestamp: u64,
    ) {
        env.events().publish(
            (TOPIC_ENDORSED, endorser.clone()),
            (attestation_id.clone(), timestamp),
        );
    }

    pub fn expiration_hook_triggered(
        env: &Env,
        subject: &Address,
        attestation_id: &String,
        expiration: u64,
    ) {
        env.events().publish(
            (TOPIC_EXP_HOOK, subject.clone()),
            (attestation_id.clone(), expiration),
        );
    }

    pub fn contract_paused(env: &Env, admin: &Address, timestamp: u64) {
        env.events()
            .publish((TOPIC_PAUSED,), (admin.clone(), timestamp));
            .publish((symbol_short!("paused"),), (admin.clone(), timestamp));
    }

    pub fn contract_unpaused(env: &Env, admin: &Address, timestamp: u64) {
        env.events()
            .publish((symbol_short!("unpaused"),), (admin.clone(), timestamp));
    }

    pub fn attestation_requested(
        env: &Env,
        request_id: &String,
        subject: &Address,
        issuer: &Address,
        claim_type: &String,
        expires_at: u64,
    ) {
        env.events().publish(
            (TOPIC_REQ, issuer.clone()),
            (
                request_id.clone(),
                subject.clone(),
                claim_type.clone(),
                expires_at,
            ),
        );
    }

    pub fn request_fulfilled(
        env: &Env,
        request_id: &String,
        issuer: &Address,
        attestation_id: &String,
    ) {
        env.events().publish(
            (TOPIC_REQ_OK, issuer.clone()),
            (request_id.clone(), attestation_id.clone()),
        );
    }

    pub fn request_rejected(
        env: &Env,
        request_id: &String,
        issuer: &Address,
        reason: &Option<String>,
    ) {
        env.events().publish(
            (TOPIC_REQ_NO, issuer.clone()),
            (request_id.clone(), reason.clone()),
        );
    }

    /// Emitted when a subject cancels their own pending attestation request.
    pub fn request_cancelled(env: &Env, request_id: &String, subject: &Address) {
        env.events().publish(
            (TOPIC_REQ_CANCEL, subject.clone()),
            request_id.clone(),
        );
    }

    /// Emitted when issuer creates a delegation to a sub-issuer for a claim type.
    pub fn delegation_created(
        env: &Env,
        delegator: &Address,
        delegate: &Address,
        claim_type: &String,
        expiration: Option<u64>,
    ) {
        env.events().publish(
            (symbol_short!("del_crtd"), delegator.clone()),
            (delegate.clone(), claim_type.clone(), expiration),
        );
    }

    pub fn delegation_revoked(
        env: &Env,
        delegator: &Address,
        delegate: &Address,
        claim_type: &String,
    ) {
        env.events().publish(
            (symbol_short!("del_rvkd"), delegator.clone()),
            (delegate.clone(), claim_type.clone()),
        );
    }

    pub fn whitelist_mode_enabled(env: &Env, issuer: &Address) {
        env.events()
            .publish((TOPIC_WL_ON, issuer.clone()), ());
    }

    pub fn whitelist_updated(env: &Env, issuer: &Address, subject: &Address, added: bool) {
        let sym = if added { TOPIC_WL_ADD } else { TOPIC_WL_REM };
        env.events().publish((sym, issuer.clone()), subject.clone());
    }

    /// Emitted when an issuer creates or overwrites a template.
    pub fn template_created(env: &Env, issuer: &Address, template_id: &String) {
        env.events().publish(
            (symbol_short!("tmpl_crt"), issuer.clone()),
            template_id.clone(),
        );
    }

    pub fn council_initialized(env: &Env, quorum: u32, member_count: u32) {
        env.events().publish(
            (symbol_short!("cncl_ini"),),
            (quorum, member_count),
        );
    }

    pub fn proposal_created(env: &Env, proposal_id: u32, proposer: &Address) {
        env.events().publish(
            (symbol_short!("prop_new"), proposer.clone()),
            proposal_id,
        );
    }

    pub fn proposal_approved(env: &Env, proposal_id: u32, approver: &Address) {
        env.events().publish(
            (symbol_short!("prop_ok"), approver.clone()),
            proposal_id,
        );
    }

    pub fn proposal_executed(env: &Env, proposal_id: u32) {
        env.events().publish(
            (symbol_short!("prop_exe"),),
            proposal_id,
        );
    }

    /// Emitted when an issuer deletes one of their attestation templates (Issue #530).
    pub fn template_deleted(env: &Env, issuer: &Address, template_id: &String) {
        env.events().publish(
            (TOPIC_TPL_DEL, issuer.clone()),
            template_id.clone(),
        );
    }
}
