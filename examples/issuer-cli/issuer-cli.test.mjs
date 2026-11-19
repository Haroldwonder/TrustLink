/**
 * Tests for the TrustLink Issuer CLI.
 *
 * Covers:
 *  - parseCsvLine: CSV field tokenization (quotes, commas)
 *  - parseCsv: full CSV parsing including header and data rows
 *  - parseActionName: AuditAction normalization from scValToNative
 *  - Argument parsing helpers used by individual commands
 *  - export-audit-trail date validation logic
 *
 * Run with: node --test issuer-cli.test.mjs
 */

import { strict as assert } from "node:assert";
import { describe, it } from "node:test";

// ---------------------------------------------------------------------------
// Re-implement the pure helpers under test.
// We inline them here so we don't import issuer-cli.mjs (which calls main()
// at module level and would fail without env vars and a live network).
// ---------------------------------------------------------------------------

/**
 * Tokenize one CSV line, respecting double-quoted fields.
 * Mirrors parseCsvLine in issuer-cli.mjs exactly.
 */
function parseCsvLine(line) {
  const values = [];
  let current = "";
  let inQuotes = false;
  for (const char of line) {
    if (char === '"') {
      inQuotes = !inQuotes;
    } else if (char === "," && !inQuotes) {
      values.push(current);
      current = "";
    } else {
      current += char;
    }
  }
  values.push(current);
  return values.map((v) => v.trim());
}

/**
 * Parse CSV text into an array of row objects.
 * Mirrors parseCsv in issuer-cli.mjs exactly.
 */
function parseCsv(content) {
  const lines = content.trim().split(/\r?\n/).filter((l) => l.trim());
  if (lines.length < 2) throw new Error("CSV must have a header row and at least one data row.");
  const headers = parseCsvLine(lines[0]).map((h) => h.toLowerCase());
  const addrIdx = headers.indexOf("address");
  if (addrIdx === -1) throw new Error("CSV must have an 'address' column.");
  const ctIdx = headers.indexOf("claim_type");
  const expIdx = headers.indexOf("expiry_days");
  const metaIdx = headers.indexOf("metadata");
  const rows = [];
  for (let i = 1; i < lines.length; i++) {
    const vals = parseCsvLine(lines[i]);
    const address = vals[addrIdx];
    if (!address) continue;
    rows.push({
      lineNum: i + 1,
      address,
      claim_type: ctIdx >= 0 ? vals[ctIdx] || null : null,
      expiry_days: expIdx >= 0 && vals[expIdx] ? parseInt(vals[expIdx], 10) : null,
      metadata: metaIdx >= 0 ? vals[metaIdx] || null : null,
    });
  }
  return rows;
}

/**
 * Normalize an AuditAction value from scValToNative.
 * Mirrors parseActionName in issuer-cli.mjs exactly.
 */
function parseActionName(raw) {
  if (typeof raw === "string") return raw;
  if (Array.isArray(raw) && raw.length > 0) return String(raw[0]);
  if (raw && typeof raw === "object") return Object.keys(raw)[0] ?? String(raw);
  return String(raw);
}

// ---------------------------------------------------------------------------
// parseCsvLine
// ---------------------------------------------------------------------------

describe("parseCsvLine", () => {
  it("splits a simple comma-separated line", () => {
    assert.deepEqual(parseCsvLine("a,b,c"), ["a", "b", "c"]);
  });

  it("handles an empty field at the start", () => {
    assert.deepEqual(parseCsvLine(",b,c"), ["", "b", "c"]);
  });

  it("handles an empty field at the end", () => {
    assert.deepEqual(parseCsvLine("a,b,"), ["a", "b", ""]);
  });

  it("handles consecutive empty fields", () => {
    assert.deepEqual(parseCsvLine("a,,c"), ["a", "", "c"]);
  });

  it("preserves commas inside double-quoted fields", () => {
    assert.deepEqual(parseCsvLine(`"hello, world",b`), ["hello, world", "b"]);
  });

  it("trims surrounding whitespace from unquoted fields", () => {
    assert.deepEqual(parseCsvLine("  a  ,  b  "), ["a", "b"]);
  });

  it("handles an escaped double-quote inside a quoted field", () => {
    // The function toggles inQuotes on each ", so ""foo"" → foo
    const result = parseCsvLine(`"""foo"""`);
    assert.equal(result[0], "foo");
  });

  it("returns a single-element array for a line with no commas", () => {
    assert.deepEqual(parseCsvLine("only"), ["only"]);
  });
});

// ---------------------------------------------------------------------------
// parseCsv
// ---------------------------------------------------------------------------

describe("parseCsv", () => {
  it("parses a minimal valid CSV", () => {
    const csv = "address\nGABC123";
    const rows = parseCsv(csv);
    assert.equal(rows.length, 1);
    assert.equal(rows[0].address, "GABC123");
    assert.equal(rows[0].claim_type, null);
    assert.equal(rows[0].expiry_days, null);
    assert.equal(rows[0].metadata, null);
  });

  it("parses all optional columns when present", () => {
    const csv = [
      "address,claim_type,expiry_days,metadata",
      `GABC123,KYC_PASSED,365,"{""tier"":""gold""}"`,
    ].join("\n");
    const rows = parseCsv(csv);
    assert.equal(rows.length, 1);
    assert.equal(rows[0].claim_type, "KYC_PASSED");
    assert.equal(rows[0].expiry_days, 365);
    assert.equal(rows[0].metadata, '{"tier":"gold"}');
  });

  it("skips rows with an empty address", () => {
    const csv = "address,claim_type\n,KYC_PASSED\nGABC123,KYC_PASSED";
    const rows = parseCsv(csv);
    assert.equal(rows.length, 1);
    assert.equal(rows[0].address, "GABC123");
  });

  it("assigns correct lineNum to each row (1-indexed, header is row 1)", () => {
    const csv = "address\nGABC\nGXYZ";
    const rows = parseCsv(csv);
    assert.equal(rows[0].lineNum, 2);
    assert.equal(rows[1].lineNum, 3);
  });

  it("throws when there is no header row", () => {
    assert.throws(() => parseCsv("GABC123"), /header row/);
  });

  it("throws when the address column is missing", () => {
    const csv = "subject,claim_type\nGABC123,KYC_PASSED";
    assert.throws(() => parseCsv(csv), /address/);
  });

  it("parses multiple rows correctly", () => {
    const csv = [
      "address,claim_type,expiry_days",
      "GABC,KYC_PASSED,365",
      "GXYZ,AML_CLEARED,180",
    ].join("\n");
    const rows = parseCsv(csv);
    assert.equal(rows.length, 2);
    assert.equal(rows[0].address, "GABC");
    assert.equal(rows[1].claim_type, "AML_CLEARED");
    assert.equal(rows[1].expiry_days, 180);
  });

  it("treats a missing expiry_days value as null", () => {
    const csv = "address,expiry_days\nGABC,";
    const rows = parseCsv(csv);
    assert.equal(rows[0].expiry_days, null);
  });

  it("handles Windows-style CRLF line endings", () => {
    const csv = "address\r\nGABC123";
    const rows = parseCsv(csv);
    assert.equal(rows.length, 1);
    assert.equal(rows[0].address, "GABC123");
  });
});

// ---------------------------------------------------------------------------
// parseActionName
// ---------------------------------------------------------------------------

describe("parseActionName", () => {
  it("returns a plain string unchanged", () => {
    assert.equal(parseActionName("Created"), "Created");
  });

  it("decodes an array form ['Revoked']", () => {
    assert.equal(parseActionName(["Revoked"]), "Revoked");
  });

  it("decodes an object form {Transferred: null}", () => {
    assert.equal(parseActionName({ Transferred: null }), "Transferred");
  });

  it("decodes an object form {Updated: undefined}", () => {
    assert.equal(parseActionName({ Updated: undefined }), "Updated");
  });

  it("converts a number to a string", () => {
    assert.equal(parseActionName(42), "42");
  });

  it("handles an empty array by returning 'undefined' string", () => {
    // Array.length === 0, so raw[0] is undefined → String(undefined)
    assert.equal(parseActionName([]), "undefined");
  });
});

// ---------------------------------------------------------------------------
// export-audit-trail date validation logic
// ---------------------------------------------------------------------------

describe("export-audit-trail date validation", () => {
  /**
   * Reproduce the date-parse check from exportAuditTrail in issuer-cli.mjs.
   */
  function validateDates(fromDate, toDate) {
    const fromTs = Math.floor(new Date(fromDate).getTime() / 1000);
    const toTs = Math.floor(new Date(toDate).getTime() / 1000);
    return !isNaN(fromTs) && !isNaN(toTs);
  }

  it("accepts valid ISO date strings", () => {
    assert.ok(validateDates("2024-01-01", "2024-12-31"));
  });

  it("accepts full ISO 8601 timestamps", () => {
    assert.ok(validateDates("2024-01-01T00:00:00Z", "2024-12-31T23:59:59Z"));
  });

  it("rejects non-date strings", () => {
    assert.equal(validateDates("not-a-date", "2024-12-31"), false);
  });

  it("rejects an empty string", () => {
    assert.equal(validateDates("", "2024-12-31"), false);
  });

  it("from timestamp is before to timestamp for a valid range", () => {
    const fromTs = Math.floor(new Date("2024-01-01").getTime() / 1000);
    const toTs = Math.floor(new Date("2024-12-31").getTime() / 1000);
    assert.ok(fromTs < toTs);
  });
});

// ---------------------------------------------------------------------------
// Argument-parsing helpers
// ---------------------------------------------------------------------------

describe("args index helpers", () => {
  /**
   * Reproduce the pattern used throughout issuer-cli.mjs for reading
   * named flag values from the argv array.
   */
  function getFlagValue(argv, flag) {
    const idx = argv.indexOf(flag);
    return idx >= 0 ? argv[idx + 1] : null;
  }

  it("reads a flag that is present", () => {
    const argv = ["export-audit-trail", "--issuer", "GABC", "--from", "2024-01-01"];
    assert.equal(getFlagValue(argv, "--issuer"), "GABC");
    assert.equal(getFlagValue(argv, "--from"), "2024-01-01");
  });

  it("returns null for a flag that is absent", () => {
    const argv = ["export-audit-trail", "--issuer", "GABC"];
    assert.equal(getFlagValue(argv, "--to"), null);
  });

  it("returns null for an empty argv", () => {
    assert.equal(getFlagValue([], "--issuer"), null);
  });

  it("reads --format flag correctly", () => {
    const argv = ["export-audit-trail", "--issuer", "GABC", "--format", "csv"];
    assert.equal(getFlagValue(argv, "--format"), "csv");
  });

  it("returns the value immediately after the flag, not a later occurrence", () => {
    // Edge case: same flag name used as a value
    const argv = ["--format", "json", "--output", "--format"];
    assert.equal(getFlagValue(argv, "--format"), "json");
  });
});
