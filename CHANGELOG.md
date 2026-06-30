# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 (2026-06-30)


### Features

* [#760](https://github.com/Haroldwonder/TrustLink/issues/760) add TemplatePanel for issuer attestation template management ([fb66692](https://github.com/Haroldwonder/TrustLink/commit/fb66692dde74f754f82b2271535aa43f7bc4c8e9))
* [#763](https://github.com/Haroldwonder/TrustLink/issues/763) real-time toast notifications for attestation events ([1a52f23](https://github.com/Haroldwonder/TrustLink/commit/1a52f23aea38ddbc658e6f5964fd67d722c08ec1))
* [#764](https://github.com/Haroldwonder/TrustLink/issues/764) add attestation history timeline view to UserPanel ([ad3c905](https://github.com/Haroldwonder/TrustLink/commit/ad3c905969d448f64b4bd090a45daf7bca7c2b62))
* [#765](https://github.com/Haroldwonder/TrustLink/issues/765) add CSV export for issuer attestation lists ([cccfd7c](https://github.com/Haroldwonder/TrustLink/commit/cccfd7c4b51a039e1c2930244942eb95d4382049))
* **#355:** add health check endpoints (/health and /ready) ([b63bbe9](https://github.com/Haroldwonder/TrustLink/commit/b63bbe9562877890c94b76013c90ea5d6061fee3))
* **#356:** add Prometheus metrics endpoint (/metrics) ([ad5a757](https://github.com/Haroldwonder/TrustLink/commit/ad5a757077119fbcacad7b7a120df0f99914650d))
* **#357:** add multi-sig proposal event indexing ([93ea49e](https://github.com/Haroldwonder/TrustLink/commit/93ea49e77509a0aa14dd1f1402f643e04a5dd131))
* **#358:** add governance/DAO voting example contract ([158c644](https://github.com/Haroldwonder/TrustLink/commit/158c644a6a63c35feb1f061499099b84b061bf7e))
* **#766:** add search and filter controls to attestation lists ([f2a4db3](https://github.com/Haroldwonder/TrustLink/commit/f2a4db3052c5befd10a543a415481192d902e024))
* **#767:** add QR code generation for sharing attestation IDs ([e6ac3b3](https://github.com/Haroldwonder/TrustLink/commit/e6ac3b356bf9ac233f585e8dd743b942fba5adf0))
* **#768:** add wallet-connection error states for unsupported networks ([2cdbdf8](https://github.com/Haroldwonder/TrustLink/commit/2cdbdf8405558d07d9fb3c425a201180f1db38ff))
* **#769:** add RateLimitPanel showing issuer rate-limit usage ([0414913](https://github.com/Haroldwonder/TrustLink/commit/0414913245d8fee2ab3b75b052b765a2653a848c))
* **#798:** add ADR-010 for issuer-defined custom validation hooks ([8bf2bf2](https://github.com/Haroldwonder/TrustLink/commit/8bf2bf2066b55452423504762d4f55c59fcf03dc))
* **#799:** add k6 load test for indexer GraphQL API ([b68a099](https://github.com/Haroldwonder/TrustLink/commit/b68a099a78bc6c51379e6d554a0f6de266bf72df))
* **#806:** add multi-architecture Docker builds for indexer (amd64 + arm64) ([c4fa150](https://github.com/Haroldwonder/TrustLink/commit/c4fa1508420350f7b3b4c7ba96e1c6758a779417))
* **#807:** add infrastructure cost monitoring and budget alerts (AWS Budgets + SNS) ([8c66e95](https://github.com/Haroldwonder/TrustLink/commit/8c66e95992af72a90b68fd7b99df14d561eb4559))
* **#808:** add post-release SDK and indexer compatibility smoke test ([94f6656](https://github.com/Haroldwonder/TrustLink/commit/94f6656d29d898ca05980cf0fb4ac13df1919753))
* **#814:** add supply-chain provenance verification example with multi-issuer attestations ([95445e8](https://github.com/Haroldwonder/TrustLink/commit/95445e8429ae6cad9dfbddc176bedade18bfecd3))
* **#814:** Supply-chain provenance verification with multi-issuer attestations ([#848](https://github.com/Haroldwonder/TrustLink/issues/848)) ([1eabdc7](https://github.com/Haroldwonder/TrustLink/commit/1eabdc74fd0a647eea6c516c8516b666d83bcef6))
* **798:** add custom claim type constraint validation hooks ([c1a2c6b](https://github.com/Haroldwonder/TrustLink/commit/c1a2c6bee15c80711d810e6b1479aa655d796d1b)), closes [#798](https://github.com/Haroldwonder/TrustLink/issues/798)
* add claim type existence check with require_registered_claim_type config ([408fd28](https://github.com/Haroldwonder/TrustLink/commit/408fd2861f475667c3d9a7266a239907a2c1f9fb))
* add clippy CI, coverage threshold, multisig CLI commands, and import command ([eb4a013](https://github.com/Haroldwonder/TrustLink/commit/eb4a0130df6b9a2a83696ad2a13f1461f245eea7)), closes [#374](https://github.com/Haroldwonder/TrustLink/issues/374) [#375](https://github.com/Haroldwonder/TrustLink/issues/375) [#566](https://github.com/Haroldwonder/TrustLink/issues/566) [#567](https://github.com/Haroldwonder/TrustLink/issues/567)
* add CODEOWNERS, ValidAttestations index, and batch benchmarks ([b177bc1](https://github.com/Haroldwonder/TrustLink/commit/b177bc15c18a1efe7978e7a65ff2698a404ba80d)), closes [#593](https://github.com/Haroldwonder/TrustLink/issues/593) [#594](https://github.com/Haroldwonder/TrustLink/issues/594) [#596](https://github.com/Haroldwonder/TrustLink/issues/596)
* add custom claim type constraint validation hooks (closes [#798](https://github.com/Haroldwonder/TrustLink/issues/798), PR [#855](https://github.com/Haroldwonder/TrustLink/issues/855)) ([89b2281](https://github.com/Haroldwonder/TrustLink/commit/89b2281eaa560b23244bfbfe3536073e8caf2c07))
* add freelance reputation example, subject data export, audit trail CLI, and sanctions screening docs ([76b816f](https://github.com/Haroldwonder/TrustLink/commit/76b816fc9a98be604ec5c58c9357537936092df8))
* add freelance reputation example, subject data export, audit trail, screening ([#854](https://github.com/Haroldwonder/TrustLink/issues/854)) ([dbf57fa](https://github.com/Haroldwonder/TrustLink/commit/dbf57fa20ddfd72128e190e483334bf05c91a050))
* add get_bridge_list() paginated query ([d51a4e6](https://github.com/Haroldwonder/TrustLink/commit/d51a4e60ed356aa4acb34a643bf7129770d98ae8))
* add get_issuer_list() paginated query ([fa9d85b](https://github.com/Haroldwonder/TrustLink/commit/fa9d85b1d913c23e445fceeefe2540c08e9eae08))
* add global stats integration test and fix pre-existing compilation errors ([4e768de](https://github.com/Haroldwonder/TrustLink/commit/4e768de38fdfefb5b3701e727357acdc605e837b))
* add insurance policy underwriting example with KYC and AML verification ([d788ca8](https://github.com/Haroldwonder/TrustLink/commit/d788ca8d62bedb186c09bfbc55b6e70eb7c06202))
* add IssuerTier enforcement to attestation weight and logic ([#305](https://github.com/Haroldwonder/TrustLink/issues/305)) ([3d7cf18](https://github.com/Haroldwonder/TrustLink/commit/3d7cf186984e7662b2c050dd4bb23b435c381101))
* add pagination to get_attestations_by_jurisdiction ([#307](https://github.com/Haroldwonder/TrustLink/issues/307)) ([1c8358c](https://github.com/Haroldwonder/TrustLink/commit/1c8358c75e5c1922e2a4c22b8e085c296504b798))
* Add Python bindings with get_audit_log() and GraphQL pagination ([c5d0102](https://github.com/Haroldwonder/TrustLink/commit/c5d0102d4aa1ad7e4f9175f23f5aac6e4cbd3c5e))
* add quickstart, real-estate and healthcare examples, WCAG audit ([#860](https://github.com/Haroldwonder/TrustLink/issues/860)) ([6990002](https://github.com/Haroldwonder/TrustLink/commit/69900029e5932cb9bada8590c010c956ad6dca2e))
* add quickstart, real-estate and healthcare examples, WCAG audit fixes ([#809](https://github.com/Haroldwonder/TrustLink/issues/809) [#815](https://github.com/Haroldwonder/TrustLink/issues/815) [#816](https://github.com/Haroldwonder/TrustLink/issues/816) [#825](https://github.com/Haroldwonder/TrustLink/issues/825)) ([0d86a69](https://github.com/Haroldwonder/TrustLink/commit/0d86a69569a6c62fe17f5dd4139361cff271b209))
* add SDK methods for issues [#744](https://github.com/Haroldwonder/TrustLink/issues/744) [#745](https://github.com/Haroldwonder/TrustLink/issues/745) [#746](https://github.com/Haroldwonder/TrustLink/issues/746) [#747](https://github.com/Haroldwonder/TrustLink/issues/747) ([b8d6736](https://github.com/Haroldwonder/TrustLink/commit/b8d673603ecdca1b67416a7151f8187576333c49))
* add security hardening and indexer reliability improvements ([0e10d64](https://github.com/Haroldwonder/TrustLink/commit/0e10d641df6e7e9b74f7bd7c156b6e1642107fce))
* add Template, Delegation, WhitelistEntry, CouncilAction to indexer ([#770](https://github.com/Haroldwonder/TrustLink/issues/770) [#771](https://github.com/Haroldwonder/TrustLink/issues/771) [#772](https://github.com/Haroldwonder/TrustLink/issues/772) [#773](https://github.com/Haroldwonder/TrustLink/issues/773)) ([9a9a7c1](https://github.com/Haroldwonder/TrustLink/commit/9a9a7c12cf35ba677bf669d06e94211321dbe8ea))
* add two-step admin transfer with pending confirmation ([#284](https://github.com/Haroldwonder/TrustLink/issues/284)) ([b14d591](https://github.com/Haroldwonder/TrustLink/commit/b14d591b442d7646b6d022d12088e8ad1529ea2b))
* add TypeScript SDK with range queries, council, and limits methods ([1c347d2](https://github.com/Haroldwonder/TrustLink/commit/1c347d27eda2c3d1d440f11584539dda0ec80d56))
* add underflow-safe counter tracking for issuers, attestations, revocations ([81bdc4e](https://github.com/Haroldwonder/TrustLink/commit/81bdc4ee5520f0ec880ebcdc60544b3cfe9ffb88))
* add wallet disconnect button ([3b4efe8](https://github.com/Haroldwonder/TrustLink/commit/3b4efe8742af8b10d5bf4f35ed9e1f96e194b842))
* add whitelist queries, delegation list, template list, and Python valid-claims methods ([28146ff](https://github.com/Haroldwonder/TrustLink/commit/28146ff4dddea8daada3efcfac33d86a91017b52))
* add whitelist queries, delegation list, template list, and Python valid-claims methods (PR [#850](https://github.com/Haroldwonder/TrustLink/issues/850)) ([2885e41](https://github.com/Haroldwonder/TrustLink/commit/2885e4192b5aeef8b7aca826e945bdfa2a9c6cbe))
* **attestation:** implement transfer_attestation for compromised issuer recovery ([812964c](https://github.com/Haroldwonder/TrustLink/commit/812964cee0592960643e8d48c79496b4fbaabfea))
* **bindings,indexer:** add input validation and monitoring improvements ([0f2da49](https://github.com/Haroldwonder/TrustLink/commit/0f2da49250e67fd606f98c0fb2f85cdb50b013c5))
* **bindings:** add Python bindings for TrustLink contract ([#363](https://github.com/Haroldwonder/TrustLink/issues/363)) ([cbe8063](https://github.com/Haroldwonder/TrustLink/commit/cbe8063ca8754d054f0696b6beff13e48e851b68))
* **bindings:** document auto-generation of TypeScript bindings from contract ABI ([#362](https://github.com/Haroldwonder/TrustLink/issues/362)) ([394a42c](https://github.com/Haroldwonder/TrustLink/commit/394a42cf8cccfe137d350436e6aae03da429f88f))
* **build:** add check-wasm-size target and changelog-preview command ([9e4a478](https://github.com/Haroldwonder/TrustLink/commit/9e4a47800cbe771b644eae507145969470762302))
* council timelock, attestation dispute, reputation decay, amendment history ([#790](https://github.com/Haroldwonder/TrustLink/issues/790)-[#793](https://github.com/Haroldwonder/TrustLink/issues/793)) ([1a27697](https://github.com/Haroldwonder/TrustLink/commit/1a276975c7f5f600d4ae394c3b0a4031d11a4136))
* **examples:** add CLI tool for issuer operations ([#361](https://github.com/Haroldwonder/TrustLink/issues/361)) ([3257325](https://github.com/Haroldwonder/TrustLink/commit/3257325126f72dcae675535defbf6d3d1628bad8))
* **examples:** add Python server-side verification example ([#360](https://github.com/Haroldwonder/TrustLink/issues/360)) ([34e9455](https://github.com/Haroldwonder/TrustLink/commit/34e945542eda314bd3c78786b7d980a818e5f520))
* **examples:** complete anchor-integration flow with expiration handling ([#359](https://github.com/Haroldwonder/TrustLink/issues/359)) ([813d75c](https://github.com/Haroldwonder/TrustLink/commit/813d75c6b2b9374fe5af9abf7ba17b6f5b5114bc))
* expose expiration hook registration and notification flow ([#319](https://github.com/Haroldwonder/TrustLink/issues/319)) ([2c4f642](https://github.com/Haroldwonder/TrustLink/commit/2c4f642895f15b68573ae986014902705eb9aaf4))
* expose get_pending_admin_transfer() as read-only query ([9018d0b](https://github.com/Haroldwonder/TrustLink/commit/9018d0b44bf8d00129c7381c736fd780af248d78))
* **governance:** add proposal deadline enforcement to vote function ([14a7435](https://github.com/Haroldwonder/TrustLink/commit/14a74351c172ed3f33a8fd8b963510e2cc8fff6e))
* **governance:** implement M-of-N council quorum for sensitive admin operations ([94dcfad](https://github.com/Haroldwonder/TrustLink/commit/94dcfad3f1571c8241681aab4d4ac247330d147a)), closes [#268](https://github.com/Haroldwonder/TrustLink/issues/268)
* implement Add tests for admin council operations ([e94e739](https://github.com/Haroldwonder/TrustLink/commit/e94e739fd5b42691b6d8ca879b02bd91e70de9a3))
* implement Add tests for claim type registry pagination ([4259fb7](https://github.com/Haroldwonder/TrustLink/commit/4259fb7cf3d0181dd2e0ca6faef49a53fabb02ff))
* implement attestation templates (create, instantiate, list, get) ([dccfd75](https://github.com/Haroldwonder/TrustLink/commit/dccfd75d5cd8b45b8a470277ca7ca11cb603abc3))
* implement attestation valid_from lifecycle with Pending status ([7616c0c](https://github.com/Haroldwonder/TrustLink/commit/7616c0c213257f3572e932cdf9b77b4acf8a844b))
* implement issuer delegation (sub-issuer authority) ([#298](https://github.com/Haroldwonder/TrustLink/issues/298)) ([#431](https://github.com/Haroldwonder/TrustLink/issues/431)) ([88259db](https://github.com/Haroldwonder/TrustLink/commit/88259db01201f8f32232044126fa1d54f511a100))
* implement issues [#605](https://github.com/Haroldwonder/TrustLink/issues/605), [#606](https://github.com/Haroldwonder/TrustLink/issues/606), [#607](https://github.com/Haroldwonder/TrustLink/issues/607), [#608](https://github.com/Haroldwonder/TrustLink/issues/608) ([9662b9e](https://github.com/Haroldwonder/TrustLink/commit/9662b9e8e6dd4728939eb76d1075694e1c1c12d6))
* implement issues [#609](https://github.com/Haroldwonder/TrustLink/issues/609) [#610](https://github.com/Haroldwonder/TrustLink/issues/610) [#611](https://github.com/Haroldwonder/TrustLink/issues/611) [#612](https://github.com/Haroldwonder/TrustLink/issues/612) ([9c458dc](https://github.com/Haroldwonder/TrustLink/commit/9c458dc20f587a03c86463dff93318a810de024d))
* implement mainnet-checklist.md: add post-deployment verification steps ([9427268](https://github.com/Haroldwonder/TrustLink/commit/94272689cb30286bf0456a01650fe9d435061eb6))
* **indexer:** add API key auth, depth/complexity limits, pino logging, OTel tracing ([7b42548](https://github.com/Haroldwonder/TrustLink/commit/7b425489ce6ab1d7a5624d31ad794575512b1226))
* **indexer:** add attestation request persistence and GraphQL query support ([66a811f](https://github.com/Haroldwonder/TrustLink/commit/66a811f0d77b7b4de6e6d554bec21640c6ca5e8c)), closes [#545](https://github.com/Haroldwonder/TrustLink/issues/545)
* **indexer:** add database indexes for common query patterns ([#352](https://github.com/Haroldwonder/TrustLink/issues/352)) ([ec804ff](https://github.com/Haroldwonder/TrustLink/commit/ec804ff5aefceecf9fe811f14e7db68216e952bb))
* **indexer:** add event replay from genesis for full historical sync ([#354](https://github.com/Haroldwonder/TrustLink/issues/354)) ([8778ff6](https://github.com/Haroldwonder/TrustLink/commit/8778ff67a22fff99b7ba1d39b47aede533c65c61))
* **indexer:** add GraphQL subscriptions for real-time events ([#351](https://github.com/Haroldwonder/TrustLink/issues/351)) ([fd6c3ab](https://github.com/Haroldwonder/TrustLink/commit/fd6c3ab4b93e80020ff4e1e0141396b7989da8e2))
* **indexer:** add indexer-dev, indexer-build, indexer-logs Makefile targets ([d949067](https://github.com/Haroldwonder/TrustLink/commit/d949067091edf3f9a4b662bdf2797bbebeb0035d)), closes [#576](https://github.com/Haroldwonder/TrustLink/issues/576)
* **indexer:** add issuer management, health checks, and reindex capabilities ([68fe00b](https://github.com/Haroldwonder/TrustLink/commit/68fe00b9ca6a31f7f4ad7d7bb78ad72f9ba2cc2f))
* **indexer:** add multi-sig proposal persistence and GraphQL query support ([3c3a1c8](https://github.com/Haroldwonder/TrustLink/commit/3c3a1c8a05c05013b34c826b839c66b4629f8e2d))
* **indexer:** add REST API endpoints alongside GraphQL ([#353](https://github.com/Haroldwonder/TrustLink/issues/353)) ([c8fe448](https://github.com/Haroldwonder/TrustLink/commit/c8fe4487030dfb3ff1cf8e9d33a938b1e4a462f3))
* **indexer:** add Template, Delegation, WhitelistEntry, CouncilAction types and GraphQL queries (closes [#770](https://github.com/Haroldwonder/TrustLink/issues/770), [#771](https://github.com/Haroldwonder/TrustLink/issues/771), [#772](https://github.com/Haroldwonder/TrustLink/issues/772), [#773](https://github.com/Haroldwonder/TrustLink/issues/773), PR [#837](https://github.com/Haroldwonder/TrustLink/issues/837)) ([a38ebb9](https://github.com/Haroldwonder/TrustLink/commit/a38ebb9a81f27602aca504267d84f5ac58292d81))
* **indexer:** audit log, issuer rate limit, revocation reason, Redis cache ([#774](https://github.com/Haroldwonder/TrustLink/issues/774)-[#777](https://github.com/Haroldwonder/TrustLink/issues/777)) ([5168312](https://github.com/Haroldwonder/TrustLink/commit/5168312a23762243210d53ebb3502c1bead3cb7a))
* **indexer:** durable webhook failure handling and recovery ([d7f5308](https://github.com/Haroldwonder/TrustLink/commit/d7f53081b1fc808d38641062953b05713e217656)), closes [#545](https://github.com/Haroldwonder/TrustLink/issues/545)
* **issue-796:** Add batch query for has_valid_claim across multiple subjects ([a70e9fe](https://github.com/Haroldwonder/TrustLink/commit/a70e9fe0daed6342ccb948e4b857d0aa224fd636))
* issues 756-757-758-759 — Python template/delegation bindings, client unification, CouncilPanel ([d24c09b](https://github.com/Haroldwonder/TrustLink/commit/d24c09bac5660be2a513ce87c13f3b08020f8226))
* **makefile:** add snapshot-update target and snapshot testing docs ([976270c](https://github.com/Haroldwonder/TrustLink/commit/976270ca4dbd6527d25b48c3b180672b25119407))
* **monitoring:** define SLOs, synthetic uptime checks, issuer dashboard, and revocation spike alert ([abac26d](https://github.com/Haroldwonder/TrustLink/commit/abac26d62b92589abe2156827c5898e57e53d548)), closes [#821](https://github.com/Haroldwonder/TrustLink/issues/821) [#822](https://github.com/Haroldwonder/TrustLink/issues/822) [#823](https://github.com/Haroldwonder/TrustLink/issues/823) [#824](https://github.com/Haroldwonder/TrustLink/issues/824)
* **monitoring:** define SLOs, synthetic uptime checks, issuer dashboard, and revocation spike alert ([#835](https://github.com/Haroldwonder/TrustLink/issues/835)) ([d0b3ba4](https://github.com/Haroldwonder/TrustLink/commit/d0b3ba49d962c40f696ce9fc157157e79cc54acf))
* multisig configurable TTL, cancel proposal, and list open proposals ([3b13f6d](https://github.com/Haroldwonder/TrustLink/commit/3b13f6ded8f7b1596f126da1874d8b79b07c2798))
* **python-sdk:** add has_all_claims and has_any_claim with validation and tests ([b955812](https://github.com/Haroldwonder/TrustLink/commit/b95581231ba3b4d3579bdf65715d06a3bdef19f5)), closes [#545](https://github.com/Haroldwonder/TrustLink/issues/545)
* **python:** add AsyncTrustLinkClient for asyncio support ([8b21631](https://github.com/Haroldwonder/TrustLink/commit/8b2163152dd46b69d4c608ad75a13526017ea920)), closes [#540](https://github.com/Haroldwonder/TrustLink/issues/540)
* **python:** add config/metadata, async write methods, multisig read, and whitelist support ([4ad94da](https://github.com/Haroldwonder/TrustLink/commit/4ad94daee1cbe3513833cbf02f96775ceae9111e))
* **python:** add config/metadata, async write methods, multisig read, and whitelist support (PR [#851](https://github.com/Haroldwonder/TrustLink/issues/851)) ([6b13a3c](https://github.com/Haroldwonder/TrustLink/commit/6b13a3cdb96df5d08308efe2093af930f997af22))
* **python:** prepare trustlink-sdk for PyPI distribution ([7daa0fa](https://github.com/Haroldwonder/TrustLink/commit/7daa0fae2635bbed8fe8359cdfd83fded1386eb6))
* React app — CSV export, attestation timeline, toast notifications, and template management (closes [#760](https://github.com/Haroldwonder/TrustLink/issues/760), [#763](https://github.com/Haroldwonder/TrustLink/issues/763), [#764](https://github.com/Haroldwonder/TrustLink/issues/764), [#765](https://github.com/Haroldwonder/TrustLink/issues/765), PR [#836](https://github.com/Haroldwonder/TrustLink/issues/836)) ([36e1ea0](https://github.com/Haroldwonder/TrustLink/commit/36e1ea0ba5dca51c16f785e874fda145a8e9df39))
* React app UX improvements — rate limits, network check, QR codes, attestation filters (closes [#766](https://github.com/Haroldwonder/TrustLink/issues/766), [#767](https://github.com/Haroldwonder/TrustLink/issues/767), [#768](https://github.com/Haroldwonder/TrustLink/issues/768), [#769](https://github.com/Haroldwonder/TrustLink/issues/769), PR [#838](https://github.com/Haroldwonder/TrustLink/issues/838)) ([b272512](https://github.com/Haroldwonder/TrustLink/commit/b2725122183518512b95c2284e34eeb8eb75cf82))
* **react-app:** add DelegationPanel for sub-issuer delegation management ([#762](https://github.com/Haroldwonder/TrustLink/issues/762)) ([3a6c916](https://github.com/Haroldwonder/TrustLink/commit/3a6c91696bb27570717de35fdaf84dc839c8fecd))
* **react-app:** add DelegationPanel for sub-issuer delegation management (closes [#762](https://github.com/Haroldwonder/TrustLink/issues/762), PR [#833](https://github.com/Haroldwonder/TrustLink/issues/833)) ([97f204a](https://github.com/Haroldwonder/TrustLink/commit/97f204aff70e4dace5411050c662710553060af5))
* **react-app:** add expiring attestations section with renewal to IssuerDashboard ([043eda5](https://github.com/Haroldwonder/TrustLink/commit/043eda5e221c581941193efb8cfc5dc244fa79af)), closes [#562](https://github.com/Haroldwonder/TrustLink/issues/562)
* **react-app:** add useGlobalStats hook and refactor AdminPanel ([dc4ba44](https://github.com/Haroldwonder/TrustLink/commit/dc4ba44bfdee263cf5ad9651705f05d509cbf021)), closes [#539](https://github.com/Haroldwonder/TrustLink/issues/539)
* **react-app:** add WhitelistPanel for issuer whitelist management ([#761](https://github.com/Haroldwonder/TrustLink/issues/761)) ([105c454](https://github.com/Haroldwonder/TrustLink/commit/105c4545dc489da90ae8d0ff0c1270f75da7ae4b))
* **react-app:** add WhitelistPanel for issuer whitelist management (closes [#761](https://github.com/Haroldwonder/TrustLink/issues/761), PR [#834](https://github.com/Haroldwonder/TrustLink/issues/834)) ([9c3871f](https://github.com/Haroldwonder/TrustLink/commit/9c3871f7c8fd86b0c1b18e2746ff9673ed179533))
* **react:** add attestation request flow UI ([#364](https://github.com/Haroldwonder/TrustLink/issues/364)) ([d553342](https://github.com/Haroldwonder/TrustLink/commit/d5533427fd97a0cd1ee34d0a61431e9a6548c447))
* **react:** add issuer dashboard with stats ([#366](https://github.com/Haroldwonder/TrustLink/issues/366)) ([fb94038](https://github.com/Haroldwonder/TrustLink/commit/fb940389935170feff7e9832d96326d993403d78))
* **react:** add multi-sig proposal UI ([#365](https://github.com/Haroldwonder/TrustLink/issues/365)) ([2b4917e](https://github.com/Haroldwonder/TrustLink/commit/2b4917ee8924c1907406c38d0df2dfc982c2f256))
* **requests:** implement attestation request workflow ([#304](https://github.com/Haroldwonder/TrustLink/issues/304)) ([465f535](https://github.com/Haroldwonder/TrustLink/commit/465f535f776b8af092bf7ac1265f29c1c104a824))
* resolve issues [#506](https://github.com/Haroldwonder/TrustLink/issues/506) [#507](https://github.com/Haroldwonder/TrustLink/issues/507) [#508](https://github.com/Haroldwonder/TrustLink/issues/508) [#509](https://github.com/Haroldwonder/TrustLink/issues/509) ([c69deac](https://github.com/Haroldwonder/TrustLink/commit/c69deacaae1415000ec890ff4dc35e0575d796aa))
* resolve issues [#526](https://github.com/Haroldwonder/TrustLink/issues/526), [#527](https://github.com/Haroldwonder/TrustLink/issues/527), [#528](https://github.com/Haroldwonder/TrustLink/issues/528), [#529](https://github.com/Haroldwonder/TrustLink/issues/529) ([b7f2319](https://github.com/Haroldwonder/TrustLink/commit/b7f23198b9ca9bf7e81f46531b55e157ab475a7a))
* resolve issues [#530](https://github.com/Haroldwonder/TrustLink/issues/530), [#531](https://github.com/Haroldwonder/TrustLink/issues/531), [#532](https://github.com/Haroldwonder/TrustLink/issues/532) — templates, tier claims, analytics ([1efb88b](https://github.com/Haroldwonder/TrustLink/commit/1efb88b7ad71669a7e8816ecde249ffe1c4b1a87))
* SDK methods for getMultisigTtl, getRateLimitForClaimType, getRegisteredClaimType, getRequest (closes [#744](https://github.com/Haroldwonder/TrustLink/issues/744), [#745](https://github.com/Haroldwonder/TrustLink/issues/745), [#746](https://github.com/Haroldwonder/TrustLink/issues/746), [#747](https://github.com/Haroldwonder/TrustLink/issues/747), PR [#839](https://github.com/Haroldwonder/TrustLink/issues/839)) ([e8decbc](https://github.com/Haroldwonder/TrustLink/commit/e8decbcf61af3b76a90157dc931d00d19a6bde92))
* **sdk/react:** add useIssuerStats hook and refactor IssuerDashboard ([502260b](https://github.com/Haroldwonder/TrustLink/commit/502260ba5db0bdf7361a472d57b86aec71568421)), closes [#538](https://github.com/Haroldwonder/TrustLink/issues/538)
* **sdk:** add get_delegation() read function to TypeScript SDK ([5d83399](https://github.com/Haroldwonder/TrustLink/commit/5d83399382094b86d8b040edcc6384359f0dc171))
* **sdk:** add missing contract methods to TypeScript client ([16e2402](https://github.com/Haroldwonder/TrustLink/commit/16e2402b4ec5f78de82fa648bf577597b302837c))
* **sdk:** add React hooks package ([48eb64d](https://github.com/Haroldwonder/TrustLink/commit/48eb64da68dcabed2fc536e6322aeeffd72be49c)), closes [#350](https://github.com/Haroldwonder/TrustLink/issues/350)
* **sdk:** add ResilienceConfig, provenance, iterateSubjectAttestations docs, and TypeDoc generation ([6d3e152](https://github.com/Haroldwonder/TrustLink/commit/6d3e1526299e6d9aba40b192cb2bc73e83b39a93))
* **sdk:** add typed error classes to TypeScript client ([22d9442](https://github.com/Haroldwonder/TrustLink/commit/22d94423ad45ee79ee3eb580fd036ec2894623b2)), closes [#347](https://github.com/Haroldwonder/TrustLink/issues/347)
* **sdk:** sync TypeScript types with Rust contract types ([1b5fbe4](https://github.com/Haroldwonder/TrustLink/commit/1b5fbe4ff15cd054fe8e3ff11c288dbd94fe0949))
* **security:** add cargo-deny integration and dependency security policy ([8262d07](https://github.com/Haroldwonder/TrustLink/commit/8262d07c81d0c6ca8f36037c40cc3f8c89d6b59a))
* **tiers:** add IssuerTier enforcement to attestation weight ([#305](https://github.com/Haroldwonder/TrustLink/issues/305)) ([1674ddc](https://github.com/Haroldwonder/TrustLink/commit/1674ddc3c6a61b66a40f23637927cf3a1bb5b039))
* validate jurisdiction field against ISO 3166-1 alpha-2 codes ([a373287](https://github.com/Haroldwonder/TrustLink/commit/a373287c28c577f575bfc625aaf420693b8be0bd))
* xBull wallet support, i18n, bulk-create, and template CLI commands (closes [#826](https://github.com/Haroldwonder/TrustLink/issues/826), [#827](https://github.com/Haroldwonder/TrustLink/issues/827), [#828](https://github.com/Haroldwonder/TrustLink/issues/828), [#829](https://github.com/Haroldwonder/TrustLink/issues/829)) ([30c15b6](https://github.com/Haroldwonder/TrustLink/commit/30c15b6a997d5c70ec80c96e8363a4991265ea58))


### Bug Fixes

* **#533:** renew_attestation records new expiration in audit log details ([a749f51](https://github.com/Haroldwonder/TrustLink/commit/a749f515cd5086d1c50f1f3b400a17035b56b746))
* **#558, #559:** add dark mode toggle and getAttestationsByTag pagination ([8c033b2](https://github.com/Haroldwonder/TrustLink/commit/8c033b25d5c6d84678990d450e5186beb5286ab1)), closes [#558](https://github.com/Haroldwonder/TrustLink/issues/558)
* **anchor-integration:** robust error handling with parseTrustLinkError ([bd13ab3](https://github.com/Haroldwonder/TrustLink/commit/bd13ab38cd657e8dc1e3f267c5b70c8a0dc5536b)), closes [#568](https://github.com/Haroldwonder/TrustLink/issues/568)
* centralize TTL constants in types.rs, remove raw literals ([#277](https://github.com/Haroldwonder/TrustLink/issues/277)) ([c871bd5](https://github.com/Haroldwonder/TrustLink/commit/c871bd51d4b112115d006a4ac46cc3d323cc2c30))
* compliance enforcement, admin alerts, reproducible builds, benchmark docs ([fe49cc5](https://github.com/Haroldwonder/TrustLink/commit/fe49cc521e0cfb36ab62c987fdaf2d1c0277ac69)), closes [#601](https://github.com/Haroldwonder/TrustLink/issues/601) [#602](https://github.com/Haroldwonder/TrustLink/issues/602) [#603](https://github.com/Haroldwonder/TrustLink/issues/603) [#590](https://github.com/Haroldwonder/TrustLink/issues/590)
* compliance enforcement, admin alerts, reproducible builds, benchmark docs (PR [#832](https://github.com/Haroldwonder/TrustLink/issues/832)) ([9151ac7](https://github.com/Haroldwonder/TrustLink/commit/9151ac72cc24ec70e7aa94fb381c6725020de859))
* enforce CEI pattern in create_attestation, document reentrancy ([#275](https://github.com/Haroldwonder/TrustLink/issues/275)) ([6377303](https://github.com/Haroldwonder/TrustLink/commit/637730322a15950da4bf1013809cdaa5ebba63a1))
* enforce RateLimitConfig per issuer in create_attestation ([#282](https://github.com/Haroldwonder/TrustLink/issues/282)) ([aae8f5c](https://github.com/Haroldwonder/TrustLink/commit/aae8f5ccddb5bd0ec4467378aa254f2bab4285ad))
* **errors:** replace raw panics with typed Error variants ([#283](https://github.com/Haroldwonder/TrustLink/issues/283)) ([574bf08](https://github.com/Haroldwonder/TrustLink/commit/574bf085c2dac65909d93c95d6856889ff308138))
* **errors:** replace raw panics with typed Error variants ([#283](https://github.com/Haroldwonder/TrustLink/issues/283)) ([cddf4b6](https://github.com/Haroldwonder/TrustLink/commit/cddf4b6a7d29d4495c0de092e7c58c762396d0f2))
* **errors:** replace raw panics with typed Error variants ([#283](https://github.com/Haroldwonder/TrustLink/issues/283)) ([e7c3094](https://github.com/Haroldwonder/TrustLink/commit/e7c30947cb031b4532466a727c574ae4700d728e))
* **errors:** replace raw panics with typed Error variants ([#283](https://github.com/Haroldwonder/TrustLink/issues/283)) ([#434](https://github.com/Haroldwonder/TrustLink/issues/434)) ([73c9be4](https://github.com/Haroldwonder/TrustLink/commit/73c9be4cdf1f4342b2872532293bfe242c5af55b))
* filter expired pending requests and add list_delegations_by_delegator ([1797e46](https://github.com/Haroldwonder/TrustLink/commit/1797e46dc7ba4910d9b73ae11693250bb1cb2c41))
* fire expiration hook in all claim-check variants and add cancel_request ([328178f](https://github.com/Haroldwonder/TrustLink/commit/328178fcf9d33adba38f84c851430db49fd98598))
* import ContractConfig in admin.rs ([348b21d](https://github.com/Haroldwonder/TrustLink/commit/348b21d6f04ca22f7feb5f5904ed6db0d62ab8eb))
* **indexer:** multi-stage Dockerfile and GHCR publish workflow ([948a8fb](https://github.com/Haroldwonder/TrustLink/commit/948a8fbb9dc68060a04d0da5159bb978603c511b))
* **kyc-token:** require_auth before reading Admin storage in initialize ([559cce6](https://github.com/Haroldwonder/TrustLink/commit/559cce609199ff6c8232ef254ea620fd87189e75))
* **makefile:** add verify target and wire deploy reminder ([1d53448](https://github.com/Haroldwonder/TrustLink/commit/1d534482358a4e28aa0baec6c91d3493bd851bfe)), closes [#568](https://github.com/Haroldwonder/TrustLink/issues/568)
* prevent bridge contracts from being registered as issuers ([#288](https://github.com/Haroldwonder/TrustLink/issues/288)) ([b0eda6c](https://github.com/Haroldwonder/TrustLink/commit/b0eda6ce89560fcfc41d77ebb15f13ac6ed9001e))
* prevent bridge contracts from being registered as issuers ([#288](https://github.com/Haroldwonder/TrustLink/issues/288)) ([3dfcb6b](https://github.com/Haroldwonder/TrustLink/commit/3dfcb6b16a59fa5d9d4e834de6fdb20096e2f4f1))
* **query:** add cursor-based pagination for get_attestations_in_range and document deletion-safe workflow ([b8ea318](https://github.com/Haroldwonder/TrustLink/commit/b8ea31888eb42e80c8fee6cf35ab227b8d37177b))
* **react-app:** add error boundaries to prevent full app unmount on panel errors ([1fbec92](https://github.com/Haroldwonder/TrustLink/commit/1fbec92bb5ace1ddc7865d0ac19b93b888c38ca7))
* **react-app:** add skeleton loading states for attestation lists ([18c6d90](https://github.com/Haroldwonder/TrustLink/commit/18c6d90e1d3da016312fd3149af0156c6b78a168))
* remove merge conflict marker from test.rs ([9bb07b1](https://github.com/Haroldwonder/TrustLink/commit/9bb07b1c4c69dd4814df59b767a6ba0faa1583bc))
* remove unused constants and dead code warnings ([540f565](https://github.com/Haroldwonder/TrustLink/commit/540f565e39b7677b827176529d3178d8163469dd))
* resolve 212 compilation errors from duplicate code blocks ([95ceef3](https://github.com/Haroldwonder/TrustLink/commit/95ceef35e5df3eeee66dd537fb539da096af17c4))
* resolve compilation errors in storage, lib, types, and attestation ([196d800](https://github.com/Haroldwonder/TrustLink/commit/196d800ef26ca4598956619c5127309d960b209f))
* resolve issues [#260](https://github.com/Haroldwonder/TrustLink/issues/260), [#327](https://github.com/Haroldwonder/TrustLink/issues/327), [#329](https://github.com/Haroldwonder/TrustLink/issues/329), [#334](https://github.com/Haroldwonder/TrustLink/issues/334) ([0a9151f](https://github.com/Haroldwonder/TrustLink/commit/0a9151f0c1468fb2696122865703c4fb87dc8c56))
* resolve issues [#331](https://github.com/Haroldwonder/TrustLink/issues/331), [#367](https://github.com/Haroldwonder/TrustLink/issues/367), [#368](https://github.com/Haroldwonder/TrustLink/issues/368), [#369](https://github.com/Haroldwonder/TrustLink/issues/369) ([ca88c6b](https://github.com/Haroldwonder/TrustLink/commit/ca88c6b577fcd39695d63610112adae83f3127ce))
* resolve issues [#522](https://github.com/Haroldwonder/TrustLink/issues/522), [#523](https://github.com/Haroldwonder/TrustLink/issues/523), [#524](https://github.com/Haroldwonder/TrustLink/issues/524), [#525](https://github.com/Haroldwonder/TrustLink/issues/525) ([2da81d0](https://github.com/Haroldwonder/TrustLink/commit/2da81d0e12a60c66be68bb8e1ca2708ba9d36717))
* **search:** implement date-range edge cases and fix variable name bug ([12903da](https://github.com/Haroldwonder/TrustLink/commit/12903daf3b0df51083dc6cd7f16fafa03d3f50c7))
* **security:** complete auth-first audit for all public functions ([#432](https://github.com/Haroldwonder/TrustLink/issues/432)) ([2194e7c](https://github.com/Haroldwonder/TrustLink/commit/2194e7ca93ce136c413bb925ec6891d0bc8c9d67)), closes [#270](https://github.com/Haroldwonder/TrustLink/issues/270)
* validate claim_type length and chars in create_attestation ([#278](https://github.com/Haroldwonder/TrustLink/issues/278)) ([9680625](https://github.com/Haroldwonder/TrustLink/commit/9680625488f2491963c24d3c366c19604550ba3a))
* validate fee_token implements token interface in set_fee ([#276](https://github.com/Haroldwonder/TrustLink/issues/276)) ([1c04ab7](https://github.com/Haroldwonder/TrustLink/commit/1c04ab7237bb9651d920cb7b72a3b945838c5fd5))
* wire contract pause/unpause to all write operations ([#301](https://github.com/Haroldwonder/TrustLink/issues/301)) ([23fd0ca](https://github.com/Haroldwonder/TrustLink/commit/23fd0cadae23e54c441d852fb0d3b82baca04809))


### Performance Improvements

* add wasm-opt -Oz to build pipeline and document size reduction ([c819e60](https://github.com/Haroldwonder/TrustLink/commit/c819e60dc3c7849a59bca0b2e9fbe115efd420f7))
* benchmark and document storage cost per attestation ([7b71e90](https://github.com/Haroldwonder/TrustLink/commit/7b71e90171184e83c5189b8367f85b4d2990e123))
* implement chunked index storage for lazy partial index loading ([dd0fa34](https://github.com/Haroldwonder/TrustLink/commit/dd0fa34fe7dc0e6e035ca6c428147f23c3d34fd1))
* optimize batch attestation to write issuer index once per batch ([790a84a](https://github.com/Haroldwonder/TrustLink/commit/790a84a63d0802eb65b0abc2f43784a1eadb4b96))
* verify has_valid_claim short-circuit and add attestation benchmarks ([44cb729](https://github.com/Haroldwonder/TrustLink/commit/44cb72993a467b10a0462100ebb94a77e7c11a20))

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
