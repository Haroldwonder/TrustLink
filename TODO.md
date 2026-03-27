# TODO: Implement Attestation Search by Claim Type (#24)

## Steps:
- [ ] 1. Add StorageKey::ClaimTypeAttestations(String) in src/storage.rs
- [ ] 2. Add get_claim_type_attestations, add_claim_type_attestation in src/storage.rs
- [ ] 3. Update store_attestation in src/lib.rs to index claim_type
- [ ] 4. Add get_subjects_by_claim_type in src/lib.rs impl
- [ ] 5. Add unit tests in src/test.rs
- [ ] 6. Update README.md
- [ ] 7. cargo test
- [ ] 8. git commit, gh pr create

Current: Step 1.

