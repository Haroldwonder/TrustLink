# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 (2026-08-14)


### Features

* [#760](https://github.com/Trustlink-1/TrustLink/issues/760) add TemplatePanel for issuer attestation template management ([fb66692](https://github.com/Trustlink-1/TrustLink/commit/fb66692dde74f754f82b2271535aa43f7bc4c8e9))
* [#763](https://github.com/Trustlink-1/TrustLink/issues/763) real-time toast notifications for attestation events ([1a52f23](https://github.com/Trustlink-1/TrustLink/commit/1a52f23aea38ddbc658e6f5964fd67d722c08ec1))
* [#764](https://github.com/Trustlink-1/TrustLink/issues/764) add attestation history timeline view to UserPanel ([ad3c905](https://github.com/Trustlink-1/TrustLink/commit/ad3c905969d448f64b4bd090a45daf7bca7c2b62))
* [#765](https://github.com/Trustlink-1/TrustLink/issues/765) add CSV export for issuer attestation lists ([cccfd7c](https://github.com/Trustlink-1/TrustLink/commit/cccfd7c4b51a039e1c2930244942eb95d4382049))
* **#766:** add search and filter controls to attestation lists ([f2a4db3](https://github.com/Trustlink-1/TrustLink/commit/f2a4db3052c5befd10a543a415481192d902e024))
* **#767:** add QR code generation for sharing attestation IDs ([e6ac3b3](https://github.com/Trustlink-1/TrustLink/commit/e6ac3b356bf9ac233f585e8dd743b942fba5adf0))
* **#768:** add wallet-connection error states for unsupported networks ([2cdbdf8](https://github.com/Trustlink-1/TrustLink/commit/2cdbdf8405558d07d9fb3c425a201180f1db38ff))
* **#769:** add RateLimitPanel showing issuer rate-limit usage ([0414913](https://github.com/Trustlink-1/TrustLink/commit/0414913245d8fee2ab3b75b052b765a2653a848c))
* **#798:** add ADR-010 for issuer-defined custom validation hooks ([8bf2bf2](https://github.com/Trustlink-1/TrustLink/commit/8bf2bf2066b55452423504762d4f55c59fcf03dc))
* **#799:** add k6 load test for indexer GraphQL API ([b68a099](https://github.com/Trustlink-1/TrustLink/commit/b68a099a78bc6c51379e6d554a0f6de266bf72df))
* **#806:** add multi-architecture Docker builds for indexer (amd64 + arm64) ([c4fa150](https://github.com/Trustlink-1/TrustLink/commit/c4fa1508420350f7b3b4c7ba96e1c6758a779417))
* **#807:** add infrastructure cost monitoring and budget alerts (AWS Budgets + SNS) ([8c66e95](https://github.com/Trustlink-1/TrustLink/commit/8c66e95992af72a90b68fd7b99df14d561eb4559))
* **#808:** add post-release SDK and indexer compatibility smoke test ([94f6656](https://github.com/Trustlink-1/TrustLink/commit/94f6656d29d898ca05980cf0fb4ac13df1919753))
* **#814:** add supply-chain provenance verification example with multi-issuer attestations ([95445e8](https://github.com/Trustlink-1/TrustLink/commit/95445e8429ae6cad9dfbddc176bedade18bfecd3))
* **#814:** Supply-chain provenance verification with multi-issuer attestations ([#848](https://github.com/Trustlink-1/TrustLink/issues/848)) ([1eabdc7](https://github.com/Trustlink-1/TrustLink/commit/1eabdc74fd0a647eea6c516c8516b666d83bcef6))
* **#927:** add cargo check --workspace as required CI gate ([4e9c6d9](https://github.com/Trustlink-1/TrustLink/commit/4e9c6d965ab744ccf8f6ba6eef6f531b80ac29cf)), closes [#927](https://github.com/Trustlink-1/TrustLink/issues/927)
* **798:** add custom claim type constraint validation hooks ([c1a2c6b](https://github.com/Trustlink-1/TrustLink/commit/c1a2c6bee15c80711d810e6b1479aa655d796d1b)), closes [#798](https://github.com/Trustlink-1/TrustLink/issues/798)
* add carbon-credit verifier-of-verifiers example ([#1018](https://github.com/Trustlink-1/TrustLink/issues/1018)) ([156cfe6](https://github.com/Trustlink-1/TrustLink/commit/156cfe6f2504579709444d7da2cd2a78cb9c0090))
* add claim type existence check with require_registered_claim_type config ([408fd28](https://github.com/Trustlink-1/TrustLink/commit/408fd2861f475667c3d9a7266a239907a2c1f9fb))
* add clippy CI, coverage threshold, multisig CLI commands, and import command ([eb4a013](https://github.com/Trustlink-1/TrustLink/commit/eb4a0130df6b9a2a83696ad2a13f1461f245eea7)), closes [#374](https://github.com/Trustlink-1/TrustLink/issues/374) [#375](https://github.com/Trustlink-1/TrustLink/issues/375) [#566](https://github.com/Trustlink-1/TrustLink/issues/566) [#567](https://github.com/Trustlink-1/TrustLink/issues/567)
* add CODEOWNERS, ValidAttestations index, and batch benchmarks ([b177bc1](https://github.com/Trustlink-1/TrustLink/commit/b177bc15c18a1efe7978e7a65ff2698a404ba80d)), closes [#593](https://github.com/Trustlink-1/TrustLink/issues/593) [#594](https://github.com/Trustlink-1/TrustLink/issues/594) [#596](https://github.com/Trustlink-1/TrustLink/issues/596)
* add custom claim type constraint validation hooks (closes [#798](https://github.com/Trustlink-1/TrustLink/issues/798), PR [#855](https://github.com/Trustlink-1/TrustLink/issues/855)) ([89b2281](https://github.com/Trustlink-1/TrustLink/commit/89b2281eaa560b23244bfbfe3536073e8caf2c07))
* add DID-style guardian social-recovery example ([#1019](https://github.com/Trustlink-1/TrustLink/issues/1019)) ([452e6a6](https://github.com/Trustlink-1/TrustLink/commit/452e6a6f73de744e853aa2978793f0ffcb4abef4))
* add event ticketing example ([8701199](https://github.com/Trustlink-1/TrustLink/commit/8701199570cac2840567caad5616c45bfbc858c0))
* add freelance reputation example, subject data export, audit trail CLI, and sanctions screening docs ([76b816f](https://github.com/Trustlink-1/TrustLink/commit/76b816fc9a98be604ec5c58c9357537936092df8))
* add freelance reputation example, subject data export, audit trail, screening ([#854](https://github.com/Trustlink-1/TrustLink/issues/854)) ([dbf57fa](https://github.com/Trustlink-1/TrustLink/commit/dbf57fa20ddfd72128e190e483334bf05c91a050))
* add get_bridge_list() paginated query ([d51a4e6](https://github.com/Trustlink-1/TrustLink/commit/d51a4e60ed356aa4acb34a643bf7129770d98ae8))
* add get_expiring_attestations for subjects and issuers ([#604](https://github.com/Trustlink-1/TrustLink/issues/604)) ([b34bb39](https://github.com/Trustlink-1/TrustLink/commit/b34bb39b3c04b106724b469428e7b653e65d30cd))
* add get_issuer_list() paginated query ([fa9d85b](https://github.com/Trustlink-1/TrustLink/commit/fa9d85b1d913c23e445fceeefe2540c08e9eae08))
* add global stats integration test and fix pre-existing compilation errors ([4e768de](https://github.com/Trustlink-1/TrustLink/commit/4e768de38fdfefb5b3701e727357acdc605e837b))
* add insurance policy underwriting example with KYC and AML verification ([d788ca8](https://github.com/Trustlink-1/TrustLink/commit/d788ca8d62bedb186c09bfbc55b6e70eb7c06202))
* add missing error variants ([#919](https://github.com/Trustlink-1/TrustLink/issues/919)) ([c22ddd9](https://github.com/Trustlink-1/TrustLink/commit/c22ddd9717d2e195facd2ffa5bd2a2c114dce45b))
* add NFT marketplace example gated by KYC and jurisdiction ([9be831a](https://github.com/Trustlink-1/TrustLink/commit/9be831a3bb71a8a4bcbcca621a754bf3a4116022)), closes [#1024](https://github.com/Trustlink-1/TrustLink/issues/1024)
* add optional per-subject attestation limit to prevent unbounded growth ([f5b662e](https://github.com/Trustlink-1/TrustLink/commit/f5b662e6fd0a9b12c290cbcab4ebed4b681ad357))
* Add Python bindings with get_audit_log() and GraphQL pagination ([c5d0102](https://github.com/Trustlink-1/TrustLink/commit/c5d0102d4aa1ad7e4f9175f23f5aac6e4cbd3c5e))
* add quickstart, real-estate and healthcare examples, WCAG audit ([#860](https://github.com/Trustlink-1/TrustLink/issues/860)) ([6990002](https://github.com/Trustlink-1/TrustLink/commit/69900029e5932cb9bada8590c010c956ad6dca2e))
* add quickstart, real-estate and healthcare examples, WCAG audit fixes ([#809](https://github.com/Trustlink-1/TrustLink/issues/809) [#815](https://github.com/Trustlink-1/TrustLink/issues/815) [#816](https://github.com/Trustlink-1/TrustLink/issues/816) [#825](https://github.com/Trustlink-1/TrustLink/issues/825)) ([0d86a69](https://github.com/Trustlink-1/TrustLink/commit/0d86a69569a6c62fe17f5dd4139361cff271b209))
* add SDK methods for issues [#744](https://github.com/Trustlink-1/TrustLink/issues/744) [#745](https://github.com/Trustlink-1/TrustLink/issues/745) [#746](https://github.com/Trustlink-1/TrustLink/issues/746) [#747](https://github.com/Trustlink-1/TrustLink/issues/747) ([b8d6736](https://github.com/Trustlink-1/TrustLink/commit/b8d673603ecdca1b67416a7151f8187576333c49))
* add security hardening and indexer reliability improvements ([0e10d64](https://github.com/Trustlink-1/TrustLink/commit/0e10d641df6e7e9b74f7bd7c156b6e1642107fce))
* add Template, Delegation, WhitelistEntry, CouncilAction to indexer ([#770](https://github.com/Trustlink-1/TrustLink/issues/770) [#771](https://github.com/Trustlink-1/TrustLink/issues/771) [#772](https://github.com/Trustlink-1/TrustLink/issues/772) [#773](https://github.com/Trustlink-1/TrustLink/issues/773)) ([9a9a7c1](https://github.com/Trustlink-1/TrustLink/commit/9a9a7c12cf35ba677bf669d06e94211321dbe8ea))
* add trade finance example ([b431b41](https://github.com/Trustlink-1/TrustLink/commit/b431b412e95cfb49d29bed9e935d28599b7d8ed0))
* add TypeScript SDK with range queries, council, and limits methods ([1c347d2](https://github.com/Trustlink-1/TrustLink/commit/1c347d27eda2c3d1d440f11584539dda0ec80d56))
* add underflow-safe counter tracking for issuers, attestations, revocations ([81bdc4e](https://github.com/Trustlink-1/TrustLink/commit/81bdc4ee5520f0ec880ebcdc60544b3cfe9ffb88))
* add version compatibility guard for state-changing entry points ([7176ede](https://github.com/Trustlink-1/TrustLink/commit/7176ede5cc5063f6c300e6aec5a1e26b6624828b)), closes [#952](https://github.com/Trustlink-1/TrustLink/issues/952)
* add wallet disconnect button ([3b4efe8](https://github.com/Trustlink-1/TrustLink/commit/3b4efe8742af8b10d5bf4f35ed9e1f96e194b842))
* add WASM binary hash as first-class release artifact ([9921b39](https://github.com/Trustlink-1/TrustLink/commit/9921b394bba5bfcb34a612ec93e66bb8a0890885))
* add whitelist queries, delegation list, template list, and Python valid-claims methods ([28146ff](https://github.com/Trustlink-1/TrustLink/commit/28146ff4dddea8daada3efcfac33d86a91017b52))
* add whitelist queries, delegation list, template list, and Python valid-claims methods (PR [#850](https://github.com/Trustlink-1/TrustLink/issues/850)) ([2885e41](https://github.com/Trustlink-1/TrustLink/commit/2885e4192b5aeef8b7aca826e945bdfa2a9c6cbe))
* **attestation:** implement transfer_attestation for compromised issuer recovery ([812964c](https://github.com/Trustlink-1/TrustLink/commit/812964cee0592960643e8d48c79496b4fbaabfea))
* **bindings,indexer:** add input validation and monitoring improvements ([0f2da49](https://github.com/Trustlink-1/TrustLink/commit/0f2da49250e67fd606f98c0fb2f85cdb50b013c5))
* **bindings:** add Rust RPC client crate with core read queries ([9005faf](https://github.com/Trustlink-1/TrustLink/commit/9005faf0bac4ec05a60414da259e7b7f33f13bdf))
* **build:** add check-wasm-size target and changelog-preview command ([9e4a478](https://github.com/Trustlink-1/TrustLink/commit/9e4a47800cbe771b644eae507145969470762302))
* council timelock, attestation dispute, reputation decay, amendment history ([#790](https://github.com/Trustlink-1/TrustLink/issues/790)-[#793](https://github.com/Trustlink-1/TrustLink/issues/793)) ([1a27697](https://github.com/Trustlink-1/TrustLink/commit/1a276975c7f5f600d4ae394c3b0a4031d11a4136))
* **errors:** single source of truth for contract error codes ([#964](https://github.com/Trustlink-1/TrustLink/issues/964)) ([a7c33f6](https://github.com/Trustlink-1/TrustLink/commit/a7c33f64388a50c5aa2d5e45dbc1055e49f0d715))
* **examples:** add IssuerTier-gated lending-pool reference ([b042191](https://github.com/Trustlink-1/TrustLink/commit/b0421919e7cf716387db793d8bf957d96802f276)), closes [#1022](https://github.com/Trustlink-1/TrustLink/issues/1022)
* **examples:** add multi-tenant SaaS seat-license verification ([07ead56](https://github.com/Trustlink-1/TrustLink/commit/07ead56aaa5496dad65aee72b28b52e5e9ca67d6))
* expose get_pending_admin_transfer() as read-only query ([9018d0b](https://github.com/Trustlink-1/TrustLink/commit/9018d0b44bf8d00129c7381c736fd780af248d78))
* expose list_endorsements_by_endorser in public contract interface (issue [#939](https://github.com/Trustlink-1/TrustLink/issues/939)) ([a4e58eb](https://github.com/Trustlink-1/TrustLink/commit/a4e58eb5968cead3bb6e04aa858c90a676cd0b58))
* **governance:** add proposal deadline enforcement to vote function ([14a7435](https://github.com/Trustlink-1/TrustLink/commit/14a74351c172ed3f33a8fd8b963510e2cc8fff6e))
* implement Add tests for admin council operations ([e94e739](https://github.com/Trustlink-1/TrustLink/commit/e94e739fd5b42691b6d8ca879b02bd91e70de9a3))
* implement Add tests for claim type registry pagination ([4259fb7](https://github.com/Trustlink-1/TrustLink/commit/4259fb7cf3d0181dd2e0ca6faef49a53fabb02ff))
* implement atomic attestation bundling with shared bundle IDs ([268925a](https://github.com/Trustlink-1/TrustLink/commit/268925addcd08d3410a117105596bbe31afc099a))
* implement attestation templates (create, instantiate, list, get) ([dccfd75](https://github.com/Trustlink-1/TrustLink/commit/dccfd75d5cd8b45b8a470277ca7ca11cb603abc3))
* implement attestation valid_from lifecycle with Pending status ([7616c0c](https://github.com/Trustlink-1/TrustLink/commit/7616c0c213257f3572e932cdf9b77b4acf8a844b))
* implement issues [#605](https://github.com/Trustlink-1/TrustLink/issues/605), [#606](https://github.com/Trustlink-1/TrustLink/issues/606), [#607](https://github.com/Trustlink-1/TrustLink/issues/607), [#608](https://github.com/Trustlink-1/TrustLink/issues/608) ([9662b9e](https://github.com/Trustlink-1/TrustLink/commit/9662b9e8e6dd4728939eb76d1075694e1c1c12d6))
* implement issues [#609](https://github.com/Trustlink-1/TrustLink/issues/609) [#610](https://github.com/Trustlink-1/TrustLink/issues/610) [#611](https://github.com/Trustlink-1/TrustLink/issues/611) [#612](https://github.com/Trustlink-1/TrustLink/issues/612) ([9c458dc](https://github.com/Trustlink-1/TrustLink/commit/9c458dc20f587a03c86463dff93318a810de024d))
* implement mainnet-checklist.md: add post-deployment verification steps ([9427268](https://github.com/Trustlink-1/TrustLink/commit/94272689cb30286bf0456a01650fe9d435061eb6))
* implement unified event-topic taxonomy and SDK filtering ([2864d64](https://github.com/Trustlink-1/TrustLink/commit/2864d64fd329f0434c2b0e277d113909c43c382a))
* **indexer:** add API key auth, depth/complexity limits, pino logging, OTel tracing ([7b42548](https://github.com/Trustlink-1/TrustLink/commit/7b425489ce6ab1d7a5624d31ad794575512b1226))
* **indexer:** add attestation request persistence and GraphQL query support ([66a811f](https://github.com/Trustlink-1/TrustLink/commit/66a811f0d77b7b4de6e6d554bec21640c6ca5e8c)), closes [#545](https://github.com/Trustlink-1/TrustLink/issues/545)
* **indexer:** add indexer-dev, indexer-build, indexer-logs Makefile targets ([d949067](https://github.com/Trustlink-1/TrustLink/commit/d949067091edf3f9a4b662bdf2797bbebeb0035d)), closes [#576](https://github.com/Trustlink-1/TrustLink/issues/576)
* **indexer:** add issuer management, health checks, and reindex capabilities ([68fe00b](https://github.com/Trustlink-1/TrustLink/commit/68fe00b9ca6a31f7f4ad7d7bb78ad72f9ba2cc2f))
* **indexer:** add multi-sig proposal persistence and GraphQL query support ([3c3a1c8](https://github.com/Trustlink-1/TrustLink/commit/3c3a1c8a05c05013b34c826b839c66b4629f8e2d))
* **indexer:** add Template, Delegation, WhitelistEntry, CouncilAction types and GraphQL queries (closes [#770](https://github.com/Trustlink-1/TrustLink/issues/770), [#771](https://github.com/Trustlink-1/TrustLink/issues/771), [#772](https://github.com/Trustlink-1/TrustLink/issues/772), [#773](https://github.com/Trustlink-1/TrustLink/issues/773), PR [#837](https://github.com/Trustlink-1/TrustLink/issues/837)) ([a38ebb9](https://github.com/Trustlink-1/TrustLink/commit/a38ebb9a81f27602aca504267d84f5ac58292d81))
* **indexer:** audit log, issuer rate limit, revocation reason, Redis cache ([#774](https://github.com/Trustlink-1/TrustLink/issues/774)-[#777](https://github.com/Trustlink-1/TrustLink/issues/777)) ([5168312](https://github.com/Trustlink-1/TrustLink/commit/5168312a23762243210d53ebb3502c1bead3cb7a))
* **indexer:** bulk export, log sampling, route fix, and cross-SDK conformance ([10b0a5e](https://github.com/Trustlink-1/TrustLink/commit/10b0a5e7cbe380bcf0852153e3fb8e8400f30820))
* **indexer:** durable webhook failure handling and recovery ([d7f5308](https://github.com/Trustlink-1/TrustLink/commit/d7f53081b1fc808d38641062953b05713e217656)), closes [#545](https://github.com/Trustlink-1/TrustLink/issues/545)
* **indexer:** replace in-memory PubSub with Redis-backed implementation for horizontal scaling ([ac65608](https://github.com/Trustlink-1/TrustLink/commit/ac656081aa6f7fd73a5553cb5ccea22c02ff6bb2))
* **indexer:** support multiple contract IDs with per-contract data partitioning ([fb9d7b6](https://github.com/Trustlink-1/TrustLink/commit/fb9d7b6054b52cf9c190bf15da8e24139f3af5bb))
* **issue-796:** Add batch query for has_valid_claim across multiple subjects ([a70e9fe](https://github.com/Trustlink-1/TrustLink/commit/a70e9fe0daed6342ccb948e4b857d0aa224fd636))
* issues 756-757-758-759 — Python template/delegation bindings, client unification, CouncilPanel ([d24c09b](https://github.com/Trustlink-1/TrustLink/commit/d24c09bac5660be2a513ce87c13f3b08020f8226))
* make React dApp deployable on Vercel ([1227a10](https://github.com/Trustlink-1/TrustLink/commit/1227a108cc95416532691120c757190d65c772c9))
* **makefile:** add snapshot-update target and snapshot testing docs ([976270c](https://github.com/Trustlink-1/TrustLink/commit/976270ca4dbd6527d25b48c3b180672b25119407))
* **monitoring:** define SLOs, synthetic uptime checks, issuer dashboard, and revocation spike alert ([abac26d](https://github.com/Trustlink-1/TrustLink/commit/abac26d62b92589abe2156827c5898e57e53d548)), closes [#821](https://github.com/Trustlink-1/TrustLink/issues/821) [#822](https://github.com/Trustlink-1/TrustLink/issues/822) [#823](https://github.com/Trustlink-1/TrustLink/issues/823) [#824](https://github.com/Trustlink-1/TrustLink/issues/824)
* **monitoring:** define SLOs, synthetic uptime checks, issuer dashboard, and revocation spike alert ([#835](https://github.com/Trustlink-1/TrustLink/issues/835)) ([d0b3ba4](https://github.com/Trustlink-1/TrustLink/commit/d0b3ba49d962c40f696ce9fc157157e79cc54acf))
* multisig configurable TTL, cancel proposal, and list open proposals ([3b13f6d](https://github.com/Trustlink-1/TrustLink/commit/3b13f6ded8f7b1596f126da1874d8b79b07c2798))
* **python-sdk:** add has_all_claims and has_any_claim with validation and tests ([b955812](https://github.com/Trustlink-1/TrustLink/commit/b95581231ba3b4d3579bdf65715d06a3bdef19f5)), closes [#545](https://github.com/Trustlink-1/TrustLink/issues/545)
* **python:** add AsyncTrustLinkClient for asyncio support ([8b21631](https://github.com/Trustlink-1/TrustLink/commit/8b2163152dd46b69d4c608ad75a13526017ea920)), closes [#540](https://github.com/Trustlink-1/TrustLink/issues/540)
* **python:** add config/metadata, async write methods, multisig read, and whitelist support ([4ad94da](https://github.com/Trustlink-1/TrustLink/commit/4ad94daee1cbe3513833cbf02f96775ceae9111e))
* **python:** add config/metadata, async write methods, multisig read, and whitelist support (PR [#851](https://github.com/Trustlink-1/TrustLink/issues/851)) ([6b13a3c](https://github.com/Trustlink-1/TrustLink/commit/6b13a3cdb96df5d08308efe2093af930f997af22))
* **python:** prepare trustlink-sdk for PyPI distribution ([7daa0fa](https://github.com/Trustlink-1/TrustLink/commit/7daa0fae2635bbed8fe8359cdfd83fded1386eb6))
* React app — CSV export, attestation timeline, toast notifications, and template management (closes [#760](https://github.com/Trustlink-1/TrustLink/issues/760), [#763](https://github.com/Trustlink-1/TrustLink/issues/763), [#764](https://github.com/Trustlink-1/TrustLink/issues/764), [#765](https://github.com/Trustlink-1/TrustLink/issues/765), PR [#836](https://github.com/Trustlink-1/TrustLink/issues/836)) ([36e1ea0](https://github.com/Trustlink-1/TrustLink/commit/36e1ea0ba5dca51c16f785e874fda145a8e9df39))
* React app UX improvements — rate limits, network check, QR codes, attestation filters (closes [#766](https://github.com/Trustlink-1/TrustLink/issues/766), [#767](https://github.com/Trustlink-1/TrustLink/issues/767), [#768](https://github.com/Trustlink-1/TrustLink/issues/768), [#769](https://github.com/Trustlink-1/TrustLink/issues/769), PR [#838](https://github.com/Trustlink-1/TrustLink/issues/838)) ([b272512](https://github.com/Trustlink-1/TrustLink/commit/b2725122183518512b95c2284e34eeb8eb75cf82))
* **react-app:** add DelegationPanel for sub-issuer delegation management ([#762](https://github.com/Trustlink-1/TrustLink/issues/762)) ([3a6c916](https://github.com/Trustlink-1/TrustLink/commit/3a6c91696bb27570717de35fdaf84dc839c8fecd))
* **react-app:** add DelegationPanel for sub-issuer delegation management (closes [#762](https://github.com/Trustlink-1/TrustLink/issues/762), PR [#833](https://github.com/Trustlink-1/TrustLink/issues/833)) ([97f204a](https://github.com/Trustlink-1/TrustLink/commit/97f204aff70e4dace5411050c662710553060af5))
* **react-app:** add expiring attestations section with renewal to IssuerDashboard ([043eda5](https://github.com/Trustlink-1/TrustLink/commit/043eda5e221c581941193efb8cfc5dc244fa79af)), closes [#562](https://github.com/Trustlink-1/TrustLink/issues/562)
* **react-app:** add useGlobalStats hook and refactor AdminPanel ([dc4ba44](https://github.com/Trustlink-1/TrustLink/commit/dc4ba44bfdee263cf5ad9651705f05d509cbf021)), closes [#539](https://github.com/Trustlink-1/TrustLink/issues/539)
* **react-app:** add WhitelistPanel for issuer whitelist management ([#761](https://github.com/Trustlink-1/TrustLink/issues/761)) ([105c454](https://github.com/Trustlink-1/TrustLink/commit/105c4545dc489da90ae8d0ff0c1270f75da7ae4b))
* **react-app:** add WhitelistPanel for issuer whitelist management (closes [#761](https://github.com/Trustlink-1/TrustLink/issues/761), PR [#834](https://github.com/Trustlink-1/TrustLink/issues/834)) ([9c3871f](https://github.com/Trustlink-1/TrustLink/commit/9c3871f7c8fd86b0c1b18e2746ff9673ed179533))
* **react:** implement and export useGlobalStats hook ([#962](https://github.com/Trustlink-1/TrustLink/issues/962)) ([7ebfad5](https://github.com/Trustlink-1/TrustLink/commit/7ebfad5437f6674f4ef6ef94a1ee41ed4656db0d))
* resolve issues [#506](https://github.com/Trustlink-1/TrustLink/issues/506) [#507](https://github.com/Trustlink-1/TrustLink/issues/507) [#508](https://github.com/Trustlink-1/TrustLink/issues/508) [#509](https://github.com/Trustlink-1/TrustLink/issues/509) ([c69deac](https://github.com/Trustlink-1/TrustLink/commit/c69deacaae1415000ec890ff4dc35e0575d796aa))
* resolve issues [#526](https://github.com/Trustlink-1/TrustLink/issues/526), [#527](https://github.com/Trustlink-1/TrustLink/issues/527), [#528](https://github.com/Trustlink-1/TrustLink/issues/528), [#529](https://github.com/Trustlink-1/TrustLink/issues/529) ([b7f2319](https://github.com/Trustlink-1/TrustLink/commit/b7f23198b9ca9bf7e81f46531b55e157ab475a7a))
* resolve issues [#530](https://github.com/Trustlink-1/TrustLink/issues/530), [#531](https://github.com/Trustlink-1/TrustLink/issues/531), [#532](https://github.com/Trustlink-1/TrustLink/issues/532) — templates, tier claims, analytics ([1efb88b](https://github.com/Trustlink-1/TrustLink/commit/1efb88b7ad71669a7e8816ecde249ffe1c4b1a87))
* SDK methods for getMultisigTtl, getRateLimitForClaimType, getRegisteredClaimType, getRequest (closes [#744](https://github.com/Trustlink-1/TrustLink/issues/744), [#745](https://github.com/Trustlink-1/TrustLink/issues/745), [#746](https://github.com/Trustlink-1/TrustLink/issues/746), [#747](https://github.com/Trustlink-1/TrustLink/issues/747), PR [#839](https://github.com/Trustlink-1/TrustLink/issues/839)) ([e8decbc](https://github.com/Trustlink-1/TrustLink/commit/e8decbcf61af3b76a90157dc931d00d19a6bde92))
* **sdk/react:** add useIssuerStats hook and refactor IssuerDashboard ([502260b](https://github.com/Trustlink-1/TrustLink/commit/502260ba5db0bdf7361a472d57b86aec71568421)), closes [#538](https://github.com/Trustlink-1/TrustLink/issues/538)
* **sdk:** add get_delegation() read function to TypeScript SDK ([5d83399](https://github.com/Trustlink-1/TrustLink/commit/5d83399382094b86d8b040edcc6384359f0dc171))
* **sdk:** add React hooks package ([48eb64d](https://github.com/Trustlink-1/TrustLink/commit/48eb64da68dcabed2fc536e6322aeeffd72be49c)), closes [#350](https://github.com/Trustlink-1/TrustLink/issues/350)
* **sdk:** add ResilienceConfig, provenance, iterateSubjectAttestations docs, and TypeDoc generation ([6d3e152](https://github.com/Trustlink-1/TrustLink/commit/6d3e1526299e6d9aba40b192cb2bc73e83b39a93))
* **sdk:** add typed error classes to TypeScript client ([22d9442](https://github.com/Trustlink-1/TrustLink/commit/22d94423ad45ee79ee3eb580fd036ec2894623b2)), closes [#347](https://github.com/Trustlink-1/TrustLink/issues/347)
* **sdk:** add W3C Verifiable Credential export for attestations ([7a066ef](https://github.com/Trustlink-1/TrustLink/commit/7a066ef90b6a9823087966625c08f6bee626b751))
* **sdk:** sync TypeScript types with Rust contract types ([1b5fbe4](https://github.com/Trustlink-1/TrustLink/commit/1b5fbe4ff15cd054fe8e3ff11c288dbd94fe0949))
* **security:** add cargo-deny integration and dependency security policy ([8262d07](https://github.com/Trustlink-1/TrustLink/commit/8262d07c81d0c6ca8f36037c40cc3f8c89d6b59a))
* **storage,admin:** make ChunkedIndex chunk size admin-configurable ([1467efe](https://github.com/Trustlink-1/TrustLink/commit/1467efec589d3e235c13ceda4a5d6708b9fb0f01)), closes [#956](https://github.com/Trustlink-1/TrustLink/issues/956)
* validate jurisdiction field against ISO 3166-1 alpha-2 codes ([a373287](https://github.com/Trustlink-1/TrustLink/commit/a373287c28c577f575bfc625aaf420693b8be0bd))
* xBull wallet support, i18n, bulk-create, and template CLI commands (closes [#826](https://github.com/Trustlink-1/TrustLink/issues/826), [#827](https://github.com/Trustlink-1/TrustLink/issues/827), [#828](https://github.com/Trustlink-1/TrustLink/issues/828), [#829](https://github.com/Trustlink-1/TrustLink/issues/829)) ([30c15b6](https://github.com/Trustlink-1/TrustLink/commit/30c15b6a997d5c70ec80c96e8363a4991265ea58))


### Bug Fixes

* **#533:** renew_attestation records new expiration in audit log details ([a749f51](https://github.com/Trustlink-1/TrustLink/commit/a749f515cd5086d1c50f1f3b400a17035b56b746))
* **#558, #559:** add dark mode toggle and getAttestationsByTag pagination ([8c033b2](https://github.com/Trustlink-1/TrustLink/commit/8c033b25d5c6d84678990d450e5186beb5286ab1)), closes [#558](https://github.com/Trustlink-1/TrustLink/issues/558)
* **#928:** fix duplicate Prometheus metric names and add registration test ([3b77350](https://github.com/Trustlink-1/TrustLink/commit/3b77350b78679791ff184f867de6c143642f20ef)), closes [#928](https://github.com/Trustlink-1/TrustLink/issues/928)
* **#929:** fix ms_sign event handler undefined variable and add end-to-end test ([90dab0a](https://github.com/Trustlink-1/TrustLink/commit/90dab0a4221ffd5eaa84e3b47f040775c43502f1)), closes [#929](https://github.com/Trustlink-1/TrustLink/issues/929)
* 910: Fix build - resolve duplicate methods and add missing error variants ([b570847](https://github.com/Trustlink-1/TrustLink/commit/b57084762b571721a642eb8bd67c3ed55df482fb))
* 911: Remove duplicate StorageKey enum variants ([c05a39e](https://github.com/Trustlink-1/TrustLink/commit/c05a39e2ed565e3988d8370b9a5f7b230083e2b1))
* 912: Remove duplicate struct definitions in types.rs ([9d204af](https://github.com/Trustlink-1/TrustLink/commit/9d204af23239e799421ba141a71ae43358c88530))
* 913: Ensure MultiSigProposal has cancelled field ([4f9b7a9](https://github.com/Trustlink-1/TrustLink/commit/4f9b7a9739da784870be9e402396dff59ebdbf3f))
* add API key authorization to admin REST endpoints ([5ac1edb](https://github.com/Trustlink-1/TrustLink/commit/5ac1edbfac29d9b6610f5c6c7bace87d29147507)), closes [#935](https://github.com/Trustlink-1/TrustLink/issues/935)
* add cleanup mechanism for expired attestation requests (issue [#940](https://github.com/Trustlink-1/TrustLink/issues/940)) ([8a255d1](https://github.com/Trustlink-1/TrustLink/commit/8a255d1db1b3d63f09cca78b57b8dfa7c9bc8d6e))
* add client-side input validation before constructing ScVals ([df8470b](https://github.com/Trustlink-1/TrustLink/commit/df8470b5b6f826c4586baec16a0edfe94ca237e5))
* add root vercel.json to fix 404 on Vercel deploys ([a920375](https://github.com/Trustlink-1/TrustLink/commit/a920375e3728632210ef33048a4b7af9918fd5ba))
* **anchor-integration:** robust error handling with parseTrustLinkError ([bd13ab3](https://github.com/Trustlink-1/TrustLink/commit/bd13ab38cd657e8dc1e3f267c5b70c8a0dc5536b)), closes [#568](https://github.com/Trustlink-1/TrustLink/issues/568)
* compliance enforcement, admin alerts, reproducible builds, benchmark docs ([fe49cc5](https://github.com/Trustlink-1/TrustLink/commit/fe49cc521e0cfb36ab62c987fdaf2d1c0277ac69)), closes [#601](https://github.com/Trustlink-1/TrustLink/issues/601) [#602](https://github.com/Trustlink-1/TrustLink/issues/602) [#603](https://github.com/Trustlink-1/TrustLink/issues/603) [#590](https://github.com/Trustlink-1/TrustLink/issues/590)
* compliance enforcement, admin alerts, reproducible builds, benchmark docs (PR [#832](https://github.com/Trustlink-1/TrustLink/issues/832)) ([9151ac7](https://github.com/Trustlink-1/TrustLink/commit/9151ac72cc24ec70e7aa94fb381c6725020de859))
* enforce require_not_paused in cancel_multisig_proposal ([3e40e2a](https://github.com/Trustlink-1/TrustLink/commit/3e40e2aa7350a9d78bafe61af3727948e965e433)), closes [#950](https://github.com/Trustlink-1/TrustLink/issues/950)
* filter expired pending requests and add list_delegations_by_delegator ([1797e46](https://github.com/Trustlink-1/TrustLink/commit/1797e46dc7ba4910d9b73ae11693250bb1cb2c41))
* fire expiration hook in all claim-check variants and add cancel_request ([328178f](https://github.com/Trustlink-1/TrustLink/commit/328178fcf9d33adba38f84c851430db49fd98598))
* import ContractConfig in admin.rs ([348b21d](https://github.com/Trustlink-1/TrustLink/commit/348b21d6f04ca22f7feb5f5904ed6db0d62ab8eb))
* **indexer:** consistent subscription filters, dead-letter, federation ADR, scaling docs ([#974](https://github.com/Trustlink-1/TrustLink/issues/974)-[#977](https://github.com/Trustlink-1/TrustLink/issues/977)) ([638e878](https://github.com/Trustlink-1/TrustLink/commit/638e878f61ac96b1666859f88ea2fe1a105f7915))
* **indexer:** multi-stage Dockerfile and GHCR publish workflow ([948a8fb](https://github.com/Trustlink-1/TrustLink/commit/948a8fbb9dc68060a04d0da5159bb978603c511b))
* **indexer:** resolve [#974](https://github.com/Trustlink-1/TrustLink/issues/974) [#975](https://github.com/Trustlink-1/TrustLink/issues/975) [#976](https://github.com/Trustlink-1/TrustLink/issues/976) [#977](https://github.com/Trustlink-1/TrustLink/issues/977) — subscription filters, backpressure, federation, scaling ([71f01f8](https://github.com/Trustlink-1/TrustLink/commit/71f01f87b101ac761b0012020b0b89223c2262ad))
* **indexer:** uniform subject/issuer/claimType filters on GraphQL subscriptions ([b86bb2d](https://github.com/Trustlink-1/TrustLink/commit/b86bb2dee59817168b8d31348f1a476662debaab))
* **kyc-token:** require_auth before reading Admin storage in initialize ([559cce6](https://github.com/Trustlink-1/TrustLink/commit/559cce609199ff6c8232ef254ea620fd87189e75))
* **makefile:** add verify target and wire deploy reminder ([1d53448](https://github.com/Trustlink-1/TrustLink/commit/1d534482358a4e28aa0baec6c91d3493bd851bfe)), closes [#568](https://github.com/Trustlink-1/TrustLink/issues/568)
* prevent duplicate pending requests across timestamps (issue [#941](https://github.com/Trustlink-1/TrustLink/issues/941)) ([ed587dc](https://github.com/Trustlink-1/TrustLink/commit/ed587dc7fffb073edc2f1deb667b2e35527261c3))
* **query:** add cursor-based pagination for get_attestations_in_range and document deletion-safe workflow ([b8ea318](https://github.com/Trustlink-1/TrustLink/commit/b8ea31888eb42e80c8fee6cf35ab227b8d37177b))
* **react-app:** add error boundaries to prevent full app unmount on panel errors ([1fbec92](https://github.com/Trustlink-1/TrustLink/commit/1fbec92bb5ace1ddc7865d0ac19b93b888c38ca7))
* **react-app:** add skeleton loading states for attestation lists ([18c6d90](https://github.com/Trustlink-1/TrustLink/commit/18c6d90e1d3da016312fd3149af0156c6b78a168))
* regenerate stale react-app package-lock.json ([40f08f3](https://github.com/Trustlink-1/TrustLink/commit/40f08f385731d23c60e31b507e2d289d79a710c9))
* remove duplicate event functions ([#915](https://github.com/Trustlink-1/TrustLink/issues/915)) ([3f692bf](https://github.com/Trustlink-1/TrustLink/commit/3f692bfb1927e131427eb0541db2127ab96ab5b3))
* remove duplicate get_multisig_ttl function ([#916](https://github.com/Trustlink-1/TrustLink/issues/916)) ([ba5d338](https://github.com/Trustlink-1/TrustLink/commit/ba5d3388a615bab5df304d796be1087365849cf9))
* remove duplicate storage methods ([#914](https://github.com/Trustlink-1/TrustLink/issues/914)) ([e5db61f](https://github.com/Trustlink-1/TrustLink/commit/e5db61f37836a2092d052775072793f8338211e1))
* remove hardcoded API key from TypeScript SDK ([cefedcb](https://github.com/Trustlink-1/TrustLink/commit/cefedcbbb960ec138edc7bdf9a2a23abf526f877)), closes [#934](https://github.com/Trustlink-1/TrustLink/issues/934)
* remove merge conflict marker from test.rs ([9bb07b1](https://github.com/Trustlink-1/TrustLink/commit/9bb07b1c4c69dd4814df59b767a6ba0faa1583bc))
* remove orphaned/duplicated code left by prior botched merges ([06df2a5](https://github.com/Trustlink-1/TrustLink/commit/06df2a56d6c56489e2161079066309b02d93f9a1))
* remove stray code fragment in lib.rs ([#917](https://github.com/Trustlink-1/TrustLink/issues/917)) ([2b563e7](https://github.com/Trustlink-1/TrustLink/commit/2b563e72367e9c9655f91b8ae0c0350ecc935020))
* remove unused constants and dead code warnings ([540f565](https://github.com/Trustlink-1/TrustLink/commit/540f565e39b7677b827176529d3178d8163469dd))
* repair build after merging 41 PRs (missing imports, storage fns, types) ([d1fe52e](https://github.com/Trustlink-1/TrustLink/commit/d1fe52e048526ebff0916f627579c12161178e64))
* replace bubble sort with insertion sort in expiring attestations queries ([d3190ee](https://github.com/Trustlink-1/TrustLink/commit/d3190ee4af75d2f31ef6f67ef8512e25e1b58aff)), closes [#937](https://github.com/Trustlink-1/TrustLink/issues/937)
* resolve 212 compilation errors from duplicate code blocks ([95ceef3](https://github.com/Trustlink-1/TrustLink/commit/95ceef35e5df3eeee66dd537fb539da096af17c4))
* resolve compilation errors in storage, lib, types, and attestation ([196d800](https://github.com/Trustlink-1/TrustLink/commit/196d800ef26ca4598956619c5127309d960b209f))
* resolve issues [#260](https://github.com/Trustlink-1/TrustLink/issues/260), [#327](https://github.com/Trustlink-1/TrustLink/issues/327), [#329](https://github.com/Trustlink-1/TrustLink/issues/329), [#334](https://github.com/Trustlink-1/TrustLink/issues/334) ([0a9151f](https://github.com/Trustlink-1/TrustLink/commit/0a9151f0c1468fb2696122865703c4fb87dc8c56))
* resolve issues [#331](https://github.com/Trustlink-1/TrustLink/issues/331), [#367](https://github.com/Trustlink-1/TrustLink/issues/367), [#368](https://github.com/Trustlink-1/TrustLink/issues/368), [#369](https://github.com/Trustlink-1/TrustLink/issues/369) ([ca88c6b](https://github.com/Trustlink-1/TrustLink/commit/ca88c6b577fcd39695d63610112adae83f3127ce))
* resolve issues [#522](https://github.com/Trustlink-1/TrustLink/issues/522), [#523](https://github.com/Trustlink-1/TrustLink/issues/523), [#524](https://github.com/Trustlink-1/TrustLink/issues/524), [#525](https://github.com/Trustlink-1/TrustLink/issues/525) ([2da81d0](https://github.com/Trustlink-1/TrustLink/commit/2da81d0e12a60c66be68bb8e1ca2708ba9d36717))
* resolve issues [#958](https://github.com/Trustlink-1/TrustLink/issues/958) [#959](https://github.com/Trustlink-1/TrustLink/issues/959) [#960](https://github.com/Trustlink-1/TrustLink/issues/960) [#961](https://github.com/Trustlink-1/TrustLink/issues/961) ([675ce9d](https://github.com/Trustlink-1/TrustLink/commit/675ce9d4a55ce297658a5741df84061e564684ba))
* **search:** implement date-range edge cases and fix variable name bug ([12903da](https://github.com/Trustlink-1/TrustLink/commit/12903daf3b0df51083dc6cd7f16fafa03d3f50c7))
* stop blank-page crash when VITE_CONTRACT_ID is unset ([3c45451](https://github.com/Trustlink-1/TrustLink/commit/3c45451cf3f34c618fbef9b8addfa988ae0c2318))
* **types,storage,query,validation:** resolve duplicate definitions, missing StorageKey variants, and unwrap violations ([209519a](https://github.com/Trustlink-1/TrustLink/commit/209519a9f939f8e446ea01d8181b7e18e89303d5))
* validate TrustLinkClientOptions network option ([c68265c](https://github.com/Trustlink-1/TrustLink/commit/c68265c326c9d7b11fec81ef30ef7b96f6671aed))
* wire Prometheus metrics into event processing pipeline ([5a75e6a](https://github.com/Trustlink-1/TrustLink/commit/5a75e6ab7808cc56a9367743226dbd88f91dbe06)), closes [#936](https://github.com/Trustlink-1/TrustLink/issues/936)


### Performance Improvements

* add wasm-opt -Oz to build pipeline and document size reduction ([c819e60](https://github.com/Trustlink-1/TrustLink/commit/c819e60dc3c7849a59bca0b2e9fbe115efd420f7))
* benchmark and document storage cost per attestation ([7b71e90](https://github.com/Trustlink-1/TrustLink/commit/7b71e90171184e83c5189b8367f85b4d2990e123))
* implement chunked index storage for lazy partial index loading ([dd0fa34](https://github.com/Trustlink-1/TrustLink/commit/dd0fa34fe7dc0e6e035ca6c428147f23c3d34fd1))
* optimize batch attestation to write issuer index once per batch ([790a84a](https://github.com/Trustlink-1/TrustLink/commit/790a84a63d0802eb65b0abc2f43784a1eadb4b96))
* verify has_valid_claim short-circuit and add attestation benchmarks ([44cb729](https://github.com/Trustlink-1/TrustLink/commit/44cb72993a467b10a0462100ebb94a77e7c11a20))

## [Unreleased]

<!-- Add new changes here before they are released. Use the categories below:
### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security
-->

## [0.1.0] - 2026-03-25

### Added

- `initialize(admin, ttl_days)` — deploy and set the contract administrator with configurable storage TTL.
- `register_issuer(admin, issuer)` — admin registers a trusted attestation issuer.
- `remove_issuer(admin, issuer)` — admin removes an issuer from the registry.
- `is_issuer(address)` — query whether an address is an authorized issuer.
- `get_admin()` — return the current admin address.
- `transfer_admin(current_admin, new_admin)` — transfer contract administration rights.
- `create_attestation(issuer, subject, claim_type, expiration, metadata)` — issuer creates a new attestation with optional expiration and metadata; returns a deterministic hash-based ID.
- `revoke_attestation(issuer, attestation_id)` — issuer marks an attestation as revoked.
- `get_attestation(attestation_id)` — fetch full attestation data by ID.
- `get_attestation_status(attestation_id)` — return `Valid`, `Expired`, or `Revoked`; emits an `expired` event when status is `Expired`.
- `has_valid_claim(subject, claim_type)` — returns `true` if the subject holds a non-expired, non-revoked attestation of the given type; emits an `expired` event for any expired attestation encountered.
- `has_valid_claim_from_issuer(subject, claim_type, issuer)` — constrain verification to a specific issuer.
- `has_any_claim(subject, claim_types)` and `has_all_claims(subject, claim_types)` — OR/AND claim verification across multiple claim types.
- `get_subject_attestations(subject, start, limit)` — paginated list of attestation IDs for a subject.
- `get_issuer_attestations(issuer, start, limit)` — paginated list of attestation IDs issued by an issuer.
- `get_subject_attestation_count(subject)`, `get_issuer_attestation_count(issuer)`, and `get_valid_claim_count(subject)` — aggregate query helpers.
- Claim type registry: `register_claim_type`, `update_claim_type`, `remove_claim_type`, `get_claim_type_description`, and `list_claim_types`.
- Historical import support: `import_attestation(admin, issuer, subject, claim_type, timestamp, expiration)` and `Attestation.imported`.
- Fee configuration: `set_fee(admin, fee, collector, fee_token)` and `get_fee_config()` with optional token-denominated attestation fees.
- Bridge support: `register_bridge`, `remove_bridge`, `is_bridge`, and `bridge_attestation` with source-chain metadata.
- Batch operations: `create_attestations_batch` and `revoke_attestations_batch`.
- Expiration hooks: `register_expiration_hook`, `get_expiration_hook`, and `remove_expiration_hook` for callback notifications.
- Multi-signature attestations: `propose_attestation`, `cosign_attestation`, and `get_multisig_proposal`.
- Global and per-issuer statistics: `get_global_stats`, `get_issuer_stats`, and issuer tier/metadata management.
- Comprehensive event set for creation, revocation, bridge/import, fee updates, claim-type administration, multi-sig lifecycle, and expiration hooks.
- Integration examples under `examples/` including KYC token and governance-gated voting patterns.

### Fixed

- Validation coverage for metadata, tag cardinality/length, and timestamp/expiration edge cases.
- Deterministic storage/index consistency for issuer and subject attestation lookups.
- Authorization checks across admin, issuer, bridge, and multisig signer flows.

[Unreleased]: https://github.com/Haroldwonder/TrustLink/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Haroldwonder/TrustLink/releases/tag/v0.1.0
