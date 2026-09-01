# Security Policy

## Supported Versions

The following versions of TrustLink currently receive security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

> **Note (updated 2026-09-01):** Two previously listed channels are currently
> non-functional. GitHub's private vulnerability reporting ("Report a
> vulnerability" / Security Advisories) is **not yet enabled** for this
> repository — the advisories page returns a 404. The email address
> **security@trustlink.io** is also **not operational** (mailbox not found).
> Until these are restored, use the fallback path described below. We
> apologise for the inconvenience and are working to restore both channels.

### Preferred channel (temporarily unavailable — see fallback below)

TrustLink intends to support GitHub's [private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability). Once enabled, use the **"Report a vulnerability"** button on the [Security Advisories](../../security/advisories/new) page, or email **security@trustlink.io**.

### Fallback channel (use this now)

Open a **new GitHub Discussion** in the
[Security category](https://github.com/Haroldwonder/TrustLink/discussions/new?category=security)
with the subject line `[SECURITY] <brief one-line summary>` and **do not
include vulnerability details in the body**. A maintainer will respond within
48 hours with a private channel (direct message or encrypted email) to receive
the full report. If no Security discussion category exists, open a regular
Discussion and mark it with the `security` label — the maintainers monitor
those.

### What to include

- A clear description of the vulnerability
- Steps to reproduce the issue
- The potential impact (e.g., unauthorized access, fund loss, data exposure)
- The affected version(s) and contract function(s)
- A suggested fix or mitigation, if you have one

### Disclosure process

1. **Submit** your report via private advisory, email, or (currently) the fallback Discussion channel described above.
2. **Acknowledgement** — you will receive a confirmation within **48 hours**.
3. **Triage** — the team evaluates severity using the CVSS scoring framework within **5 business days**.
4. **Remediation** — patches for `HIGH` and `CRITICAL` severity findings are targeted for release within **30 days** of confirmation. Lower-severity issues are addressed in the next scheduled release.
5. **Disclosure** — a public security advisory is published after the patch is released. Reporters are credited (with consent).

### Severity response targets

| Severity | Acknowledgement | Patch Target |
| -------- | --------------- | ------------ |
| CRITICAL | 48 hours        | 30 days      |
| HIGH     | 48 hours        | 30 days      |
| MEDIUM   | 48 hours        | Next release |
| LOW      | 48 hours        | Next release |

## Scope

The following are **in scope** for vulnerability reports:

- The TrustLink Soroban smart contract (`src/`)
- Authorization logic (admin, issuer, bridge, multi-sig flows)
- Storage key collisions or data corruption
- Fee bypass or manipulation
- Attestation forgery or unauthorized revocation
- TypeScript and Python SDK bindings that could expose integrators to exploits

The following are **out of scope**:

- Denial-of-service attacks that rely on abnormally high ledger fees
- Social engineering or phishing
- Vulnerabilities in third-party dependencies (report those upstream)
- Issues in example code under `examples/` that do not affect the core contract

## Contact

- **Email:** security@trustlink.io *(currently non-operational — use the fallback channel above)*
- **GitHub Private Advisory:** [Submit here](../../security/advisories/new) *(not yet enabled — use the fallback channel above)*
- **Fallback:** [Open a Security Discussion](https://github.com/Haroldwonder/TrustLink/discussions/new?category=security) — a maintainer will provide a private channel within 48 hours

For general questions that are not security-sensitive, open a [GitHub Discussion](../../discussions) or a regular issue.
