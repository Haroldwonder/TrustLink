# TrustLink Bug Bounty Program

**Program Status:** Active  
**Last Updated:** June 27, 2026  
**Mainnet Launch:** Forthcoming

---

## Overview

TrustLink is a decentralized attestation and verification system on the Stellar blockchain. We take security seriously and welcome responsible vulnerability disclosures from the security research community.

This document outlines the scope, severity tiers, and reward structure for TrustLink's bug bounty program. All accepted reports are handled under our [Vulnerability Disclosure Policy](../SECURITY.md).

---

## Scope

### In Scope

Vulnerabilities affecting the following are eligible for rewards:

#### Smart Contract

- **Repository:** TrustLink core contract (`src/`)
- **Language:** Rust (Soroban SDK v21.0.0)
- **Mainnet Contract ID:** (Published on launch)
- **Testnet Contract ID:** CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB5C

**Components:**

- Authorization and access control (`src/validation.rs`)
- Attestation creation, revocation, and queries (`src/attestation.rs`, `src/query.rs`)
- Storage and data integrity (`src/storage.rs`)
- Multi-sig proposals (`src/multisig.rs`)
- Admin functions and state management (`src/admin.rs`)
- Event emission (`src/events.rs`)
- External contract interactions (expiration hooks, fee transfers)

#### TypeScript SDK

- **Repository:** `bindings/typescript/`
- **Package:** `@trustlink/sdk`
- **Severity:** Only critical vulnerabilities affecting signature validation, transaction construction, or private key handling are in scope

#### Python SDK

- **Repository:** `bindings/python/`
- **Package:** `trustlink-sdk`
- **Severity:** Only critical vulnerabilities affecting signature validation, transaction construction, or private key handling are in scope

#### Off-Chain Indexer

- **Repository:** `indexer/` directory
- **Severity:** Only critical vulnerabilities affecting data integrity or enabling unauthorized state changes are in scope

### Out of Scope

The following are **not** eligible for bounty rewards:

- **Soroban Runtime & SDK:** Vulnerabilities in the Soroban SDK itself (report to Stellar Foundation)
- **Stellar Network:** Network consensus, validator behavior (report to Stellar Foundation)
- **Infrastructure:** Docker, Kubernetes, deployment automation
- **Third-Party Dependencies:** Vulnerabilities in external crates (report upstream)
- **Documentation:** Grammar, clarity, or factual accuracy issues
- **Example Contracts:** Reference implementations only (`examples/`)
- **Social Engineering:** Phishing, credential compromise
- **Denial of Service (Network-level):** Attacks requiring network-level resources
- **Front-end Applications:** React apps, web UIs (low-impact UI bugs only)

---

## Severity Tiers & Rewards

Rewards are determined by **CVSS v3.1 severity score** and impact on TrustLink's security model.

| Severity     | CVSS Score | Description                                                                                                              | Reward           |
| ------------ | ---------- | ------------------------------------------------------------------------------------------------------------------------ | ---------------- |
| **Critical** | 9.0–10.0   | Uncontrolled access to funds, loss of authorization enforcement, network-wide consensus failure, or mass user asset loss | $50,000–$100,000 |
| **High**     | 7.0–8.9    | Partial loss of access control, state manipulation, or denial of service affecting core functionality                    | $10,000–$50,000  |
| **Medium**   | 4.0–6.9    | Information disclosure, privilege escalation via specific user action, or localized denial of service                    | $1,000–$10,000   |
| **Low**      | 0.1–3.9    | Minor bugs, edge cases, or informational findings with minimal or no impact                                              | $100–$1,000      |

### Reward Adjustments

Rewards may be adjusted based on:

- **Proof of Concept:** Working PoC increases reward by up to 20%
- **Report Quality:** Clear steps to reproduce, comprehensive analysis
- **Responsible Disclosure:** Early report pre-launch vs. post-mainnet
- **Duplicate Reports:** First reporter receives full reward; duplicates receive 10–50% depending on novelty
- **Claimed vs. Unclaimed:** Reports claiming previously unknown vulnerabilities increase reward

### Examples

| Vulnerability                                                    | CVSS | Reward Range     |
| ---------------------------------------------------------------- | ---- | ---------------- |
| Admin key can be stolen via signature replay                     | 9.8  | $75,000–$100,000 |
| Issuer can create attestations after being de-registered         | 7.5  | $15,000–$30,000  |
| Attestation revocation can be bypassed under specific conditions | 6.8  | $3,000–$8,000    |
| Event data is inconsistent with stored attestation               | 3.2  | $200–$500        |

---

## Submission Process

### 1. Report Submission

**Email:** security@trustlink.io  
**Response Time:** 48 hours (Monday–Friday)

**Include in your report:**

- Vulnerability title and type (e.g., "Authorization Bypass in create_attestation")
- Description of the vulnerability
- Affected component(s) and version(s)
- Steps to reproduce (with code if possible)
- Proof of Concept (PoC) contract or transaction, if applicable
- Potential impact and attack scenario
- Suggested remediation (optional)
- Your name or handle (for public credit, if approved)

**Optional:**

- Affected contract address(es)
- Estimated CVSS score
- References to similar CVEs or public disclosures

### 2. Triage & Assessment

Our security team will:

1. **Acknowledge receipt** — within 24 hours
2. **Assess validity** — confirm the vulnerability and reproducibility
3. **Determine severity** — assign CVSS score and impact tier
4. **Propose reward** — based on severity and scope
5. **Agree on timeline** — work with you on disclosure

### 3. Remediation & Disclosure

- **Fix Development:** Our team develops and tests a fix
- **Disclosure Embargo:** 90 days for critical, 60 days for high, 30 days for medium/low (can be shortened with prior agreement)
- **Public Disclosure:** Coordinated post-fix release with credit to researcher
- **Patch Release:** Announced on GitHub and in project communications

### 4. Reward Payment

- **Currency:** USD (via wire transfer, ACH, or cryptocurrency)
- **Timing:** Within 30 days of fix verification and public disclosure
- **Tax:** You are responsible for any tax obligations

---

## Safe Harbor & Legal

### Protection from Liability

When conducting authorized security research under this bug bounty program, you will not be held liable for:

- Accessing the contract's public functions
- Creating read-only queries to test authorization
- Deploying test transactions on testnet
- Submitting vulnerability reports in good faith

### Expected Conduct

To remain eligible for rewards:

- **Do not** exploit vulnerabilities for personal gain or unauthorized access
- **Do not** publicly disclose the vulnerability without prior agreement
- **Do not** share the vulnerability with third parties before disclosure
- **Do not** access, modify, or delete other users' data or funds
- **Do not** conduct denial-of-service (DoS) attacks on production systems
- **Do not** violate applicable laws or regulations

Violations may disqualify you from future bounty eligibility and result in legal action.

### No Guarantee

TrustLink reserves the right to:

- Decline or defer rewards for out-of-scope vulnerabilities
- Reduce rewards for duplicate reports
- Disqualify researchers who violate safe harbor conduct
- Modify the program scope, tiers, or rewards with 30 days' notice

---

## Previous Audits & Known Findings

### Security Audit Status

- **Pre-Mainnet Audit:** In Progress (scheduled completion by Q3 2026)
- **Audit Firm:** OpenZeppelin (or equivalent tier)
- **Audit Scope:** [AUDIT_SCOPE.md](./AUDIT_SCOPE.md)

### Known Limitations

Certain design constraints are **intentional** and are not eligible for bounty rewards:

1. **Admin Key Compromise** — Admin is a single address (no multisig on-chain). Mitigation is operational (hardware wallet or multisig account).
2. **Imported Attestations** — Admin can import fabricated attestations. Integrators distinguish via `imported` flag.
3. **Bridge Contract Trust** — Registered bridges have broad authority. Vetting before registration is essential.
4. **No Subject Consent** — Subjects cannot reject attestations about them. This is by design for permissionless flows.
5. **Storage TTL Expiry** — Persistent storage has a 30-day TTL and can be evicted. Regular interaction required.

See [docs/security.md — Known Limitations](./security.md#known-limitations) for full details.

### Pre-Audit Findings

Three findings were identified in early security review:

| Finding     | Severity | Status      | Details                                               |
| ----------- | -------- | ----------- | ----------------------------------------------------- |
| FINDING-001 | MEDIUM   | ✅ Resolved | `initialize()` state read before auth                 |
| FINDING-002 | HIGH     | ✅ Resolved | `revoke_attestation()` missing `require_issuer` check |
| FINDING-003 | HIGH     | ✅ Resolved | `update_expiration()` missing `require_issuer` check  |

See [docs/security-review.md](./security-review.md) for details.

---

## Frequently Asked Questions

### How are duplicates handled?

If multiple researchers report the same vulnerability:

- **First report:** Receives full reward amount
- **Subsequent reports:** Receive 10–50% of reward, depending on uniqueness of attack vector or evidence

We encourage coordinated disclosure if you're aware of prior reports.

### Can I test on mainnet?

**Testnet only.** Do not test vulnerabilities on mainnet without explicit permission. Testing should be limited to:

- Public `query` functions
- Deploying test transactions to testnet
- Creating attestations with test data

**Never:**

- Transfer real funds without authorization
- Modify or delete other users' attestations
- Conduct operations intended to cause financial loss

### What if I find a vulnerability after mainnet launch?

Post-launch findings are still eligible for rewards. The reward may be adjusted based on:

- Whether funds were at risk
- Whether the vulnerability was exploited before being reported
- The time to patch after reporting

### Can I disclose before receiving payment?

No. Disclosure must be coordinated and embargoed until:

1. A fix is developed and tested
2. A patch release is prepared
3. Affected users have time to upgrade

Early public disclosure may disqualify you from the bounty.

### What if I disagree with the severity assessment?

We're open to discussion. If you believe your report was under-scored:

1. **Respond to triage email** with your CVSS justification
2. **Include additional evidence** or attack scenarios
3. Our security lead will re-evaluate within 5 business days

### Can I negotiate rewards?

Reward amounts are based on severity and impact using the tiers above. We may increase rewards for:

- Exceptional clarity and quality of report
- High-impact proof of concept
- Pre-launch vs. post-launch timing

We do not negotiate downward.

---

## Trust Boundaries & High-Impact Areas

Researchers are encouraged to focus on these trust boundaries for maximum impact:

### Admin Boundary

**Trust Assumption:** Admin is trusted and uses secure key management.

**Attack Surface:**

- Can the admin's authority be bypassed or impersonated?
- Can administrative functions be called without proper auth?
- Can the admin transfer their own privileges without authorization?

### Issuer Boundary

**Trust Assumption:** Registered issuers are vetted before registration.

**Attack Surface:**

- Can an unregistered issuer create attestations?
- Can an issuer modify or revoke attestations they did not create?
- Can an issuer revoke their own de-registration?

### Multi-Sig Boundary

**Trust Assumption:** M-of-N co-signers enforce consensus for high-value claims.

**Attack Surface:**

- Can a single issuer finalize a multi-sig proposal without threshold approval?
- Can the threshold be modified after proposal creation?
- Can signatures be forged or replayed?

### Bridge Boundary

**Trust Assumption:** Bridge contracts only relay valid cross-chain attestations.

**Attack Surface:**

- Can a bridge create attestations for unauthorized claim types?
- Can a bridge forge `source_chain` or `source_tx` values?
- Can the bridge be unregistered and re-registered to bypass safeguards?

### Fee Boundary

**Trust Assumption:** Fee collection does not introduce reentrancy or theft.

**Attack Surface:**

- Can fee transfers be exploited via reentrancy?
- Can the fee collector be changed without admin authorization?
- Can token balance checks be bypassed?

---

## Contact & Resources

| Item                  | Contact                                                 |
| --------------------- | ------------------------------------------------------- |
| **Security Reports**  | security@trustlink.io                                   |
| **General Inquiries** | hello@trustlink.io                                      |
| **GitHub Issues**     | https://github.com/afurious/TrustLink/issues            |
| **Documentation**     | https://github.com/afurious/TrustLink/tree/main/docs    |
| **Testnet Contract**  | CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB5C |

---

## Acknowledgments

We are grateful to the security researchers and community members who help us keep TrustLink secure. Researchers whose vulnerabilities are accepted will be publicly credited in our [Security Hall of Fame](#hall-of-fame) (unless they prefer anonymity).

### Hall of Fame

Coming soon after first accepted reports.

---

**Last Updated:** June 27, 2026  
**Next Review:** December 27, 2026
