/**
 * WCAG 2.1 AA Accessibility Tests
 *
 * Runs axe-core against each panel to catch critical/serious violations:
 * missing labels, insufficient contrast, missing focus indicators,
 * and absent ARIA landmarks.
 *
 * These tests act as a regression guard — any future change that
 * introduces an axe violation will fail the suite.
 *
 * To run:
 *   npm test
 *   # or for a single file:
 *   npx vitest run src/test/accessibility.test.tsx
 */

import { render, waitFor } from "@testing-library/react";
import { axe } from "vitest-axe";
import { vi, describe, it, expect, beforeEach } from "vitest";

// ── Mock the contract module for all panels ──────────────────────────────────
vi.mock("../contract", () => ({
  registerIssuer: vi.fn(),
  removeIssuer: vi.fn(),
  isIssuer: vi.fn().mockResolvedValue(false),
  getConfig: vi.fn().mockResolvedValue({
    ttl_config: { ttl_days: 30 },
    limits: { max_attestations_per_issuer: 10000, max_attestations_per_subject: 100 },
    fee_config: { attestation_fee: 0n, fee_collector: "GAAA...", fee_token: null },
  }),
  getGlobalStats: vi.fn().mockResolvedValue({
    total_attestations: 0n,
    total_revocations: 0n,
    total_issuers: 0n,
  }),
  createAttestation: vi.fn(),
  revokeAttestation: vi.fn(),
  getSubjectAttestations: vi.fn().mockResolvedValue([]),
  getIssuerStats: vi.fn().mockResolvedValue({ total_issued: 0, active: 0, revoked: 0, expired: 0 }),
  getIssuerAttestations: vi.fn().mockResolvedValue([]),
  getExpiringAttestations: vi.fn().mockResolvedValue([]),
  hasValidClaim: vi.fn().mockResolvedValue(false),
  proposeAttestation: vi.fn(),
  cosignAttestation: vi.fn(),
  getMultiSigProposal: vi.fn(),
  submitAttestationRequest: vi.fn(),
  getSubjectRequests: vi.fn().mockResolvedValue([]),
  getIssuerRequests: vi.fn().mockResolvedValue([]),
  fulfillRequest: vi.fn(),
  rejectRequest: vi.fn(),
  cancelRequest: vi.fn(),
}));

// ── Mock useGlobalStats hook ──────────────────────────────────────────────────
vi.mock("../hooks/useGlobalStats", () => ({
  useGlobalStats: () => ({
    data: { total_attestations: 5n, total_revocations: 1n, total_issuers: 2n },
    loading: false,
    error: null,
  }),
}));

// ── Import panels after mocks ─────────────────────────────────────────────────
import AdminPanel from "../panels/AdminPanel";
import IssuerPanel from "../panels/IssuerPanel";
import UserPanel from "../panels/UserPanel";
import VerifierPanel from "../panels/VerifierPanel";
import MultiSigPanel from "../panels/MultiSigPanel";
import AttestationRequestPanel from "../panels/AttestationRequestPanel";

const ADDRESS = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";

// ── Axe configuration: WCAG 2.1 AA ───────────────────────────────────────────
const axeOptions = {
  runOnly: {
    type: "tag" as const,
    values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"],
  },
};

// ── AdminPanel ────────────────────────────────────────────────────────────────
describe("AdminPanel — WCAG 2.1 AA", () => {
  it("has no critical or serious axe violations", async () => {
    const { container } = render(<AdminPanel address={ADDRESS} />);
    // Wait for async state (stats / config) to settle
    await waitFor(() => {}, { timeout: 500 });
    const results = await axe(container, axeOptions);
    expect(results).toHaveNoViolations();
  });

  it("all form inputs have associated labels", () => {
    const { container } = render(<AdminPanel address={ADDRESS} />);
    const inputs = container.querySelectorAll("input");
    inputs.forEach((input) => {
      const id = input.getAttribute("id");
      const ariaLabel = input.getAttribute("aria-label");
      const ariaLabelledBy = input.getAttribute("aria-labelledby");
      const hasLabel = id
        ? container.querySelector(`label[for="${id}"]`) !== null
        : false;
      expect(hasLabel || ariaLabel || ariaLabelledBy).toBeTruthy();
    });
  });

  it("status alerts carry role=alert", () => {
    const { container } = render(<AdminPanel address={ADDRESS} />);
    // If an alert is rendered it must have role="alert"
    const alerts = container.querySelectorAll(".alert");
    alerts.forEach((el) => {
      expect(el.getAttribute("role")).toBe("alert");
    });
  });
});

// ── IssuerPanel ───────────────────────────────────────────────────────────────
describe("IssuerPanel — WCAG 2.1 AA", () => {
  it("has no critical or serious axe violations on dashboard tab", async () => {
    const { container } = render(<IssuerPanel address={ADDRESS} />);
    await waitFor(() => {}, { timeout: 500 });
    const results = await axe(container, axeOptions);
    expect(results).toHaveNoViolations();
  });

  it("tab list has correct ARIA roles", () => {
    const { container } = render(<IssuerPanel address={ADDRESS} />);
    const tabList = container.querySelector("[role='tablist']");
    expect(tabList).not.toBeNull();

    const tabs = tabList!.querySelectorAll("[role='tab']");
    expect(tabs.length).toBeGreaterThan(0);

    tabs.forEach((tab) => {
      expect(tab.hasAttribute("aria-selected")).toBe(true);
    });
  });
});

// ── UserPanel ─────────────────────────────────────────────────────────────────
describe("UserPanel — WCAG 2.1 AA", () => {
  it("has no critical or serious axe violations", async () => {
    const { container } = render(<UserPanel address={ADDRESS} />);
    await waitFor(() => {}, { timeout: 500 });
    const results = await axe(container, axeOptions);
    expect(results).toHaveNoViolations();
  });
});

// ── VerifierPanel ─────────────────────────────────────────────────────────────
describe("VerifierPanel — WCAG 2.1 AA", () => {
  it("has no critical or serious axe violations", async () => {
    const { container } = render(<VerifierPanel />);
    const results = await axe(container, axeOptions);
    expect(results).toHaveNoViolations();
  });

  it("all form inputs have associated labels", () => {
    const { container } = render(<VerifierPanel />);
    const inputs = container.querySelectorAll("input");
    inputs.forEach((input) => {
      const id = input.getAttribute("id");
      const ariaLabel = input.getAttribute("aria-label");
      const hasLabel = id
        ? container.querySelector(`label[for="${id}"]`) !== null
        : false;
      expect(hasLabel || ariaLabel).toBeTruthy();
    });
  });
});

// ── MultiSigPanel ─────────────────────────────────────────────────────────────
describe("MultiSigPanel — WCAG 2.1 AA", () => {
  beforeEach(() => {
    // isIssuer returns false by default — panel shows only "Co-Sign" tab
  });

  it("has no critical or serious axe violations (non-issuer view)", async () => {
    const { container } = render(<MultiSigPanel address={ADDRESS} />);
    await waitFor(() => {}, { timeout: 500 });
    const results = await axe(container, axeOptions);
    expect(results).toHaveNoViolations();
  });

  it("proposal ID input is accessible when shown", () => {
    const { container } = render(<MultiSigPanel address={ADDRESS} />);
    const input = container.querySelector("#ms-proposal-id");
    if (input) {
      const ariaLabel = input.getAttribute("aria-label");
      const id = input.getAttribute("id");
      const label = id ? container.querySelector(`label[for="${id}"]`) : null;
      expect(ariaLabel || label).toBeTruthy();
    }
  });
});

// ── AttestationRequestPanel ───────────────────────────────────────────────────
describe("AttestationRequestPanel — WCAG 2.1 AA", () => {
  it("has no critical or serious axe violations on submit tab", async () => {
    const { container } = render(<AttestationRequestPanel address={ADDRESS} />);
    await waitFor(() => {}, { timeout: 500 });
    const results = await axe(container, axeOptions);
    expect(results).toHaveNoViolations();
  });

  it("tab list has correct ARIA roles", () => {
    const { container } = render(<AttestationRequestPanel address={ADDRESS} />);
    const tabList = container.querySelector("[role='tablist']");
    expect(tabList).not.toBeNull();

    const tabs = tabList!.querySelectorAll("[role='tab']");
    expect(tabs.length).toBeGreaterThan(0);

    tabs.forEach((tab) => {
      expect(tab.hasAttribute("aria-selected")).toBe(true);
    });
  });

  it("submit form inputs are labelled", () => {
    const { container } = render(<AttestationRequestPanel address={ADDRESS} />);
    container.querySelectorAll("input").forEach((input) => {
      const id = input.getAttribute("id");
      const ariaLabel = input.getAttribute("aria-label");
      const hasLabel = id
        ? container.querySelector(`label[for="${id}"]`) !== null
        : false;
      expect(hasLabel || ariaLabel).toBeTruthy();
    });
  });
});
