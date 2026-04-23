
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

