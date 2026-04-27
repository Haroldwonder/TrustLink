
#![cfg(test)]


use soroban_sdk::{testutils::{budget, Address as _, Ledger as _, Logs}, Address, Env, String, Vec};
use trustlink::TrustLinkContractClient;

mod trustlink {
    soroban_sdk::contractimport!(file = "../target/soroban_output/trustlink.wasm");
}


e: &Env

    let contract_id = e.register_contract(None, trustlink::TrustLinkContract);
    let client = TrustLinkContractClient::new(e, &amp;contract_id);
    
    let admin = Address::generate(e);
    let issuer = Address::generate(e);
    let subject = Address::generate(e);
    
    e.mock_all_auths();
    client.initialize(e, admin.clone(), None);
    client.register_issuer(e, admin, issuer.clone());
    
    (client, admin, issuer, subject)
}

fn measure_cu<F>(e: &amp;mut Env, f: F) -> u64 
where F: FnOnce() {
    budget::budget(e);
    let start = budget::consume(e);
    f();
    budget::used_compute(e) - start
}

#[test]
fn benchmark_create_attestation() {
    let mut e = Env::default();
    let (client, _, issuer, subject) = setup_contract(&amp;e);
    let claim = String::from_str(&amp;e, "KYC");
    
    let cu = measure_cu(&amp;mut e, || {
        client.create_attestation(&amp;issuer, &amp;subject, claim.clone(), None, None, None);
    });
    
    println!("create_attestation baseline: {} CU", cu);
}

#[test]
fn benchmark_revoke_attestation() {
    let mut e = Env::default();
    let (client, _, issuer, subject) = setup_contract(&amp;e);
    let claim = String::from_str(&amp;e, "KYC");
    
    let id = client.create_attestation(&amp;issuer, &amp;subject, claim, None, None, None);
    
    let cu = measure_cu(&amp;mut e, || {
        client.revoke_attestation(&amp;issuer, id.clone(), None);
    });
    
    println!("revoke_attestation baseline: {} CU", cu);
}

#[test]
fn benchmark_has_valid_claim() {
    let mut e = Env::default();
    let (mut client, _, issuer, subject) = setup_contract(&amp;e);
    
    // Create noise attestations (different claims)
    for i in 0..100u32 {
        let noise_claim = String::from_str(&amp;e, &amp;format!("NOISE_{}", i));
        client.create_attestation(&amp;issuer, &amp;subject, noise_claim, None, None, None);
    }
    
    // Create 1 valid target claim
    let target_claim = String::from_str(&amp;e, "TARGET");
    let target_id = client.create_attestation(&amp;issuer, &amp;subject, target_claim.clone(), None, None, None);
    
    // Valid case
    let cu_valid = measure_cu(&amp;mut e, || {
        client.has_valid_claim(&amp;subject, target_claim.clone());
    });
    
    // Invalid case (non-existent claim)
    let invalid_claim = String::from_str(&amp;e, "INVALID");
    let cu_invalid = measure_cu(&amp;mut e, || {
        client.has_valid_claim(&amp;subject, invalid_claim);
    });
    
    println!("has_valid_claim (100 noise +1 valid): {} CU valid, {} CU invalid", cu_valid, cu_invalid);
}

#[test]
fn benchmark_get_subject_attestations() {
    let mut e = Env::default();
    let (mut client, _, issuer, subject) = setup_contract(&amp;e);
    
    // Create 100 attestations
    for i in 0..100u32 {
        let claim = String::from_str(&amp;e, &amp;format!("CLAIM_{}", i));
        client.create_attestation(&amp;issuer, &amp;subject, claim, None, None, None);
    }
    
    let sizes = vec![10u32, 50, 100];
    for size in sizes {
        let cu = measure_cu(&amp;mut e, || {
            client.get_subject_attestations(&amp;subject, 0u32, size);
        });
        println!("get_subject_attestations (page_size={}): {} CU", size, cu);
    }
}

#[test]
fn benchmark_all() {
    benchmark_create_attestation();
    benchmark_revoke_attestation();
    benchmark_has_valid_claim();
    benchmark_get_subject_attestations();
    
    println!("All benchmarks complete. Run `cargo test benches:: -- --nocapture` to see CU results.");
}


#[test]
fn benchmark_1000_attestations_single_subject() {
    let mut e = Env::default();
    let (client, admin, issuer, subject) = setup_contract(&e);

    // Raise per-subject limit to accommodate 1,000 attestations
    client.set_limits(&admin, &20_000u32, &1_000u32);

    // Pre-create 999 attestations outside the measured window
    for i in 0..999u32 {
        let claim = String::from_str(&e, &format!("CLAIM_{}", i));
        client.create_attestation(&issuer, &subject, claim, &None, &None, &None);
    }

    // Measure the cost of the 1,000th attestation
    let last_claim = String::from_str(&e, "CLAIM_999_FINAL");
    let cu = measure_cu(&mut e, || {
        client.create_attestation(&issuer, &subject, last_claim.clone(), &None, &None, &None);
    });

    println!("create_attestation (1,000th for single subject): {} CU", cu);
}

#[test]
fn benchmark_has_valid_claim_100_attestations() {
    let mut e = Env::default();
    let (client, admin, issuer, subject) = setup_contract(&e);

    // Raise per-subject limit to hold exactly 100 attestations
    client.set_limits(&admin, &20_000u32, &200u32);

    // Create 99 noise attestations on the subject
    for i in 0..99u32 {
        let noise_claim = String::from_str(&e, &format!("NOISE_{}", i));
        client.create_attestation(&issuer, &subject, noise_claim, &None, &None, &None);
    }

    // Create the 100th attestation as the target claim
    let target_claim = String::from_str(&e, "TARGET");
    client.create_attestation(&issuer, &subject, target_claim.clone(), &None, &None, &None);

    // Measure has_valid_claim when subject holds exactly 100 attestations
    let cu_hit = measure_cu(&mut e, || {
        client.has_valid_claim(&subject, target_claim.clone());
    });

    let missing_claim = String::from_str(&e, "MISSING");
    let cu_miss = measure_cu(&mut e, || {
        client.has_valid_claim(&subject, missing_claim);
    });

    println!(
        "has_valid_claim (100 attestations/subject): {} CU hit, {} CU miss",
        cu_hit, cu_miss
    );
}

#[test]
fn benchmark_batch_create_50_attestations() {
    let mut e = Env::default();
    let (client, _, issuer, _) = setup_contract(&e);

    // Build a Vec of 50 distinct subjects
    let mut subjects: Vec<Address> = Vec::new(&e);
    for _ in 0..50u32 {
        subjects.push_back(Address::generate(&e));
    }

    let claim = String::from_str(&e, "BATCH_CLAIM");

    let cu = measure_cu(&mut e, || {
        client.create_attestations_batch(&issuer, &subjects, &claim, &None);
    });

    println!("create_attestations_batch (50 subjects): {} CU", cu);
}

#[test]
fn benchmark_paginate_10000_issuer_attestations() {
    let mut e = Env::default();
    let (client, admin, issuer, _) = setup_contract(&e);

    // Raise issuer limit to hold 10,000 attestations
    client.set_limits(&admin, &10_001u32, &10_001u32);

    // Pre-create 10,000 attestations across unique subjects
    for i in 0..10_000u32 {
        let subject = Address::generate(&e);
        let claim = String::from_str(&e, &format!("CLAIM_{}", i));
        client.create_attestation(&issuer, &subject, claim, &None, &None, &None);
    }

    // Measure paginating through the full set in pages of 100
    let page_size = 100u32;
    let cu_first = measure_cu(&mut e, || {
        client.get_issuer_attestations(&issuer, &0u32, &page_size);
    });
    let cu_mid = measure_cu(&mut e, || {
        client.get_issuer_attestations(&issuer, &5_000u32, &page_size);
    });
    let cu_last = measure_cu(&mut e, || {
        client.get_issuer_attestations(&issuer, &9_900u32, &page_size);
    });

    println!(
        "get_issuer_attestations (10,000 total, page_size=100): first={} CU, mid={} CU, last={} CU",
        cu_first, cu_mid, cu_last
    );
}
