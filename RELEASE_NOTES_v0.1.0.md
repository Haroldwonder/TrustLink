# TrustLink v0.1.0

First official release of TrustLink, providing a reusable on-chain attestation and verification layer for Stellar Soroban applications.

## Highlights

- Trusted issuer registry with admin-managed access control
- Deterministic attestation creation with revocation and status tracking
- Claim verification APIs for single and multi-claim checks
- Claim type registry and paginated query interfaces
- Historical attestation imports with provenance metadata
- Bridge contract integration for cross-chain attestation mirroring
- Configurable token-denominated attestation fees
- Expiration hook callbacks for proactive renewal workflows
- Multi-signature attestation proposals and cosign activation
- Integration examples for KYC-gated token transfers and governance voter eligibility

## Included Artifact

- `trustlink.wasm` (Soroban contract artifact)

## Testnet Deployment

- Contract ID: `REPLACE_WITH_TESTNET_CONTRACT_ID`

## Verification Checklist

- `cargo test` passes with zero failures
- `cargo build --target wasm32-unknown-unknown --release` succeeds
- Git tag `v0.1.0` created and pushed
- Release artifact attached to GitHub Release
