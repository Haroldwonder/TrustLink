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