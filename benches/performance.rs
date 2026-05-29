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
fn benchmark_all() {
    benchmark_create_attestation();
    benchmark_revoke_attestation();
    benchmark_has_valid_claim_short_circuit();
    benchmark_get_subject_attestations();

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
