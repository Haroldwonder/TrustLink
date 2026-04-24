# Requirements Document

## Introduction

The attestation ID stability feature ensures that the `Attestation::generate_id` and
`Attestation::generate_bridge_id` functions in TrustLink produce byte-for-byte identical
output for the same inputs across all Soroban environment versions. A hard-coded regression
test captures a known (issuer, subject, claim_type, timestamp) tuple and its expected SHA-256
hex digest, so any accidental change to the hashing algorithm, XDR serialization order, or
hex-encoding logic causes an immediate, explicit test failure. The hash algorithm and input
encoding scheme are documented in code comments so future maintainers understand the stability
contract they must preserve.

## Glossary

- **ID_Generator**: The `Attestation::generate_id` and `Attestation::generate_bridge_id`
  functions defined in `src/types.rs` that produce deterministic attestation identifiers.
- **Attestation_ID**: A 32-character lowercase hex string produced by SHA-256 hashing the
  XDR-encoded concatenation of the attestation inputs.
- **Bridge_Attestation_ID**: A 32-character lowercase hex string produced by SHA-256 hashing
  the XDR-encoded concatenation of bridge-specific inputs (bridge address, subject, claim type,
  source chain, source tx, timestamp).
- **Regression_Test**: A Rust unit test that hard-codes known inputs and their expected output
  hash and asserts equality, serving as a canary for algorithm changes.
- **Hash_Algorithm**: SHA-256 applied to the XDR-encoded payload, with the digest truncated to
  the first 16 bytes and hex-encoded to a 32-character string.
- **Soroban_Env**: The `soroban_sdk::Env` execution environment used inside Soroban smart
  contracts.

## Requirements

### Requirement 1: Stable Standard Attestation ID

**User Story:** As a contract integrator, I want the standard attestation ID to be identical
for the same inputs regardless of the Soroban environment version, so that off-chain systems
can predict and cache attestation IDs without re-querying the chain after upgrades.

#### Acceptance Criteria

1. THE ID_Generator SHALL produce a 32-character lowercase hex string for any valid combination
   of issuer address, subject address, claim type string, and timestamp.
2. WHEN the same issuer address, subject address, claim type, and timestamp are provided,
   THE ID_Generator SHALL return the same Attestation_ID on every invocation regardless of
   Soroban_Env version.
3. THE Regression_Test SHALL hard-code at least one known (issuer, subject, claim_type,
   timestamp) input tuple and its expected Attestation_ID hex string.
4. WHEN the Regression_Test is executed, THE Regression_Test SHALL assert that the computed
   Attestation_ID equals the hard-coded expected value.
5. IF the Hash_Algorithm, XDR encoding order, or hex-encoding logic changes, THEN THE
   Regression_Test SHALL fail with a clear assertion error identifying the mismatch.

### Requirement 2: Stable Bridge Attestation ID

**User Story:** As a bridge operator, I want the bridge attestation ID to be stable across
Soroban environment versions, so that cross-chain attestation records remain consistently
addressable after contract upgrades.

#### Acceptance Criteria

1. THE ID_Generator SHALL produce a 32-character lowercase hex string for any valid combination
   of bridge address, subject address, claim type, source chain, source tx, and timestamp.
2. WHEN the same bridge address, subject address, claim type, source chain, source tx, and
   timestamp are provided, THE ID_Generator SHALL return the same Bridge_Attestation_ID on
   every invocation regardless of Soroban_Env version.
3. THE Regression_Test SHALL hard-code at least one known bridge input tuple and its expected
   Bridge_Attestation_ID hex string.
4. WHEN the Regression_Test is executed, THE Regression_Test SHALL assert that the computed
   Bridge_Attestation_ID equals the hard-coded expected value.
5. IF the Hash_Algorithm or bridge payload construction changes, THEN THE Regression_Test SHALL
   fail with a clear assertion error identifying the mismatch.

### Requirement 3: Hash Algorithm Documentation

**User Story:** As a maintainer, I want the hash algorithm and input encoding scheme documented
in code comments, so that I understand the stability contract before making changes to the ID
generation logic.

#### Acceptance Criteria

1. THE ID_Generator SHALL include inline code comments describing the Hash_Algorithm as:
   SHA-256 over the XDR-encoded concatenation of inputs, digest truncated to the first 16
   bytes, hex-encoded to a 32-character lowercase string.
2. THE ID_Generator SHALL include inline code comments listing the exact field order used to
   construct the XDR payload for both standard and bridge attestation IDs.
3. THE Regression_Test SHALL include a comment stating the expected hash value and the inputs
   used to derive it, so the test is self-documenting.

### Requirement 4: CHANGELOG Entry

**User Story:** As a project contributor, I want the CHANGELOG to note the addition of
attestation ID stability guarantees, so that consumers of the contract know when this
guarantee was introduced.

#### Acceptance Criteria

1. THE CHANGELOG SHALL contain an entry under `[Unreleased]` → `Added` documenting that
   attestation ID generation is stable across Soroban environment versions.
2. THE CHANGELOG SHALL reference the regression test as the mechanism that enforces the
   stability guarantee.
