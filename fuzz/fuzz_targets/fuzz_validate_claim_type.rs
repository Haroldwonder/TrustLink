#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use soroban_sdk::{Env, String};
use trustlink::Validation;

fuzz_target!(|data: Vec<u8>| {
    let env = Env::default();
    let claim_type = String::from_bytes(&env, data.as_slice());
    let _ = Validation::validate_claim_type(&claim_type);
});
