# Glossary

Key terms used throughout the TrustLink documentation and codebase.

| Term | Definition |
|---|---|
| **Attestation** | An on-chain record created by an issuer that a subject holds a specific claim (e.g. "KYC_PASSED"). |
| **Claim** | A named assertion about a subject, such as `KYC_PASSED` or `ACCREDITED_INVESTOR`. Claim types are registered before use. |
| **Issuer** | An address authorized by the admin to create attestations. Issuers can be added or removed by the admin. |
| **Subject** | The address that an attestation is issued about (e.g. the user being KYC-verified). |
| **Admin** | The address with authority to configure the contract: registering issuers, setting fees, managing bridge contracts, and transferring admin rights. |
| **Bridge Contract** | An external contract registered to import attestations that originated on another chain. |
| **Revocation** | Marking an existing attestation as no longer valid. TrustLink keeps an immutable history — revoked attestations are not deleted, only flagged. |
| **Expiration** | An optional `valid_to` timestamp after which an attestation is no longer considered valid. |
| **TTL (Time To Live)** | The Soroban ledger mechanism controlling how long persistent storage entries remain live before requiring an extension. See [docs/stellar-concepts.md](stellar-concepts.md). |
| **`require_auth`** | The Soroban SDK call that verifies the calling address has authorized the current invocation. |
| **Multi-Sig Attestation** | An attestation that requires signatures/approval from multiple issuers before becoming valid. |
| **Indexer** | The off-chain service (`indexer/`) that reads contract events and stores them in a queryable database. |
| **Soroban** | Stellar's smart contract platform, which TrustLink's contract is built on. |
| **WASM** | WebAssembly — the compiled binary format Soroban contracts run as. |

Translations of this glossary: [Español](i18n/es/glossary.md)
