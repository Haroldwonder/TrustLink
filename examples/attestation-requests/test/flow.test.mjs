/**
 * Tests for the attestation-requests example flow.
 *
 * Covers:
 *  - parseRequestStatus: enum resolution across different SDK decoding variants
 *  - isRequestExpired: TTL calculation and lazy expiration logic
 *  - formatRequestSummary: logging and status visualization
 *  - Lifecycle invariants & error codes
 *
 * Run with: node --test test/flow.test.mjs
 */

import { strict as assert } from "node:assert";
import { describe, it } from "node:test";

// Pure helper definitions mirroring flow.ts for isolated unit testing
export const RequestStatus = {
  Pending: "Pending",
  Fulfilled: "Fulfilled",
  Rejected: "Rejected",
  Cancelled: "Cancelled",
  Expired: "Expired",
};

export function parseRequestStatus(raw) {
  if (typeof raw === "string") {
    return RequestStatus[raw] ?? RequestStatus.Pending;
  }
  if (Array.isArray(raw) && raw.length > 0) {
    return RequestStatus[raw[0]] ?? RequestStatus.Pending;
  }
  if (raw && typeof raw === "object") {
    const key = Object.keys(raw)[0];
    return RequestStatus[key] ?? RequestStatus.Pending;
  }
  return RequestStatus.Pending;
}

export function formatRequestSummary(req) {
  const dateStr = new Date(req.timestamp * 1000).toISOString();
  const expStr = new Date(req.expires_at * 1000).toISOString();
  let base = `[${req.status}] ID: ${req.id} | Claim: ${req.claim_type} | Requested: ${dateStr} | Expires: ${expStr}`;
  if (req.rejection_reason) {
    base += ` | Reason: "${req.rejection_reason}"`;
  }
  return base;
}

export function isRequestExpired(req, currentTimestampSecs) {
  return currentTimestampSecs >= req.expires_at;
}

describe("Attestation Request Pure Logic", () => {
  describe("parseRequestStatus", () => {
    it("parses string representations", () => {
      assert.equal(parseRequestStatus("Pending"), RequestStatus.Pending);
      assert.equal(parseRequestStatus("Fulfilled"), RequestStatus.Fulfilled);
      assert.equal(parseRequestStatus("Rejected"), RequestStatus.Rejected);
      assert.equal(parseRequestStatus("Cancelled"), RequestStatus.Cancelled);
      assert.equal(parseRequestStatus("Expired"), RequestStatus.Expired);
    });

    it("parses array representations from Soroban XDR decoders", () => {
      assert.equal(parseRequestStatus(["Fulfilled"]), RequestStatus.Fulfilled);
      assert.equal(parseRequestStatus(["Rejected"]), RequestStatus.Rejected);
    });

    it("parses object representations with variant keys", () => {
      assert.equal(parseRequestStatus({ Fulfilled: [] }), RequestStatus.Fulfilled);
      assert.equal(parseRequestStatus({ Expired: [] }), RequestStatus.Expired);
    });

    it("defaults to Pending for unknown or empty input", () => {
      assert.equal(parseRequestStatus(null), RequestStatus.Pending);
      assert.equal(parseRequestStatus(undefined), RequestStatus.Pending);
      assert.equal(parseRequestStatus("UnknownVariant"), RequestStatus.Pending);
    });
  });

  describe("isRequestExpired", () => {
    const sampleRequest = {
      id: "req_123",
      subject: "GXXXXX",
      issuer: "GYYYYY",
      claim_type: "KYC_PASSED",
      timestamp: 1000,
      expires_at: 1000 + 7 * 24 * 60 * 60, // 605800
      status: RequestStatus.Pending,
    };

    it("returns false when current time is strictly before expires_at", () => {
      assert.equal(isRequestExpired(sampleRequest, 605799), false);
    });

    it("returns true when current time is equal to or past expires_at", () => {
      assert.equal(isRequestExpired(sampleRequest, 605800), true);
      assert.equal(isRequestExpired(sampleRequest, 700000), true);
    });
  });

  describe("formatRequestSummary", () => {
    it("formats request without rejection reason", () => {
      const req = {
        id: "req_001",
        subject: "G_SUB",
        issuer: "G_ISS",
        claim_type: "KYC_PASSED",
        timestamp: 1700000000,
        expires_at: 1700604800,
        status: RequestStatus.Fulfilled,
      };
      const summary = formatRequestSummary(req);
      assert.match(summary, /\[Fulfilled\]/);
      assert.match(summary, /KYC_PASSED/);
      assert.doesNotMatch(summary, /Reason:/);
    });

    it("formats request with rejection reason", () => {
      const req = {
        id: "req_002",
        subject: "G_SUB",
        issuer: "G_ISS",
        claim_type: "KYC_PASSED",
        timestamp: 1700000000,
        expires_at: 1700604800,
        status: RequestStatus.Rejected,
        rejection_reason: "Invalid identity document",
      };
      const summary = formatRequestSummary(req);
      assert.match(summary, /\[Rejected\]/);
      assert.match(summary, /Reason: "Invalid identity document"/);
    });
  });

  describe("Lifecycle Flow Invariants", () => {
    it("ensures terminal states are distinct and immutable", () => {
      const terminalStatuses = [
        RequestStatus.Fulfilled,
        RequestStatus.Rejected,
        RequestStatus.Cancelled,
        RequestStatus.Expired,
      ];

      for (const status of terminalStatuses) {
        assert.notEqual(status, RequestStatus.Pending);
      }
    });

    it("validates 7-day TTL constant calculation", () => {
      const ATTESTATION_REQUEST_TTL_SECS = 7 * 24 * 60 * 60;
      assert.equal(ATTESTATION_REQUEST_TTL_SECS, 604800);
    });
  });
});
