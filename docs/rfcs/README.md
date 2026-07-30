# Requests for Comments (RFCs)

RFCs are how TrustLink discusses significant changes *before* they are built,
as opposed to [ADRs](../adr/README.md), which record a decision *after* it
has been made (or is about to be implemented).

Use an RFC when a change is large enough to benefit from community input
before code is written — for example: new storage layouts, changes to the
public contract interface, new attestation workflows, or anything with
backward-compatibility implications. Small or purely internal changes don't
need one; open a PR directly.

## Process

1. **Copy the template**: duplicate [TEMPLATE.md](TEMPLATE.md) to
   `docs/rfcs/NNNN-short-title.md`, where `NNNN` is the next unused number.
2. **Open a PR** adding the file with status `Draft`. This starts the
   discussion — reviewers and the community comment inline on the proposal.
3. **Iterate** based on feedback until the proposal reaches consensus or is
   withdrawn.
4. **Resolve** the RFC by merging the PR with its status updated to
   `Accepted`, `Rejected`, or `Withdrawn`.

## Relationship to ADRs

An RFC and an ADR capture different moments in the same decision:

- The **RFC** is the proposal — written before implementation, open for
  discussion, and may go through multiple revisions before consensus.
- The **ADR** is the record — written once the decision is final, capturing
  what was decided and why, for future readers who won't see the discussion.

An accepted RFC should link to the ADR that formalizes it once implementation
lands, and the ADR should link back to the RFC that proposed it. Not every
RFC needs a corresponding ADR (e.g., a rejected or superseded RFC), and not
every ADR needs a preceding RFC (smaller decisions can go straight to an
ADR).
