//! Fuzz target for the multisig propose/cosign lifecycle.
//!
//! # Invariant under test
//!
//! A proposal must never transition to `finalized = true` unless the number
//! of recorded signers meets or exceeds the declared threshold.  Equivalently,
//! while `finalized == false` the signer count must be strictly less than the
//! threshold.
//!
//! The fuzzer drives arbitrary sequences of `propose_attestation` /
//! `cosign_attestation` calls with varying cosigner sets and threshold values,
//! asserting the invariant after every mutating operation.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use trustlink::TrustLinkContract;

/// Maximum number of required signers generated per run.
const MAX_SIGNERS: usize = 5;

/// Fixed claim type used throughout; content does not affect the state machine.
const CLAIM_TYPE: &str = "KYC_VERIFIED";

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    /// Raw byte used to derive the number of required signers (1..=MAX_SIGNERS).
    num_signers: u8,
    /// Raw byte used to derive the threshold (1..=num_signers).
    threshold: u8,
    /// Sequence of cosigner indices (mod num_signers) to attempt co-signing with.
    cosign_sequence: std::vec::Vec<u8>,
}

fuzz_target!(|input: FuzzInput| {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = trustlink::TrustLinkContractClient::new(&env, &contract_id);

    // Bootstrap: admin + register all participants as issuers.
    let admin = Address::generate(&env);
    client.initialize(&admin, &None);

    let num_signers = ((input.num_signers as usize) % MAX_SIGNERS).max(1);
    let threshold = ((input.threshold as u32) % (num_signers as u32)).max(1);

    let proposer = Address::generate(&env);
    client.register_issuer(&admin, &proposer);

    let mut required_signers: Vec<Address> = Vec::new(&env);
    let mut signer_vec: std::vec::Vec<Address> = std::vec::Vec::new();
    for _ in 0..num_signers {
        let s = Address::generate(&env);
        client.register_issuer(&admin, &s);
        required_signers.push_back(s.clone());
        signer_vec.push(s);
    }

    let subject = Address::generate(&env);
    let claim_type = String::from_str(&env, CLAIM_TYPE);

    // Propose; bail out if the proposal itself is rejected (e.g. invalid threshold).
    let proposal_id = match client.try_propose_attestation(
        &proposer,
        &subject,
        &claim_type,
        &required_signers,
        &threshold,
    ) {
        Ok(Ok(id)) => id,
        _ => return,
    };

    // Drive cosign attempts in the order dictated by the fuzzer.
    for idx_byte in &input.cosign_sequence {
        let signer_idx = (*idx_byte as usize) % num_signers;
        // Ignore errors (duplicate cosign, wrong signer, expired, etc.) — they
        // are expected paths; only the invariant matters.
        let _ = client.try_cosign_attestation(&signer_vec[signer_idx], &proposal_id);

        // Assert invariant after every attempt.
        if let Ok(Ok(proposal)) = client.try_get_multisig_proposal(&proposal_id) {
            if proposal.finalized {
                assert!(
                    proposal.signers.len() >= proposal.threshold,
                    "invariant violated: proposal finalized with {} signer(s) but threshold is {}",
                    proposal.signers.len(),
                    proposal.threshold,
                );
                // Proposal is finalized; further cosigns are no-ops.
                break;
            } else {
                assert!(
                    proposal.signers.len() < proposal.threshold,
                    "invariant violated: proposal not finalized but signer count {} >= threshold {}",
                    proposal.signers.len(),
                    proposal.threshold,
                );
            }
        }
    }

    // Final check after the full sequence.
    if let Ok(Ok(proposal)) = client.try_get_multisig_proposal(&proposal_id) {
        if proposal.finalized {
            assert!(
                proposal.signers.len() >= proposal.threshold,
                "invariant violated at end: finalized with {} signer(s), threshold {}",
                proposal.signers.len(),
                proposal.threshold,
            );
        } else {
            assert!(
                proposal.signers.len() < proposal.threshold,
                "invariant violated at end: not finalized but {} signer(s) >= threshold {}",
                proposal.signers.len(),
                proposal.threshold,
            );
        }
    }
});
