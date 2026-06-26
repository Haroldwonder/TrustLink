use soroban_sdk::{Address, Env, String, Vec};

use crate::attestation::store_attestation;
use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    Attestation, AttestationOrigin, Error, IssuerTier, MultiSigProposal,
    MULTISIG_PROPOSAL_TTL_SECS,
};
use crate::validation::Validation;

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

    // Premium issuers bypass multi-sig for ACCREDITED_INVESTOR.
    let accredited = String::from_str(env, "ACCREDITED_INVESTOR");
    if claim_type == accredited {
        if let Some(IssuerTier::Premium) = Storage::get_issuer_tier(env, &proposer) {
            let timestamp = env.ledger().timestamp();
            let attestation_id =
                Attestation::generate_id(env, &proposer, &subject, &claim_type, timestamp);
            let attestation = Attestation {
                id: attestation_id.clone(),
                issuer: proposer.clone(),
                subject: subject.clone(),
                claim_type: claim_type.clone(),
                timestamp,
                expiration: None,
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
            return Ok(attestation_id);
        }
    }

    for signer in required_signers.iter() {
        Validation::require_issuer(env, &signer)?;
    }
    let signer_count = required_signers.len();
    if threshold == 0 || threshold > signer_count {
        return Err(Error::InvalidThreshold);
    }

    let timestamp = env.ledger().timestamp();
    let proposal_id = MultiSigProposal::generate_id(env, &proposer, &subject, &claim_type, timestamp);
    let mut signers = Vec::new(env);
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
    Storage::set_multisig_proposal(env, &proposal);
    Events::multisig_proposed(env, &proposal_id, &proposer, &subject, threshold);
    Ok(proposal_id)
}

pub fn cosign_attestation(env: &Env, issuer: Address, proposal_id: String) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(env, &issuer)?;
    Validation::require_not_paused(env)?;

    let mut proposal = Storage::get_multisig_proposal(env, &proposal_id)?;
    if proposal.finalized { return Err(Error::ProposalFinalized); }
    let current_time = env.ledger().timestamp();
    if current_time >= proposal.expires_at { return Err(Error::ProposalExpired); }

    let mut is_required = false;
    for signer in proposal.required_signers.iter() {
        if signer == issuer { is_required = true; break; }
    }
    if !is_required { return Err(Error::NotRequiredSigner); }

    for signer in proposal.signers.iter() {
        if signer == issuer { return Err(Error::AlreadySigned); }
    }

    proposal.signers.push_back(issuer.clone());
    let sig_count = proposal.signers.len();
    Events::multisig_cosigned(env, &proposal_id, &issuer, sig_count, proposal.threshold);

    if sig_count >= proposal.threshold {
        proposal.finalized = true;
        Storage::set_multisig_proposal(env, &proposal);

        let attestation_id = Attestation::generate_id(
            env, &proposal.proposer, &proposal.subject, &proposal.claim_type, proposal.created_at,
        );
        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: proposal.proposer.clone(),
            subject: proposal.subject.clone(),
            claim_type: proposal.claim_type.clone(),
            timestamp: proposal.created_at,
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
        };

        store_attestation(env, &attestation);
        Events::attestation_created(env, &attestation);
        Events::multisig_activated(env, &proposal_id, &attestation_id);
    } else {
        Storage::set_multisig_proposal(env, &proposal);
    }
    Ok(())
}

pub fn get_multisig_proposal(env: &Env, proposal_id: String) -> Result<MultiSigProposal, Error> {
    Storage::get_multisig_proposal(env, &proposal_id)
}

pub fn get_multisig_ttl(env: &Env) -> u32 {
    Storage::get_multisig_ttl_days(env)
}
