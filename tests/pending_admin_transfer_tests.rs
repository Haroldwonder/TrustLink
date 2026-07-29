//! Tests for get_pending_admin_transfer() read-only query.
//!
//! Acceptance criteria:
//!   - Returns None before a transfer is proposed.
//!   - Returns the correct PendingAdminTransfer after one is proposed.
//!   - Returns None again after the transfer is cancelled.
//!   - Returns None again after the transfer is accepted.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};
use trustlink::{types::PendingAdminTransfer, TrustLinkContract, TrustLinkContractClient};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn setup(env: &Env) -> (Address, TrustLinkContractClient<'_>) {
    let id = env.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin, &None);
    (admin, client)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_returns_none_before_transfer_proposed() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, client) = setup(&env);

    assert_eq!(client.get_pending_admin_transfer(), None);
}

#[test]
fn test_returns_correct_value_after_transfer_proposed() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup(&env);
    let new_admin = Address::generate(&env);

    client.propose_admin_transfer(&admin, &new_admin);

    let pending = client.get_pending_admin_transfer();
    assert!(pending.is_some());
    let transfer = pending.unwrap();
    assert_eq!(transfer.proposed_by, admin);
    assert_eq!(transfer.new_admin, new_admin);
}

#[test]
fn test_returns_none_after_transfer_cancelled() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup(&env);
    let new_admin = Address::generate(&env);

    client.propose_admin_transfer(&admin, &new_admin);
    assert!(client.get_pending_admin_transfer().is_some());

    client.cancel_admin_transfer(&admin);
    assert_eq!(client.get_pending_admin_transfer(), None);
}

#[test]
fn test_returns_none_after_transfer_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, client) = setup(&env);
    let new_admin = Address::generate(&env);

    client.propose_admin_transfer(&admin, &new_admin);
    assert!(client.get_pending_admin_transfer().is_some());

    client.accept_admin_transfer(&new_admin);
    assert_eq!(client.get_pending_admin_transfer(), None);
}
