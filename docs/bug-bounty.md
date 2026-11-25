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
- **Testnet Contract ID:** CAK7PYYSWWQH6ML3ZPO4OB2EIONODOEESE3MIV3YGFDMHEU4EUOBUJQN

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

**Primary channel — GitHub Private Vulnerability Reporting:**  
Submit reports through GitHub's private advisory channel at  
https://github.com/TrustLink-Org/trustlink/security/advisories/new  
(or use the **"Report a vulnerability"** button on the repository's **Security** tab).

This is the only supported private reporting channel. Reports submitted here are visible only to repository maintainers and the reporter.

> **Note:** The previously listed `security@trustlink.io` address is no longer in service and must not be used. If the private advisory channel is unavailable, follow the fallback instructions in [SECURITY.md](../SECURITY.md) — open a minimal issue titled `Security contact request` **without any vulnerability details** so a maintainer can enable private reporting and follow up directly.

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

1. **Acknowledge receipt** — within 48 hours
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
- **Audit Scope:** [AUDIT_SCOPE.md](./AUDIT_SCOPE.

/* … truncated 5992 chars — edit only what you need near the top … */
