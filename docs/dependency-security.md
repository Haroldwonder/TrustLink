# Dependency Security — TrustLink

## Overview

TrustLink is a Soroban smart contract compiled to WASM. Its dependency surface
is intentionally minimal: one direct production dependency (`soroban-sdk`) and
two dev-only dependencies (`soroban-sdk` testutils feature and `proptest`).
All transitive packages are pulled in by those two roots.

---

## Direct Dependencies

| Crate | Version (Cargo.toml) | Resolved version (Cargo.lock) | Usage |
|-------|----------------------|-------------------------------|-------|
| `soroban-sdk` | `21.0.0` | `21.7.7` | Production — Soroban contract SDK |
| `soroban-sdk` (testutils) | `21.0.0` | `21.7.7` | Dev only — test harness |
| `proptest` | `1.5.0` | `1.11.0` | Dev only — property-based testing |

> **Note:** Cargo.toml specifies `21.0.0` as a minimum compatible version.
> Cargo.lock resolves to `21.7.7`, which is the latest patch release in the
> `21.x` series and includes all upstream security and bug fixes.

---

## Transitive Dependency Inventory

All ~130 transitive packages are pulled in exclusively by `soroban-sdk` and
`proptest`. No additional direct dependencies exist in the production build.

<details>
<summary>Full transitive dependency list (click to expand)</summary>

| Crate | Version(s) |
|-------|-----------|
| addr2line | 0.25.1 |
| adler2 | 2.0.1 |
| android_system_properties | 0.1.5 |
| arbitrary | 1.3.2 |
| autocfg | 1.5.1 |
| backtrace | 0.3.76 |
| base16ct | 0.2.0 |
| base32 | 0.4.0 |
| base64 | 0.13.1, 0.22.1 |
| base64ct | 1.8.3 |
| bitflags | 2.11.1 |
| block-buffer | 0.10.4 |
| bs58 | 0.5.1 |
| bumpalo | 3.20.3 |
| bytes-lit | 0.0.5 |
| cc | 1.2.62 |
| cfg-if | 1.0.4 |
| chrono | 0.4.44 |
| const-oid | 0.9.6 |
| core-foundation-sys | 0.8.7 |
| cpufeatures | 0.2.17 |
| crate-git-revision | 0.0.6 |
| crypto-bigint | 0.5.5 |
| crypto-common | 0.1.6 |
| ctor | 0.2.9 |
| curve25519-dalek | 4.1.3 |
| curve25519-dalek-derive | 0.1.1 |
| darling | 0.20.11, 0.23.0 |
| darling_core | 0.20.11, 0.23.0 |
| darling_macro | 0.20.11, 0.23.0 |
| der | 0.7.10 |
| deranged | 0.5.8 |
| derive_arbitrary | 1.3.2 |
| digest | 0.10.7 |
| downcast-rs | 1.2.1 |
| dyn-clone | 1.0.20 |
| ecdsa | 0.16.9 |
| ed25519 | 2.2.3 |
| ed25519-dalek | 2.2.0 |
| either | 1.16.0 |
| elliptic-curve | 0.13.8 |
| equivalent | 1.0.2 |
| escape-bytes | 0.1.1 |
| ethnum | 1.5.3 |
| ff | 0.13.1 |
| fiat-crypto | 0.2.9 |
| find-msvc-tools | 0.1.9 |
| fnv | 1.0.7 |
| futures-core | 0.3.32 |
| futures-task | 0.3.32 |
| futures-util | 0.3.32 |
| generic-array | 0.14.9 |
| getrandom | 0.2.17, 0.3.4 |
| gimli | 0.32.3 |
| group | 0.13.0 |
| hashbrown | 0.12.3, 0.17.1 |
| hex | 0.4.3 |
| hex-literal | 0.4.1 |
| hmac | 0.12.1 |
| iana-time-zone | 0.1.65 |
| iana-time-zone-haiku | 0.1.2 |
| ident_case | 1.0.1 |
| indexmap | 1.9.3, 2.14.0 |
| indexmap-nostd | 0.4.0 |
| itertools | 0.11.0 |
| itoa | 1.0.18 |
| js-sys | 0.3.99 |
| k256 | 0.13.4 |
| keccak | 0.1.6 |
| libc | 0.2.186 |
| libm | 0.2.16 |
| log | 0.4.30 |
| memchr | 2.8.0 |
| miniz_oxide | 0.8.9 |
| num-bigint | 0.4.6 |
| num-conv | 0.2.2 |
| num-derive | 0.4.2 |
| num-integer | 0.1.46 |
| num-traits | 0.2.19 |
| object | 0.37.3 |
| once_cell | 1.21.4 |
| p256 | 0.13.2 |
| paste | 1.0.15 |
| pin-project-lite | 0.2.17 |
| pkcs8 | 0.10.2 |
| powerfmt | 0.2.0 |
| ppv-lite86 | 0.2.21 |
| prettyplease | 0.2.37 |
| primeorder | 0.13.6 |
| proc-macro2 | 1.0.106 |
| proptest | 1.11.0 |
| quote | 1.0.45 |
| r-efi | 5.3.0 |
| rand | 0.8.6, 0.9.4 |
| rand_chacha | 0.3.1, 0.9.0 |
| rand_core | 0.6.4, 0.9.5 |
| rand_xorshift | 0.4.0 |
| ref-cast | 1.0.25 |
| ref-cast-impl | 1.0.25 |
| regex-syntax | 0.8.10 |
| rfc6979 | 0.4.0 |
| rustc-demangle | 0.1.27 |
| rustc_version | 0.4.1 |
| rustversion | 1.0.22 |
| schemars | 0.9.0, 1.2.1 |
| sec1 | 0.7.3 |
| semver | 1.0.28 |
| serde | 1.0.228 |
| serde_core | 1.0.228 |
| serde_derive | 1.0.228 |
| serde_json | 1.0.150 |
| serde_with | 3.20.0 |
| serde_with_macros | 3.20.0 |
| sha2 | 0.10.9 |
| sha3 | 0.10.9 |
| shlex | 1.3.0 |
| signature | 2.2.0 |
| slab | 0.4.12 |
| smallvec | 1.15.1 |
| soroban-builtin-sdk-macros | 21.2.1 |
| soroban-env-common | 21.2.1 |
| soroban-env-guest | 21.2.1 |
| soroban-env-host | 21.2.1 |
| soroban-env-macros | 21.2.1 |
| soroban-ledger-snapshot | 21.7.7 |
| soroban-sdk | 21.7.7 |
| soroban-sdk-macros | 21.7.7 |
| soroban-spec | 21.7.7 |
| soroban-spec-rust | 21.7.7 |
| soroban-wasmi | 0.31.1-soroban.20.0.1 |
| spin | 0.9.8 |
| spki | 0.7.3 |
| static_assertions | 1.1.0 |
| stellar-strkey | 0.0.8 |
| stellar-xdr | 21.2.0 |
| strsim | 0.11.1 |
| subtle | 2.6.1 |
| syn | 2.0.117 |
| thiserror | 1.0.69 |
| thiserror-impl | 1.0.69 |
| time | 0.3.47 |
| time-core | 0.1.8 |
| time-macros | 0.2.27 |
| tinyvec | 1.11.0 |
| tinyvec_macros | 0.1.1 |
| typenum | 1.20.0 |
| unarray | 0.1.4 |
| unicode-ident | 1.0.24 |
| version_check | 0.9.5 |
| wasi | 0.11.1+wasi-snapshot-preview1 |
| wasip2 | 1.0.3+wasi-0.2.9 |
| wasm-bindgen | 0.2.122 |
| wasm-bindgen-macro | 0.2.122 |
| wasm-bindgen-macro-support | 0.2.122 |
| wasm-bindgen-shared | 0.2.122 |
| wasmi_arena | 0.4.1 |
| wasmi_core | 0.13.0 |
| wasmparser | 0.116.1 |
| wasmparser-nostd | 0.100.2 |
| windows-core | 0.62.2 |
| windows-implement | 0.60.2 |
| windows-interface | 0.59.3 |
| windows-link | 0.2.1 |
| windows-result | 0.4.1 |
| windows-strings | 0.5.1 |
| wit-bindgen | 0.57.1 |
| zerocopy | 0.8.48 |
| zerocopy-derive | 0.8.48 |
| zeroize | 1.8.2 |
| zmij | 1.0.21 |

</details>

---

## Known CVEs and Mitigations

As of the last audit run, **no known CVEs affect TrustLink's dependency tree**.

`cargo audit` is run in CI on every push and pull request (see [CI Audit Process](#ci-audit-process) below). The CI job is configured with `--deny warnings`, which causes the build to fail if any advisory — including informational ones — is reported against the resolved dependency set.

The `Cargo.audit` file at the repository root is the authoritative record of any
accepted advisories. It is currently empty, meaning no vulnerabilities have been
accepted or suppressed.

---

## CI Audit Process

Security scanning is automated via `cargo-audit` in the CI pipeline.

### How it runs

The `audit` job in `.github/workflows/ci.yml` runs on every push and pull
request:

```yaml
jobs:
  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: rustup show
      - run: cargo audit --deny warnings
```

The `--deny warnings` flag treats every advisory — including low-severity and
informational notices — as a build failure. A PR cannot be merged if any
unacknowledged advisory exists in the dependency tree.

### Running locally

Install `cargo-audit` and run it from the repository root:

```bash
cargo install cargo-audit --locked
cargo audit
```

To also check against the accepted-advisory list in `Cargo.audit`:

```bash
cargo audit --config Cargo.audit
```

### Accepting an advisory

If a reported advisory does not affect TrustLink's usage pattern, it can be
accepted by adding an entry to `Cargo.audit`:

```toml
[[advisories]]
id = "RUSTSEC-YYYY-NNNNN"
reason = "Vulnerability does not affect our usage — <brief explanation>"
date = "YYYY-MM-DD"
reviewer = "github-username"
```

Every accepted advisory **must** include a written justification and a reviewer.
Entries are reviewed as part of the security audit process before mainnet
deployment.

---

## Audit Configuration File

**Location:** [`Cargo.audit`](../Cargo.audit) (repository root)

**Format:** TOML, parsed by `cargo-audit`.

**Current state:** No accepted advisories. The file contains only the format
documentation and an example entry.

There is no `deny.toml` (cargo-deny) configuration in this repository. If
license compliance or dependency graph policy enforcement is needed in the
future, `cargo-deny` can be added alongside `cargo-audit`.

---

## Dependency Security Posture

| Property | Status |
|----------|--------|
| Direct production dependencies | 1 (`soroban-sdk`) |
| Dev-only dependencies | 2 (`soroban-sdk` testutils, `proptest`) |
| Known CVEs in dependency tree | None |
| Accepted/suppressed advisories | None |
| Automated scanning | ✅ `cargo audit --deny warnings` on every PR |
| Cargo.lock committed | ✅ Reproducible builds enforced |
| Rust toolchain pinned | ✅ `rust-toolchain.toml` (stable channel) |
| `no_std` contract binary | ✅ WASM target excludes std library |

### Why the dependency surface is small

TrustLink targets `wasm32-unknown-unknown` with `#![no_std]`. The Soroban
runtime provides all host functions (storage, crypto, ledger time). This means:

- No networking, filesystem, or OS dependencies in the production binary.
- No TLS, HTTP, or database crates.
- The transitive dependency tree is entirely determined by `soroban-sdk`, which
  is maintained by the Stellar Development Foundation and audited as part of the
  Soroban platform.

The `proptest` dependency and all `soroban-sdk` testutils packages are
**dev-only** — they are compiled into the test binary but are never included in
the deployed WASM artifact.
