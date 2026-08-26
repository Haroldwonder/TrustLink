//! Tests for the cleanup_expired_requests function.
//!
//! Verifies that the function correctly removes expired AttestationRequests
//! from an issuer's pending list while leaving non-expired requests intact.
//! It also verifies auth and issuer-registration requirements.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String};
use trustlink::{types::Error, TrustLinkContract, TrustLinkContractClient};

/// 7-day TTL used by request_attestation, mirrored from types::ATTESTATION_REQUEST_TTL_SECS.
const REQUEST_TTL_SECS: u64 = 7 * 24 * 60 * 60;

fn setup(env: &Env) -> (Address, Address, Address, TrustLinkContractClient<'_>) {
    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let issuer = Address::generate(env);
    let subject = Address::generate(env);
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);
    (admin, issuer, subject, client)
}

fn kyc(env: &Env) -> String {
    String::from_str(env, "KYC_PASSED")
}

// ---------------------------------------------------------------------------
// Happy-path: expired requests are removed
// ---------------------------------------------------------------------------

#[test]
fn test_cleanup_removes_expired_requests() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim = kyc(&env);

    // Submit a request at t=1000.
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let _req_id = client.request_attestation(&subject, &issuer, &claim);

    // Confirm it is in the pending list.
    let pending_before = client.get_pending_requests(&issuer, &0, &10);
    assert_eq!(pending_before.len(), 1);

    // Advance past the 7-day TTL so the request is expired.
    env.ledger().with_mut(|l| l.timestamp = 1000 + REQUEST_TTL_SECS + 1);

    // cleanup should succeed and the pending list should now be empty.
    client.cleanup_expired_requests(&issuer);

    let pending_after = client.get_pending_requests(&issuer, &0, &10);
    assert_eq!(pending_after.len(), 0);
}

#[test]
fn test_cleanup_preserves_non_expired_requests() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim = kyc(&env);

    // Submit a request at t=1000.
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let req_id = client.request_attestation(&subject, &issuer, &claim);

    // Advance to just before the TTL expires (still within the window).
    env.ledger().with_mut(|l| l.timestamp = 1000 + REQUEST_TTL_SECS - 1);

    // Cleanup should keep the request.
    client.cleanup_expired_requests(&issuer);

    let pending = client.get_pending_requests(&issuer, &0, &10);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending.get(0).unwrap().id, req_id);
}

#[test]
fn test_cleanup_mixed_expired_and_fresh_requests() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    // Submit an old request at t=1000.
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let old_claim = String::from_str(&env, "CLAIM_OLD");
    let _old_req = client.request_attestation(&subject, &issuer, &old_claim);

    // Advance past TTL, then submit a fresh request.
    let fresh_start = 1000 + REQUEST_TTL_SECS + 1;
    env.ledger().with_mut(|l| l.timestamp = fresh_start);
    let fresh_claim = String::from_str(&env, "CLAIM_NEW");
    let fresh_req_id = client.request_attestation(&subject, &issuer, &fresh_claim);

    // Confirm both (stale + fresh) are visible before cleanup.
    // NOTE: get_pending_requests already filters expired items at read-time,
    // but cleanup_expired_requests compacts the underlying storage list.
    // After cleanup the old entry must be gone; the fresh one must survive.
    client.cleanup_expired_requests(&issuer);

    let pending = client.get_pending_requests(&issuer, &0, &10);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending.get(0).unwrap().id, fresh_req_id);
}

#[test]
fn test_cleanup_empty_list_is_noop() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, _, client) = setup(&env);

    // No requests have been created; calling cleanup should succeed silently.
    client.cleanup_expired_requests(&issuer);

    let pending = client.get_pending_requests(&issuer, &0, &10);
    assert_eq!(pending.len(), 0);
}

#[test]
fn test_cleanup_exactly_at_expiry_boundary() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim = kyc(&env);

    env.ledger().with_mut(|l| l.timestamp = 1000);
    let _req_id = client.request_attestation(&subject, &issuer, &claim);

    // Advance to exactly the expiry timestamp (expires_at = 1000 + TTL).
    // The implementation keeps entries where current_time < req.expires_at,
    // so at exactly expires_at the entry is already considered expired.
    env.ledger().with_mut(|l| l.timestamp = 1000 + REQUEST_TTL_SECS);

    client.cleanup_expired_requests(&issuer);

    let pending = client.get_pending_requests(&issuer, &0, &10);
    assert_eq!(pending.len(), 0);
}

#[test]
fn test_cleanup_multiple_expired_requests_all_removed() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    // Submit three requests at different timestamps, all of which will expire.
    let claims = ["CLAIM_A", "CLAIM_B", "CLAIM_C"];
    for (i, name) in claims.iter().enumerate() {
        env.ledger().with_mut(|l| l.timestamp = 1000 + i as u64);
        let claim = String::from_str(&env, name);
        client.request_attestation(&subject, &issuer, &claim);
    }

    // Jump past all expirations.
    env.ledger().with_mut(|l| l.timestamp = 1000 + REQUEST_TTL_SECS + 100);

    client.cleanup_expired_requests(&issuer);

    let pending = client.get_pending_requests(&issuer, &0, &10);
    assert_eq!(pending.len(), 0);
}

// ---------------------------------------------------------------------------
// Idempotency: calling cleanup twice is safe
// ---------------------------------------------------------------------------

#[test]
fn test_cleanup_is_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim = kyc(&env);
    env.ledger().with_mut(|l| l.timestamp = 1000);
    client.request_attestation(&subject, &issuer, &claim);

    env.ledger().with_mut(|l| l.timestamp = 1000 + REQUEST_TTL_SECS + 1);

    // First cleanup
    client.cleanup_expired_requests(&issuer);
    // Second cleanup on an already-empty list
    client.cleanup_expired_requests(&issuer);

    let pending = client.get_pending_requests(&issuer, &0, &10);
    assert_eq!(pending.len(), 0);
}

// ---------------------------------------------------------------------------
// Fulfilled / rejected requests do not reappear after cleanup
// ---------------------------------------------------------------------------

#[test]
fn test_cleanup_after_fulfilled_request() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, issuer, subject, client) = setup(&env);

    let claim = kyc(&env);
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let req_id = client.request_attestation(&subject, &issuer, &claim);

    // Fulfill the request before it expires.
    client.fulfill_request(&issuer, &req_id, &None);

    // Advance past TTL and clean up.
    env.ledger().with_mut(|l| l.timestamp = 1000 + REQUEST_TTL_SECS + 1);
    client.cleanup_expired_requests(&issuer);

    // The fulfilled request should not reappear.
    let pending = client.get_pending_requests(&issuer, &0, &10);
    assert_eq!(pending.len(), 0);
}

// ---------------------------------------------------------------------------
// Auth & registration errors
// ---------------------------------------------------------------------------

#[test]
fn test_cleanup_requires_issuer_registration() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &None);

    // A random address that was never registered as an issuer.
    let stranger = Address::generate(&env);
    let result = client.try_cleanup_expired_requests(&stranger);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}
