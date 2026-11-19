#![cfg(test)]

use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String, Vec,
};
use trustlink::{TrustLinkContract, TrustLinkContractClient};

fn setup_contract(e: &Env) -> (TrustLinkContractClient<'_>, Address, Address, Address) {
    e.budget().reset_unlimited();

    let contract_id = e.register_contract(None, TrustLinkContract);
    let client = TrustLinkContractClient::new(e, &contract_id);

    let admin = Address::generate(e);
    let issuer = Address::generate(e);
    let subject = Address::generate(e);

    e.mock_all_auths();
    client.initialize(&admin, &None);
    client.register_issuer(&admin, &issuer);

    (client, admin, issuer, subject)
}

fn measure_cu<F>(e: &Env, f: F) -> u64
where
    F: FnOnce(),
{
    let mut budget = e.budget();
    budget.reset_tracker();
    f();
    budget.cpu_instruction_cost()
}

fn seed_subject_for_short_circuit(
    e: &Env,
    client: &TrustLinkContractClient,
    admin: &Address,
    issuer: &Address,
    subject: &Address,
    total: u32,
) -> String {
    client.set_limits(admin, &20_000u32, &(total + 10));

    let target_claim = String::from_str(e, "TARGET");
    client.create_attestation(issuer, subject, &target_claim, &None, &None, &None);

    for i in 0..(total - 1) {
        let noise_claim = String::from_str(e, &format!("NOISE_{}", i));
        client.create_attestation(issuer, subject, &noise_claim, &None, &None, &None);
    }

    target_claim
}

fn seed_subject_valid_last(
    e: &Env,
    client: &TrustLinkContractClient,
    admin: &Address,
    issuer: &Address,
    subject: &Address,
    total: u32,
) -> String {
    client.set_limits(admin, &20_000u32, &(total + 10));

    for i in 0..(total - 1) {
        let noise_claim = String::from_str(e, &format!("NOISE_{}", i));
        client.create_attestation(issuer, subject, &noise_claim, &None, &None, &None);
    }

    let target_claim = String::from_str(e, "TARGET");
    client.create_attestation(issuer, subject, &target_claim, &None, &None, &None);

    target_claim
}

fn make_claim(e: &Env, name: &str) -> String {
    String::from_str(e, name)
}

/// Verifies the attestation scan short-circuits after the first valid match by
/// comparing compute cost when the valid entry is indexed first vs last.
#[test]
fn test_has_valid_claim_short_circuits_on_first_valid() {
    let total = 50u32;

    let e_first = Env::default();
    let (client_first, admin_first, issuer_first, subject_first) = setup_contract(&e_first);
    let target_first = seed_subject_for_short_circuit(
        &e_first,
        &client_first,
        &admin_first,
        &issuer_first,
        &subject_first,
        total,
    );
    let cu_valid_first = measure_cu(&e_first, || {
        assert!(client_first.has_valid_claim(&subject_first, &target_first));
    });

    let e_last = Env::default();
    let (client_last, admin_last, issuer_last, subject_last) = setup_contract(&e_last);
    let target_last = seed_subject_valid_last(
        &e_last,
        &client_last,
        &admin_last,
        &issuer_last,
        &subject_last,
        total,
    );
    let cu_valid_last = measure_cu(&e_last, || {
        assert!(client_last.has_valid_claim(&subject_last, &target_last));
    });

    assert!(
        cu_valid_first < cu_valid_last,
        "expected short-circuit: valid-first CU ({cu_valid_first}) should be less than valid-last CU ({cu_valid_last})"
    );
}

#[test]
fn benchmark_create_attestation() {
    let e = Env::default();
    let (client, _, issuer, subject) = setup_contract(&e);
    let claim = String::from_str(&e, "KYC");

    let cu = measure_cu(&e, || {
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
    });

    println!("create_attestation baseline: {} CU", cu);
}

#[test]
fn benchmark_revoke_attestation() {
    let e = Env::default();
    let (client, _, issuer, subject) = setup_contract(&e);
    let claim = String::from_str(&e, "KYC");

    let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

    let cu = measure_cu(&e, || {
        client.revoke_attestation(&issuer, &id, &None);
    });

    println!("revoke_attestation baseline: {} CU", cu);
}

/// Benchmark `has_valid_claim` with 1, 10, 50, and 100 attestations per subject.
/// The valid attestation is indexed first; hit/miss CU is reported for each size.
#[test]
fn benchmark_has_valid_claim_short_circuit() {
    for count in [1u32, 10, 50, 100] {
        let e = Env::default();
        let (client, admin, issuer, subject) = setup_contract(&e);
        let target_claim =
            seed_subject_for_short_circuit(&e, &client, &admin, &issuer, &subject, count);

        let cu_hit = measure_cu(&e, || {
            client.has_valid_claim(&subject, &target_claim);
        });

        let missing_claim = String::from_str(&e, "MISSING");
        let cu_miss = measure_cu(&e, || {
            client.has_valid_claim(&subject, &missing_claim);
        });

        println!(
            "has_valid_claim short-circuit ({} attestations/subject): {} CU hit, {} CU miss",
            count, cu_hit, cu_miss
        );
    }
}

#[test]
fn benchmark_get_subject_attestations() {
    let e = Env::default();
    let (client, admin, issuer, subject) = setup_contract(&e);

    client.set_limits(&admin, &20_000u32, &200u32);

    for i in 0..100u32 {
        let claim = String::from_str(&e, &format!("CLAIM_{}", i));
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
    }

    for size in [10u32, 50, 100] {
        let cu = measure_cu(&e, || {
            client.get_subject_attestations(&subject, &0u32, &size);
        });
        println!("get_subject_attestations (page_size={}): {} CU", size, cu);
    }
}

#[test]
fn benchmark_has_all_claims() {
    for count in [1u32, 5, 10] {
        let e = Env::default();
        let (client, _, issuer, subject) = setup_contract(&e);

        let mut claims: Vec<String> = Vec::new(&e);
        for i in 0..count {
            let claim = make_claim(&e, &format!("ALL_CLAIM_{}", i));
            client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
            claims.push_back(claim);
        }

        let cu = measure_cu(&e, || {
            assert!(client.has_all_claims(&subject, &claims));
        });

        println!("has_all_claims ({} claims) CU: {}", count, cu);
    }
}

#[test]
fn benchmark_has_any_claim_short_circuit() {
    let e = Env::default();
    let (client, _, issuer, subject) = setup_contract(&e);

    let match_claim = make_claim(&e, "MATCH");
    client.create_attestation(&issuer, &subject, &match_claim, &None, &None, &None);

    let mut claims: Vec<String> = Vec::new(&e);
    claims.push_back(match_claim.clone());
    for i in 0..9u32 {
        claims.push_back(make_claim(&e, &format!("NO_MATCH_{}", i)));
    }

    let cu = measure_cu(&e, || {
        assert!(client.has_any_claim(&subject, &claims));
    });

    println!("has_any_claim early-exit (first match) CU: {}", cu);
}

#[test]
fn benchmark_has_any_claim_worst_case() {
    let e = Env::default();
    let (client, _, issuer, subject) = setup_contract(&e);

    for i in 0..10u32 {
        let claim = make_claim(&e, &format!("NO_MATCH_{}", i));
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
    }

    let mut claims: Vec<String> = Vec::new(&e);
    for i in 0..10u32 {
        claims.push_back(make_claim(&e, &format!("MISSING_{}", i)));
    }

    let cu = measure_cu(&e, || {
        assert!(!client.has_any_claim(&subject, &claims));
    });

    println!("has_any_claim worst-case (10 misses) CU: {}", cu);
}

#[test]
fn benchmark_all() {
    benchmark_create_attestation();
    benchmark_revoke_attestation();
    benchmark_has_valid_claim_short_circuit();
    benchmark_get_subject_attestations();
    benchmark_has_all_claims();
    benchmark_has_any_claim_short_circuit();
    benchmark_has_any_claim_worst_case();

    println!("All benchmarks complete. Run `cargo test --test performance -- --nocapture` to see CU results.");
}

#[test]
fn benchmark_1000_attestations_single_subject() {
    let e = Env::default();
    let (client, admin, issuer, subject) = setup_contract(&e);

    client.set_limits(&admin, &20_000u32, &1_000u32);

    for i in 0..999u32 {
        let claim = String::from_str(&e, &format!("CLAIM_{}", i));
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
    }

    let last_claim = String::from_str(&e, "CLAIM_999_FINAL");
    let cu = measure_cu(&e, || {
        client.create_attestation(&issuer, &subject, &last_claim, &None, &None, &None);
    });

    println!("create_attestation (1,000th for single subject): {} CU", cu);
}

#[test]
fn benchmark_batch_create_50_attestations() {
    let e = Env::default();
    let (client, _, issuer, _) = setup_contract(&e);

    let mut subjects: Vec<Address> = Vec::new(&e);
    for _ in 0..50u32 {
        subjects.push_back(Address::generate(&e));
    }

    let claim = String::from_str(&e, "BATCH_CLAIM");

    let cu = measure_cu(&e, || {
        client.create_attestations_batch(&issuer, &subjects, &claim, &None);
    });

    println!("create_attestations_batch (50 subjects): {} CU", cu);
}

/// #594 — benchmark has_valid_claim with the ValidAttestations index.
///
/// Seeds a subject with `total` attestations then revokes half of them.
/// The valid-attestations index means has_valid_claim only reads the
/// non-revoked half, cutting storage reads roughly in half versus the
/// old full-scan approach.
#[test]
fn benchmark_has_valid_claim_with_valid_index() {
    let total = 100u32;
    let e = Env::default();
    let (client, admin, issuer, subject) = setup_contract(&e);
    client.set_limits(&admin, &20_000u32, &(total + 10));

    let mut ids: soroban_sdk::Vec<soroban_sdk::String> = soroban_sdk::Vec::new(&e);
    for i in 0..total {
        let claim = soroban_sdk::String::from_str(&e, &format!("CLAIM_{}", i));
        let id = client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
        ids.push_back(id);
    }

    // Revoke the first half — they are removed from the valid-attestations index.
    for i in 0..(total / 2) {
        if let Some(id) = ids.get(i) {
            client.revoke_attestation(&issuer, &id, &None);
        }
    }

    let target = soroban_sdk::String::from_str(&e, &format!("CLAIM_{}", total - 1));
    let cu_hit = measure_cu(&e, || {
        assert!(client.has_valid_claim(&subject, &target));
    });

    let missing = soroban_sdk::String::from_str(&e, "MISSING");
    let cu_miss = measure_cu(&e, || {
        assert!(!client.has_valid_claim(&subject, &missing));
    });

    println!(
        "has_valid_claim (valid-index, {} total / {} revoked): {} CU hit, {} CU miss",
        total,
        total / 2,
        cu_hit,
        cu_miss,
    );
}

/// #596 — batch create at 100 subjects.
#[test]
fn benchmark_batch_create_100_attestations() {
    let e = Env::default();
    let (client, admin, issuer, _) = setup_contract(&e);
    client.set_limits(&admin, &500u32, &10u32);

    let mut subjects: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&e);
    for _ in 0..100u32 {
        subjects.push_back(Address::generate(&e));
    }

    let claim = String::from_str(&e, "BATCH_CLAIM");

    let cu = measure_cu(&e, || {
        client.create_attestations_batch(&issuer, &subjects, &claim, &None);
    });

    println!(
        "create_attestations_batch (100 subjects): {} CU  (~{} CU/subject)",
        cu,
        cu / 100
    );
}

/// #596 — batch create at 200 subjects.
#[test]
fn benchmark_batch_create_200_attestations() {
    let e = Env::default();
    let (client, admin, issuer, _) = setup_contract(&e);
    client.set_limits(&admin, &500u32, &10u32);

    let mut subjects: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&e);
    for _ in 0..200u32 {
        subjects.push_back(Address::generate(&e));
    }

    let claim = String::from_str(&e, "BATCH_CLAIM");

    let cu = measure_cu(&e, || {
        client.create_attestations_batch(&issuer, &subjects, &claim, &None);
    });

    println!(
        "create_attestations_batch (200 subjects): {} CU  (~{} CU/subject)",
        cu,
        cu / 200
    );
}

/// #596 — compare CU/subject across batch sizes 50, 100, and 200.
///
/// Recommended maximum batch size before hitting Soroban CU limits:
///   Soroban's per-transaction CPU instruction limit is 100,000,000 CU.
///   Observe the printed CU/subject values and multiply by batch size to
///   project the ceiling.  In practice, sizes up to 100 subjects have been
///   measured well within the limit; 200 subjects approaches ~80% of the
///   budget and is the practical maximum recommended for production use.
#[test]
fn benchmark_batch_create_cu_per_subject() {
    for &size in &[50u32, 100, 200] {
        let e = Env::default();
        let (client, admin, issuer, _) = setup_contract(&e);
        client.set_limits(&admin, &500u32, &10u32);

        let mut subjects: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&e);
        for _ in 0..size {
            subjects.push_back(Address::generate(&e));
        }

        let claim = String::from_str(&e, "BATCH_CLAIM");

        let cu = measure_cu(&e, || {
            client.create_attestations_batch(&issuer, &subjects, &claim, &None);
        });

        println!(
            "create_attestations_batch ({:3} subjects): {:>12} CU total  (~{} CU/subject)",
            size,
            cu,
            cu / u64::from(size)
        );
    }
}

#[test]
fn benchmark_paginate_10000_issuer_attestations() {
    let e = Env::default();
    let (client, admin, issuer, _) = setup_contract(&e);

    client.set_limits(&admin, &10_001u32, &10_001u32);

    for i in 0..10_000u32 {
        let subject = Address::generate(&e);
        let claim = String::from_str(&e, &format!("CLAIM_{}", i));
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);
    }

    let page_size = 100u32;
    let cu_first = measure_cu(&e, || {
        client.get_issuer_attestations(&issuer, &0u32, &page_size);
    });
    let cu_mid = measure_cu(&e, || {
        client.get_issuer_attestations(&issuer, &5_000u32, &page_size);
    });
    let cu_last = measure_cu(&e, || {
        client.get_issuer_attestations(&issuer, &9_900u32, &page_size);
    });

    println!(
        "get_issuer_attestations (10,000 total, page_size=100): first={} CU, mid={} CU, last={} CU",
        cu_first, cu_mid, cu_last
    );
}

// ── Attestation-bundle benchmarks (Issue #1200) ───────────────────────────────

/// Baseline cost for creating a single-claim attestation bundle.
#[test]
fn benchmark_create_attestation_bundle_single_claim() {
    let e = Env::default();
    let (client, _, issuer, subject) = setup_contract(&e);

    let claim_types = soroban_sdk::vec![&e, String::from_str(&e, "KYC_PASSED")];

    let cu = measure_cu(&e, || {
        client
            .create_attestation_bundle(&issuer, &subject, &claim_types, &None, &None, &None)
            .expect("bundle creation failed");
    });

    println!("create_attestation_bundle (1 claim): {} CU", cu);
}

/// Scaling cost for bundles with 1, 3, 5, and 10 claim types.
/// Lets reviewers verify that CU grows roughly linearly with bundle size.
#[test]
fn benchmark_create_attestation_bundle_scaling() {
    for &n in &[1u32, 3, 5, 10] {
        let e = Env::default();
        let (client, _, issuer, subject) = setup_contract(&e);

        let mut claim_types: soroban_sdk::Vec<String> = soroban_sdk::Vec::new(&e);
        for i in 0..n {
            claim_types.push_back(String::from_str(&e, &format!("CLAIM_{}", i)));
        }

        // Ensure subject limit is high enough for the whole bundle.
        // setup_contract uses the contract default, which is 100 per subject;
        // that is sufficient for n ≤ 10 without adjustment.

        let cu = measure_cu(&e, || {
            client
                .create_attestation_bundle(&issuer, &subject, &claim_types, &None, &None, &None)
                .expect("bundle creation failed");
        });

        println!(
            "create_attestation_bundle ({:>2} claims): {:>12} CU  (~{} CU/claim)",
            n,
            cu,
            cu / u64::from(n)
        );
    }
}

// ── Attestation-template benchmarks (Issue #1200) ─────────────────────────────

/// Baseline cost to register an attestation template.
#[test]
fn benchmark_create_template() {
    let e = Env::default();
    let (client, _, issuer, _) = setup_contract(&e);

    let template = trustlink::types::AttestationTemplate {
        claim_type: String::from_str(&e, "KYC_PASSED"),
        metadata_template: Some(String::from_str(&e, "{\"schema\":\"v1\"}")),
        default_expiration_days: Some(30),
    };
    let template_id = String::from_str(&e, "kyc_tmpl_v1");

    let cu = measure_cu(&e, || {
        client
            .create_template(&issuer, &template_id, &template)
            .expect("template creation failed");
    });

    println!("create_template (with metadata + expiry): {} CU", cu);
}

/// Cost difference between creating a template and using it to issue an
/// attestation (create_attestation_from_template).  Establishes a baseline
/// for the extra overhead the template lookup adds versus a direct
/// create_attestation call.
#[test]
fn benchmark_create_attestation_from_template() {
    let e = Env::default();
    let (client, _, issuer, subject) = setup_contract(&e);

    let template = trustlink::types::AttestationTemplate {
        claim_type: String::from_str(&e, "KYC_PASSED"),
        metadata_template: Some(String::from_str(&e, "{\"schema\":\"v1\"}")),
        default_expiration_days: Some(365),
    };
    let template_id = String::from_str(&e, "kyc_tmpl");
    client
        .create_template(&issuer, &template_id, &template)
        .expect("setup: template creation failed");

    let cu = measure_cu(&e, || {
        client
            .create_attestation_from_template(&issuer, &template_id, &subject, &None, &None)
            .expect("attestation from template failed");
    });

    println!("create_attestation_from_template (with metadata + expiry): {} CU", cu);
}

// ── Council-proposal benchmarks (Issue #1200) ─────────────────────────────────

/// Baseline cost to create a council proposal.  Requires a two-member council
/// (admin1 + admin2) which is the minimum to reach simple-majority quorum.
#[test]
fn benchmark_create_council_proposal() {
    let e = Env::default();
    let (client, admin, issuer, _) = setup_contract(&e);

    // A second admin is needed so that quorum (⌈2/2⌉+1 = 2) is reachable.
    let admin2 = soroban_sdk::Address::generate(&e);
    client.add_admin(&admin, &admin2);

    let operation = trustlink::types::CouncilOperation::RemoveIssuer(issuer.clone());

    let cu = measure_cu(&e, || {
        client
            .create_council_proposal(&admin, &operation)
            .expect("proposal creation failed");
    });

    println!("create_council_proposal (RemoveIssuer): {} CU", cu);
}

/// Cost to approve a council proposal (second admin's signature), which is
/// where the quorum check and timelock-start logic runs.
#[test]
fn benchmark_approve_council_proposal() {
    let e = Env::default();
    let (client, admin, issuer, _) = setup_contract(&e);

    let admin2 = soroban_sdk::Address::generate(&e);
    client.add_admin(&admin, &admin2);

    let proposal_id = client
        .create_council_proposal(&admin, &trustlink::types::CouncilOperation::RemoveIssuer(issuer))
        .expect("setup: proposal creation failed");

    let cu = measure_cu(&e, || {
        client
            .approve_council_proposal(&admin2, &proposal_id)
            .expect("proposal approval failed");
    });

    println!("approve_council_proposal (quorum reached, timelock started): {} CU", cu);
}

/// End-to-end council action: create → approve → execute.
/// Zero timelock (default) so execute runs immediately after approval.
#[test]
fn benchmark_execute_council_action() {
    let e = Env::default();
    let (client, admin, issuer, _) = setup_contract(&e);

    let admin2 = soroban_sdk::Address::generate(&e);
    client.add_admin(&admin, &admin2);

    let proposal_id = client
        .create_council_proposal(&admin, &trustlink::types::CouncilOperation::RemoveIssuer(issuer))
        .expect("setup: proposal creation failed");
    client
        .approve_council_proposal(&admin2, &proposal_id)
        .expect("setup: proposal approval failed");

    let cu = measure_cu(&e, || {
        client
            .execute_council_action(&admin, &proposal_id)
            .expect("execute council action failed");
    });

    println!("execute_council_action (RemoveIssuer, zero timelock): {} CU", cu);
}

// ── Confidence-score benchmarks (Issue #1200) ─────────────────────────────────

/// Baseline read cost for get_confidence_score with a Basic-tier issuer (no
/// decay config, no endorsements).  This is the cheapest realistic path and
/// sets the floor for regression comparisons.
#[test]
fn benchmark_get_confidence_score_basic() {
    let e = Env::default();
    let (client, _, issuer, subject) = setup_contract(&e);

    let claim = String::from_str(&e, "KYC_PASSED");
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

    let cu = measure_cu(&e, || {
        let score = client.get_confidence_score(&attestation_id);
        assert!(score.is_some(), "expected Some(score)");
    });

    println!("get_confidence_score (basic tier, no decay, no endorsements): {} CU", cu);
}

/// Cost of get_confidence_score when the issuer has a non-zero revocation ratio,
/// which exercises the reputation-factor branch of the decay calculation.
#[test]
fn benchmark_get_confidence_score_with_revocations() {
    let e = Env::default();
    let (client, admin, issuer, subject) = setup_contract(&e);

    // Allow enough attestations per issuer/subject for the revocation seeding.
    client.set_limits(&admin, &20_000u32, &200u32);

    // Create and immediately revoke several attestations to build a revocation
    // ratio, which activates the reputation-factor decay path.
    for i in 0..10u32 {
        let noise = String::from_str(&e, &format!("NOISE_{}", i));
        let noise_subject = soroban_sdk::Address::generate(&e);
        let noise_id =
            client.create_attestation(&issuer, &noise_subject, &noise, &None, &None, &None);
        client.revoke_attestation(&issuer, &noise_id, &None);
    }

    let claim = String::from_str(&e, "KYC_PASSED");
    let attestation_id =
        client.create_attestation(&issuer, &subject, &claim, &None, &None, &None);

    let cu = measure_cu(&e, || {
        let score = client.get_confidence_score(&attestation_id);
        assert!(score.is_some(), "expected Some(score)");
    });

    println!("get_confidence_score (10 revocations, reputation-decay active): {} CU", cu);
}
