# TrustLink Project Governance

This document describes how the **TrustLink open-source project** is governed:
who maintains it, how decisions are made, and how new maintainers are added.

It covers project governance only. On-chain admin/multi-sig policy for the
deployed TrustLink contract is separate and is documented in the contract ADRs
and deployment runbooks.

## Maintainers

Maintainers are trusted stewards of the repository. They are responsible for
keeping TrustLink healthy, secure, and usable for integrators.

### Roles

| Role | Responsibilities |
| --- | --- |
| **Core maintainer** | Merge rights on the default branch; final call on breaking changes, releases, and security disclosures; keeps CI and release automation green. |
| **Area maintainer** | Owns a subsystem (contract, indexer, SDKs, docs/security) and reviews PRs in that area; may merge within their CODEOWNERS paths after review. |
| **Triager** | Labels issues/PRs, routes work, closes duplicates, and keeps the backlog actionable. Does not merge by default. |

Current ownership routing lives in [`.github/CODEOWNERS`](.github/CODEOWNERS).
Teams listed there are the starting point for review assignment:

- Contract (`/src/`) — `@Haroldwonder/contract-team`
- SDKs (`/sdk/`) — `@Haroldwonder/sdk-team`
- Security docs — `@Haroldwonder/security-team`
- Compliance docs — `@Haroldwonder/compliance-team`

### Responsibilities of all maintainers

- Review PRs in a timely manner and request changes when quality, security, or
  clarity is insufficient.
- Uphold the contribution standards in [`CONTRIBUTING.md`](CONTRIBUTING.md) and
  the security process in [`SECURITY.md`](SECURITY.md).
- Prefer reversible, well-tested changes; require tests for bug fixes that
  exercise the failing path.
- Never merge their own unreviewed changes to security-sensitive contract or
  indexer paths without a second maintainer approval.
- Keep secrets, keys, and production credentials out of the repository.

## Decision-making

TrustLink uses a lightweight consensus model with written design records for
non-trivial changes.

### Day-to-day changes

Routine work (bug fixes, docs, small features, dependency bumps) is decided
through pull request review:

1. Author opens a PR with a clear summary and test plan.
2. Required CODEOWNERS / CI checks pass.
3. At least one maintainer with ownership of the affected path approves.
4. The approving maintainer (or core maintainer) merges.

Disagreement on a routine PR is resolved by discussion on the PR. If consensus
is not reached within a few business days, a core maintainer decides and records
the rationale on the PR.

### Substantial changes (RFC / ADR)

Changes that alter public APIs, storage layout, trust assumptions, indexer
semantics, or release policy require a written proposal **before**
implementation lands on the default branch.

Use the Architecture Decision Record process under [`docs/adr/`](docs/adr/):

1. Open an issue or draft PR describing the problem, options, and recommendation
   (start from [`docs/adr/ADR-000-template.md`](docs/adr/ADR-000-template.md)).
2. Label it for design review and notify the relevant area maintainers.
3. Allow a comment period of at least **7 days** for non-urgent proposals
   (shorter only for active security incidents).
4. A core maintainer accepts, rejects, or requests revision. Accepted decisions
   are committed as a numbered ADR under `docs/adr/`.
5. Implementation PRs link back to the ADR.

Security-sensitive decisions follow [`SECURITY.md`](SECURITY.md) disclosure
timelines and may be discussed privately until a fix is released.

### Releases

Release cadence, versioning, and automation are documented in
[`RELEASE.md`](RELEASE.md). Core maintainers own tagging and publishing
artifacts; area maintainers are responsible for release notes in their domains.

## Adding and removing maintainers

### Adding a maintainer

Candidates are usually active contributors who have demonstrated judgment
through sustained, high-quality participation (reviews, fixes, design input).

Process:

1. An existing core maintainer nominates the candidate in a private maintainer
   channel or via email, summarizing contributions and the proposed role/area.
2. Other core maintainers consent (lazy consensus: no sustained objection within
   **7 days**). Area-maintainer additions also need consent from an existing
   maintainer of that area when one exists.
3. The nominee accepts and confirms they can uphold this governance document and
   the security policy.
4. Access is granted (GitHub team membership / CODEOWNERS update as needed) and
   announced in a public issue or discussion.

### Stepping down

Maintainers may step down at any time by notifying the other core maintainers.
Access is removed promptly; past contributions remain credited in git history.

### Emeritus and inactivity

Maintainers who are unreachable for **90 days** without notice may be moved to
emeritus status (no merge rights) after an attempt to contact them. Emeritus
maintainers can be restored by the same process used for new maintainers.

### Removal for cause

Core maintainers may revoke maintainer status for repeated violation of project
standards, security negligence, or harmful conduct. Removal requires agreement
of at least two core maintainers (or a majority when more than two exist) and a
written internal record of the reason.

## Scope boundaries

| In scope of this document | Out of scope |
| --- | --- |
| Repository permissions, review policy, release stewardship | On-chain admin keys and multi-sig thresholds for a given deployment |
| ADR / RFC process for the open-source project | Network-specific operational runbooks (see `docs/mainnet-*.md`) |
| How CODEOWNERS teams map to review | Customer / deployment-specific governance contracts under `examples/` |

## Related documents

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — how to contribute code and docs
- [`SECURITY.md`](SECURITY.md) — vulnerability reporting and disclosure
- [`docs/adr/`](docs/adr/) — architecture decision records
- [`RELEASE.md`](RELEASE.md) — release process
- [`.github/CODEOWNERS`](.github/CODEOWNERS) — review routing
