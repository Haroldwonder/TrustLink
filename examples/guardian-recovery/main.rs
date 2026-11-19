//! # DID-style social-recovery / guardian attestation flow
//!
//! When a subject loses access to their signing key, they need a way for a set
//! of trusted **guardians** to attest to their identity and authorise migrating
//! their attestation history to a **new address**. This is distinct from the
//! delegation / sub-issuer model (ADR-008), which is about *who may issue* — here
//! we deal with *account/key recovery*.
//!
//! ```text
//!            register_guardians (threshold M-of-N)
//!   Subject ───────────────────────────────────────▶ GuardianRecovery
//!
//!   (subject loses key)
//!
//!   Guardian_1 ─ initiate_recovery(new_address) ─▶ GuardianRecovery
//!   Guardian_2 ─ approve_recovery ───────────────▶ GuardianRecovery
//!                        │ quorum reached
//!                        ▼
//!            re-link subject's valid claims ──▶ TrustLink (new_address)
//! ```
//!
//! ## Design note
//!
//! The core `TrustLinkContract` currently exposes **no** `transfer_attestation`
//! or multisig primitive, so this example implements the guardian set, the
//! M-of-N quorum, and the re-linkage entirely in a self-contained
//! [`GuardianRecovery`] contract. It touches TrustLink through its existing
//! public API only:
//!
//! - reads the subject's live claims with `get_valid_claims`, and
//! - re-issues each one to the new address with `create_attestation` (the
//!   recovery contract is registered as a TrustLink issuer, so it acts as the
//!   "recovery authority" that vouches for continuity of identity).
//!
//! No changes to the core contract are required.
//!
//! ## Running
//!
//! ```bash
//! cargo run --example guardian-recovery
//! # or
//! ./examples/guardian-recovery/run.sh
//! ```

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    testutils::Address as _,
    Address, Env, String, Vec,
};
use trustlink::{TrustLinkContract, TrustLinkContractClient};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RecoveryError {
    /// Threshold is zero or exceeds the number of guardians.
    InvalidThreshold = 1,
    /// At least one guardian is required.
    GuardiansRequired = 2,
    /// The caller is not a registered guardian for this subject.
    NotGuardian = 3,
    /// No recovery request is in progress for this subject.
    NoRecoveryRequest = 4,
    /// A recovery request is already in progress for this subject.
    RecoveryInProgress = 5,
    /// The recovery request has already been executed.
    AlreadyExecuted = 6,
}

/// Per-subject storage keys for the recovery contract.
#[contracttype]
pub enum DataKey {
    /// subject -> guardian addresses
    Guardians(Address),
    /// subject -> M-of-N threshold
    Threshold(Address),
    /// subject -> in-flight recovery request
    Recovery(Address),
}

/// An in-flight recovery request for a subject.
#[contracttype]
#[derive(Clone)]
pub struct RecoveryRequest {
    /// The TrustLink contract whose attestations will be re-linked.
    pub trustlink: Address,
    /// The address the subject's claims should be migrated to.
    pub new_address: Address,
    /// Guardians who have approved (the initiator counts as the first approval).
    pub approvals: Vec<Address>,
    /// Whether re-linkage has already run.
    pub executed: bool,
}

/// A self-contained social-recovery module built on top of TrustLink.
#[contract]
pub struct GuardianRecovery;

#[contractimpl]
impl GuardianRecovery {
    /// Register the guardian set and `threshold` for `subject`.
    ///
    /// Must be authorised by the subject while they still control their key.
    pub fn register_guardians(
        env: Env,
        subject: Address,
        guardians: Vec<Address>,
        threshold: u32,
    ) -> Result<(), RecoveryError> {
        subject.require_auth();

        if guardians.is_empty() {
            return Err(RecoveryError::GuardiansRequired);
        }
        if threshold == 0 || threshold > guardians.len() {
            return Err(RecoveryError::InvalidThreshold);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Guardians(subject.clone()), &guardians);
        env.storage()
            .persistent()
            .set(&DataKey::Threshold(subject.clone()), &threshold);
        Ok(())
    }

    /// A guardian starts a recovery to `new_address`. The initiator's approval
    /// is recorded, so a 1-of-N threshold executes immediately.
    pub fn initiate_recovery(
        env: Env,
        guardian: Address,
        subject: Address,
        trustlink: Address,
        new_address: Address,
    ) -> Result<u32, RecoveryError> {
        guardian.require_auth();
        Self::require_guardian(&env, &subject, &guardian)?;

        if let Some(existing) = Self::load_request(&env, &subject) {
            if !existing.executed {
                return Err(RecoveryError::RecoveryInProgress);
            }
        }

        let mut approvals = Vec::new(&env);
        approvals.push_back(guardian.clone());

        let request = RecoveryRequest {
            trustlink,
            new_address,
            approvals,
            executed: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Recovery(subject.clone()), &request);

        Self::maybe_execute(&env, &subject, request)
    }

    /// A guardian approves the in-flight recovery request. When the number of
    /// distinct guardian approvals reaches the threshold, the subject's valid
    /// claims are re-linked to the new address. Returns the current approval
    /// count.
    pub fn approve_recovery(
        env: Env,
        guardian: Address,
        subject: Address,
    ) -> Result<u32, RecoveryError> {
        guardian.require_auth();
        Self::require_guardian(&env, &subject, &guardian)?;

        let mut request = Self::load_request(&env, &subject).ok_or(RecoveryError::NoRecoveryRequest)?;
        if request.executed {
            return Err(RecoveryError::AlreadyExecuted);
        }

        // Ignore duplicate approvals from the same guardian.
        let mut already = false;
        for a in request.approvals.iter() {
            if a == guardian {
                already = true;
                break;
            }
        }
        if !already {
            request.approvals.push_back(guardian.clone());
        }
        env.storage()
            .persistent()
            .set(&DataKey::Recovery(subject.clone()), &request);

        Self::maybe_execute(&env, &subject, request)
    }

    /// Return the guardian set registered for `subject`.
    pub fn guardians(env: Env, subject: Address) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Guardians(subject))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Return the in-flight recovery request for `subject`, if any.
    pub fn get_recovery(env: Env, subject: Address) -> Option<RecoveryRequest> {
        Self::load_request(&env, &subject)
    }

    // --- internal helpers -------------------------------------------------

    fn load_request(env: &Env, subject: &Address) -> Option<RecoveryRequest> {
        env.storage()
            .persistent()
            .get(&DataKey::Recovery(subject.clone()))
    }

    fn require_guardian(
        env: &Env,
        subject: &Address,
        candidate: &Address,
    ) -> Result<(), RecoveryError> {
        let guardians: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Guardians(subject.clone()))
            .unwrap_or_else(|| Vec::new(env));
        for g in guardians.iter() {
            if &g == candidate {
                return Ok(());
            }
        }
        Err(RecoveryError::NotGuardian)
    }

    /// Execute re-linkage if the approval count has reached the threshold.
    /// Returns the current approval count either way.
    fn maybe_execute(
        env: &Env,
        subject: &Address,
        mut request: RecoveryRequest,
    ) -> Result<u32, RecoveryError> {
        let threshold: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Threshold(subject.clone()))
            .unwrap_or(u32::MAX);

        let approvals = request.approvals.len();
        if !request.executed && approvals >= threshold {
            Self::relink(env, subject, &request);
            request.executed = true;
            env.storage()
                .persistent()
                .set(&DataKey::Recovery(subject.clone()), &request);
        }
        Ok(approvals)
    }

    /// Re-issue every valid claim the subject holds to the new address, using
    /// TrustLink's existing `create_attestation`. The recovery contract itself
    /// is the issuer (it must be registered as a TrustLink issuer).
    fn relink(env: &Env, subject: &Address, request: &RecoveryRequest) {
        let trustlink = TrustLinkContractClient::new(env, &request.trustlink);
        let claims = trustlink.get_valid_claims(subject);
        let me = env.current_contract_address();
        let note = String::from_str(env, "re-linked via guardian recovery");

        for claim_type in claims.iter() {
            trustlink.create_attestation(
                &me,
                &request.new_address,
                &claim_type,
                &None,
                &Some(note.clone()),
            );
        }
    }
}

fn main() {
    let env = Env::default();
    env.mock_all_auths();

    // --- Deploy contracts -------------------------------------------------
    let trustlink_id = env.register_contract(None, TrustLinkContract);
    let trustlink = TrustLinkContractClient::new(&env, &trustlink_id);

    let recovery_id = env.register_contract(None, GuardianRecovery);
    let recovery = GuardianRecoveryClient::new(&env, &recovery_id);

    // --- Actors -----------------------------------------------------------
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env); // original attestation issuer (e.g. a KYC anchor)
    let subject = Address::generate(&env); // account that will lose its key
    let new_address = Address::generate(&env); // recovered-to address
    let g1 = Address::generate(&env);
    let g2 = Address::generate(&env);
    let g3 = Address::generate(&env);
    let stranger = Address::generate(&env); // not a guardian

    trustlink.initialize(&admin);
    trustlink.register_issuer(&admin, &issuer);
    // The recovery contract must be a TrustLink issuer so it can re-link claims.
    trustlink.register_issuer(&admin, &recovery_id);

    let kyc = String::from_str(&env, "KYC_PASSED");
    let accredited = String::from_str(&env, "ACCREDITED_INVESTOR");

    println!("== DID-style guardian social-recovery demo ==");

    // 1. The issuer attests the subject's identity on their original key.
    trustlink.create_attestation(&issuer, &subject, &kyc, &None, &None);
    trustlink.create_attestation(&issuer, &subject, &accredited, &None, &None);
    assert!(trustlink.has_valid_claim(&subject, &kyc));
    assert!(trustlink.has_valid_claim(&subject, &accredited));
    println!("[1] subject holds KYC_PASSED + ACCREDITED_INVESTOR");

    // 2. While still in control, the subject registers a 2-of-3 guardian set.
    let mut guardians = Vec::new(&env);
    guardians.push_back(g1.clone());
    guardians.push_back(g2.clone());
    guardians.push_back(g3.clone());
    recovery.register_guardians(&subject, &guardians, &2);
    assert_eq!(recovery.guardians(&subject).len(), 3);
    println!("[2] subject registers 2-of-3 guardians");

    // 3. Subject loses their key. A guardian initiates recovery to new_address.
    //    One approval (the initiator) is below the 2-of-3 threshold, so nothing
    //    is re-linked yet.
    let approvals = recovery.initiate_recovery(&g1, &subject, &trustlink_id, &new_address);
    assert_eq!(approvals, 1);
    assert!(!trustlink.has_valid_claim(&new_address, &kyc));
    println!("[3] guardian g1 initiates recovery       -> 1/2 approvals, not yet re-linked");

    // 4. A non-guardian cannot approve.
    assert!(recovery.try_approve_recovery(&stranger, &subject).is_err());
    println!("[4] non-guardian approval rejected");

    // 5. A second guardian approves -> quorum reached -> claims re-linked.
    let approvals = recovery.approve_recovery(&g2, &subject);
    assert_eq!(approvals, 2);
    assert!(trustlink.has_valid_claim(&new_address, &kyc));
    assert!(trustlink.has_valid_claim(&new_address, &accredited));
    println!("[5] guardian g2 approves                 -> 2/2 quorum, claims re-linked");

    // 6. The migrated claims are now live on the new address.
    let migrated = trustlink.get_valid_claims(&new_address);
    assert_eq!(migrated.len(), 2);
    println!("[6] new address now holds all {} claims", migrated.len());

    println!("\nAll checks passed — guardian recovery re-linked attestations end-to-end.");
}
