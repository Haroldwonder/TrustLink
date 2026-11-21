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
//! Fuzz target for the attestation request lifecycle.
//!
//! Decodes arbitrary bytes into a sequence of operations
//! (`request_attestation`, `fulfill_request`, `reject_request`,
//! `cancel_request`, `cleanup_expired_requests`) plus random ledger time
//! jumps, executes them against the contract in a Soroban test `Env`, and
//! asserts that no operation panics and that after
//! `cleanup_expired_requests` every entry in each issuer's pending list has
//! status `Pending` and `expires_at > current ledger time`.

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Bytes, Env, Vec,
};

use attestation_contract::{
    AttestationRequestStatus, AttestationService, AttestationServiceClient,
};

/// A single fuzzable operation against the attestation contract.
#[derive(Arbitrary, Debug, Clone)]
enum Op {
    /// Request an attestation from an issuer for a subject.
    RequestAttestation {
        issuer_idx: u8,
        subject_idx: u8,
        ttl: u32,
        data: Vec<u8>,
    },
    /// Fulfill a pending request identified by its request id.
    FulfillRequest { request_id: u64 },
    /// Reject a pending request identified by its request id.
    RejectRequest { request_id: u64 },
    /// Cancel a pending request identified by its request id.
    CancelRequest { request_id: u64 },
    /// Advance the ledger timestamp by a random amount.
    AdvanceTime { delta: u32 },
    /// Run the expired-request cleanup routine.
    CleanupExpiredRequests,
}

/// A full fuzz input: a sequence of operations.
#[derive(Arbitrary, Debug)]
struct FuzzInput {
    ops: Vec<Op>,
}

const ISSUER_COUNT: u8 = 4;
const SUBJECT_COUNT: u8 = 4;

fn issuer_at(env: &Env, idx: u8) -> Address {
    // Deterministically derive an address from the index so that repeated
    // operations target the same issuer.
    let mut bytes = Bytes::new(env);
    bytes.push_back(b'i');
    bytes.push_back(idx % ISSUER_COUNT);
    Address::from_string_bytes(&bytes.to_string())
}

fn subject_at(env: &Env, idx: u8) -> Address {
    let mut bytes = Bytes::new(env);
    bytes.push_back(b's');
    bytes.push_back(idx % SUBJECT_COUNT);
    Address::from_string_bytes(&bytes.to_string())
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    let input = match FuzzInput::arbitrary(&mut u) {
        Ok(input) => input,
        Err(_) => return,
    };

    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AttestationService);
    let client = AttestationServiceClient::new(&env, &contract_id);

    // Start from a non-zero ledger time so time jumps are meaningful.
    env.ledger().with_mut(|li| {
        li.timestamp = 1_000;
    });

    // Track the request ids we have observed so we can target them later.
    let mut known_request_ids: Vec<u64> = Vec::new(&env);

    for op in input.ops.iter() {
        match op {
            Op::RequestAttestation {
                issuer_idx,
                subject_idx,
                ttl,
                data,
            } => {
                let issuer = issuer_at(&env, *issuer_idx);
                let subject = subject_at(&env, *subject_idx);
                // Clamp ttl to a sane range to avoid overflow in the contract.
                let ttl = (*ttl).min(1_000_000);
                let request_id = client.request_attestation(
                    &issuer,
                    &subject,
                    &ttl,
                    &data.clone(),
                );
                known_request_ids.push_back(request_id);
            }
            Op::FulfillRequest { request_id } => {
                if known_request_ids.is_empty() {
                    continue;
                }
                let id = request_id % known_request_ids.len() as u64;
                let id = known_request_ids.get(id as u32).unwrap();
                // Fulfillment may fail (e.g. already resolved); we only care
                // that it does not panic.
                let _ = client.try_fulfill_request(&id);
            }
            Op::RejectRequest { request_id } => {
                if known_request_ids.is_empty() {
                    continue;
                }
                let id = request_id % known_request_ids.len() as u64;
                let id = known_request_ids.get(id as u32).unwrap();
                let _ = client.try_reject_request(&id);
            }
            Op::CancelRequest { request_id } => {
                if known_request_ids.is_empty() {
                    continue;
                }
                let id = request_id % known_request_ids.len() as u64;
                let id = known_request_ids.get(id as u32).unwrap();
                let _ = client.try_cancel_request(&id);
            }
            Op::AdvanceTime { delta } => {
                let delta = (*delta).min(1_000_000) as u64;
                env.ledger().with_mut(|li| {
                    li.timestamp = li.timestamp.saturating_add(delta);
                });
            }
            Op::CleanupExpiredRequests => {
                let _ = client.try_cleanup_expired_requests();

                // Invariant: after cleanup, every entry in each issuer's
                // pending list must be `Pending` and unexpired.
                let now = env.ledger().timestamp();
                for issuer_idx in 0..ISSUER_COUNT {
                    let issuer = issuer_at(&env, issuer_idx);
                    let pending = client.get_pending_requests(&issuer);
                    for request in pending.iter() {
                        assert_eq!(
                            request.status,
                            AttestationRequestStatus::Pending,
                            "pending list contains a non-Pending request after cleanup",
                        );
                        assert!(
                            request.expires_at > now,
                            "pending list contains an expired request after cleanup",
                        );
                    }
                }
            }
        }
    }
});