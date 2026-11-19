//! # Carbon-credit / ESG offset attestation — a *verifier-of-verifiers* example
//!
//! This example models a "chain of trust about trust" on top of TrustLink:
//!
//! ```text
//!   Registry ──AUDITOR_ACCREDITED──▶ Auditor ──OFFSET_VERIFIED──▶ Project
//!   (root of trust)                  (accredited)                 (offset claim)
//! ```
//!
//! Unlike a single-party provenance model (e.g. a supply-chain example where one
//! issuer vouches directly for a subject), a carbon-offset claim is only
//! trustworthy if **both** links hold:
//!
//! 1. The **project** carries a valid `OFFSET_VERIFIED` attestation, and
//! 2. the **auditor** that issued it is itself carrying a valid
//!    `AUDITOR_ACCREDITED` attestation from a **registry** we recognise as the
//!    root of trust.
//!
//! The [`CarbonVerifier`] contract walks that chain in reverse
//! (`project → auditor → registry`) using cross-contract calls into TrustLink,
//! so any dApp can trust an offset claim with a single call.
//!
//! ## Running
//!
//! ```bash
//! cargo run --example carbon-credit-verification
//! # or
//! ./examples/carbon-credit-verification/run.sh
//! ```
//!
//! The program `assert!`s each step, so a non-zero exit means the chain failed
//! to validate — i.e. the example doubles as an end-to-end validation script.

use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as _,
    Address, Env, String,
};
use trustlink::{TrustLinkContract, TrustLinkContractClient};

/// Claim type a registry issues to an auditor it has vetted.
const AUDITOR_ACCREDITED: &str = "AUDITOR_ACCREDITED";
/// Claim type an accredited auditor issues to a specific offset project.
const OFFSET_VERIFIED: &str = "OFFSET_VERIFIED";

/// A minimal on-chain "verifier of verifiers".
///
/// It does not store anything itself — it composes existing TrustLink
/// attestations into a two-tier trust decision.
#[contract]
pub struct CarbonVerifier;

#[contractimpl]
impl CarbonVerifier {
    /// Return `true` only if `project`'s offset claim can be traced all the way
    /// back to `expected_registry` through an accredited auditor.
    ///
    /// The check is deliberately conservative:
    /// - the project's most-recent **valid** `OFFSET_VERIFIED` attestation
    ///   decides *which* auditor is being trusted, and
    /// - that auditor must itself hold a **valid** `AUDITOR_ACCREDITED`
    ///   attestation issued by `expected_registry`.
    ///
    /// Because TrustLink's `get_attestation_by_type` only ever returns a
    /// `Valid` attestation, revoked or expired links anywhere in the chain make
    /// this return `false` automatically.
    pub fn verify_offset(
        env: Env,
        trustlink: Address,
        project: Address,
        expected_registry: Address,
    ) -> bool {
        let client = TrustLinkContractClient::new(&env, &trustlink);

        let offset_claim = String::from_str(&env, OFFSET_VERIFIED);
        let accredited_claim = String::from_str(&env, AUDITOR_ACCREDITED);

        // Tier 1 — the project must carry a valid offset attestation.
        // Its issuer is the auditor whose credibility we now have to check.
        let auditor = match client.try_get_attestation_by_type(&project, &offset_claim) {
            Ok(Ok(offset)) => offset.issuer,
            _ => return false,
        };

        // Tier 2 — that auditor must itself be accredited, and the accreditation
        // must originate from the registry we treat as the root of trust.
        match client.try_get_attestation_by_type(&auditor, &accredited_claim) {
            Ok(Ok(accreditation)) => accreditation.issuer == expected_registry,
            _ => false,
        }
    }
}

fn main() {
    let env = Env::default();
    // In this demo every actor authorises freely; on-chain each `require_auth`
    // would be satisfied by the actor's own signature.
    env.mock_all_auths();

    // --- Deploy contracts -------------------------------------------------
    let trustlink_id = env.register_contract(None, TrustLinkContract);
    let trustlink = TrustLinkContractClient::new(&env, &trustlink_id);

    let verifier_id = env.register_contract(None, CarbonVerifier);
    let verifier = CarbonVerifierClient::new(&env, &verifier_id);

    // --- Actors -----------------------------------------------------------
    let admin = Address::generate(&env); // TrustLink administrator
    let registry = Address::generate(&env); // carbon registry — root of trust
    let auditor = Address::generate(&env); // an auditor the registry accredits
    let rogue_auditor = Address::generate(&env); // an *unaccredited* auditor
    let project = Address::generate(&env); // the carbon-offset project

    trustlink.initialize(&admin);

    // The registry and the auditors are all TrustLink issuers: the registry
    // issues accreditations, the auditors issue offset verifications.
    trustlink.register_issuer(&admin, &registry);
    trustlink.register_issuer(&admin, &auditor);
    trustlink.register_issuer(&admin, &rogue_auditor);

    let accredited = String::from_str(&env, AUDITOR_ACCREDITED);
    let offset = String::from_str(&env, OFFSET_VERIFIED);

    println!("== Carbon-credit verifier-of-verifiers demo ==");

    // 1. Nothing issued yet — the chain is empty, so the claim is untrusted.
    assert!(
        !verifier.verify_offset(&trustlink_id, &project, &registry),
        "empty chain must not verify"
    );
    println!("[1] no attestations yet            -> untrusted  (ok)");

    // 2. The registry accredits the auditor (tier-2 link).
    trustlink.create_attestation(
        &registry,
        &auditor,
        &accredited,
        &None,
        &Some(String::from_str(&env, "Accredited under ISO 14064-3")),
    );
    println!("[2] registry accredits auditor");

    // 3. The accredited auditor verifies the project's offset (tier-1 link).
    trustlink.create_attestation(
        &auditor,
        &project,
        &offset,
        &None,
        &Some(String::from_str(&env, "1,000 tCO2e reforestation, vintage 2026")),
    );
    println!("[3] auditor verifies offset");

    // 4. Full chain project -> auditor -> registry now resolves.
    assert!(
        verifier.verify_offset(&trustlink_id, &project, &registry),
        "complete accredited chain must verify"
    );
    println!("[4] full accredited chain          -> TRUSTED    (ok)");

    // 5. A rogue, *unaccredited* auditor also verifies the same project.
    //    `get_attestation_by_type` returns the most recent valid offset, so the
    //    verifier now follows the rogue auditor — who has no accreditation — and
    //    must reject the claim. Trust cannot be laundered through a fresh signer.
    trustlink.create_attestation(
        &rogue_auditor,
        &project,
        &offset,
        &None,
        &Some(String::from_str(&env, "unaudited self-report")),
    );
    assert!(
        !verifier.verify_offset(&trustlink_id, &project, &registry),
        "offset from an unaccredited auditor must not verify"
    );
    println!("[5] newest offset from rogue auditor -> untrusted (ok)");

    // 6. The registry revokes the real auditor's accreditation (e.g. audit
    //    fraud discovered). Even the originally-valid offset now fails, because
    //    the tier-2 link is gone.
    let auditor_attestations = trustlink.get_subject_attestations(&auditor, &0, &10);
    let accreditation_id = auditor_attestations
        .get(0)
        .expect("auditor should have one accreditation");
    trustlink.revoke_attestation(&registry, &accreditation_id);

    // Re-point tier-1 at the honest auditor by revoking the rogue offset, so we
    // isolate the effect of the revoked accreditation.
    let project_attestations = trustlink.get_subject_attestations(&project, &0, &10);
    // index 1 is the rogue offset (index 0 is the honest auditor's offset).
    let rogue_offset_id = project_attestations
        .get(1)
        .expect("project should have two offset attestations");
    trustlink.revoke_attestation(&rogue_auditor, &rogue_offset_id);

    assert!(
        !verifier.verify_offset(&trustlink_id, &project, &registry),
        "revoked accreditation must break the chain"
    );
    println!("[6] auditor accreditation revoked  -> untrusted  (ok)");

    println!("\nAll checks passed — verifier-of-verifiers chain validated end-to-end.");
}
