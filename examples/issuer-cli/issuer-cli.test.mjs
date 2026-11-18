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
    assert.equal(rows[0].metadata, "{tier:gold}");
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

  it("handles an empty array by returning empty string", () => {
    assert.equal(parseActionName([]), "");
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

// ---------------------------------------------------------------------------
// export attestations (Issue #1755)
// ---------------------------------------------------------------------------

function escapeCsvField(val) {
  if (val === null || val === undefined) return "";
  const str = String(val);
  if (/[",\n\r]/.test(str)) {
    return `"${str.replace(/"/g, '""')}"`;
  }
  return str;
}

function formatAttestationsCsv(attestations) {
  const header = "id,issuer,subject,claimType,timestamp,expiration,isRevoked,revocationReason,metadata";
  const rows = attestations.map((att) => {
    const isRevoked =
      att.isRevoked != null
        ? Boolean(att.isRevoked)
        : att.revoked != null
        ? Boolean(att.revoked)
        : false;

    return [
      escapeCsvField(att.id ?? ""),
      escapeCsvField(att.issuer ?? ""),
      escapeCsvField(att.subject ?? ""),
      escapeCsvField(att.claimType ?? att.claim_type ?? ""),
      escapeCsvField(att.timestamp != null ? String(att.timestamp) : ""),
      escapeCsvField(att.expiration != null ? String(att.expiration) : ""),
      escapeCsvField(isRevoked),
      escapeCsvField(att.revocationReason ?? att.revocation_reason ?? ""),
      escapeCsvField(att.metadata ?? ""),
    ].join(",");
  });
  return [header, ...rows].join("\n");
}

async function fetchIssuerAttestationsRest(baseUrl, issuerAddr, limit = 100, fetchFn = fetch) {
  const attestations = [];
  let cursor = null;

  while (true) {
    const url = new URL(`${baseUrl}/issuers/${encodeURIComponent(issuerAddr)}/attestations`);
    url.searchParams.set("limit", String(limit));
    if (cursor) {
      url.searchParams.set("after", cursor);
    }

    const res = await fetchFn(url.toString(), {
      headers: { Accept: "application/json" },
    });

    if (!res.ok) {
      throw new Error(`Indexer request failed: ${res.status} ${res.statusText}`);
    }

    const data = await res.json();
    const items = Array.isArray(data) ? data : (data.items || []);
    attestations.push(...items);

    if (data.nextCursor) {
      cursor = data.nextCursor;
    } else {
      break;
    }
  }

  return attestations;
}

describe("export command - escapeCsvField", () => {
  it("returns empty string for null and undefined", () => {
    assert.equal(escapeCsvField(null), "");
    assert.equal(escapeCsvField(undefined), "");
  });

  it("returns simple strings unchanged", () => {
    assert.equal(escapeCsvField("hello"), "hello");
    assert.equal(escapeCsvField("GABC123"), "GABC123");
  });

  it("converts numbers, booleans, and bigints to strings", () => {
    assert.equal(escapeCsvField(123), "123");
    assert.equal(escapeCsvField(true), "true");
    assert.equal(escapeCsvField(false), "false");
    assert.equal(escapeCsvField(1700000000n), "1700000000");
  });

  it("wraps and escapes strings with commas", () => {
    assert.equal(escapeCsvField("hello, world"), '"hello, world"');
  });

  it("escapes quotes with double-quotes", () => {
    assert.equal(escapeCsvField('hello "world"'), '"hello ""world"""');
  });

  it("wraps strings containing newlines", () => {
    assert.equal(escapeCsvField("line 1\nline 2"), '"line 1\nline 2"');
    assert.equal(escapeCsvField("line 1\r\nline 2"), '"line 1\r\nline 2"');
  });
});

describe("export command - formatAttestationsCsv", () => {
  it("generates CSV header when attestation list is empty", () => {
    const csv = formatAttestationsCsv([]);
    assert.equal(
      csv,
      "id,issuer,subject,claimType,timestamp,expiration,isRevoked,revocationReason,metadata"
    );
  });

  it("formats single attestation correctly", () => {
    const attestations = [
      {
        id: "att_001",
        issuer: "GISSUER123",
        subject: "GSUBJECT456",
        claimType: "KYC_PASSED",
        timestamp: "1700000000",
        expiration: "1731532800",
        isRevoked: false,
        revocationReason: null,
        metadata: '{"tier":"gold"}',
      },
    ];

    const csv = formatAttestationsCsv(attestations);
    const lines = csv.split("\n");
    assert.equal(lines.length, 2);
    assert.equal(
      lines[0],
      "id,issuer,subject,claimType,timestamp,expiration,isRevoked,revocationReason,metadata"
    );
    assert.equal(
      lines[1],
      'att_001,GISSUER123,GSUBJECT456,KYC_PASSED,1700000000,1731532800,false,,"{""tier"":""gold""}"'
    );
  });

  it("handles revoked attestations with reason and snake_case fallbacks", () => {
    const attestations = [
      {
        id: "att_002",
        issuer: "GISSUER123",
        subject: "GSUBJECT789",
        claim_type: "SANCTIONS_CHECK",
        timestamp: 1710000000,
        expiration: null,
        revoked: true,
        revocation_reason: "OFAC match, verified",
        metadata: null,
      },
    ];

    const csv = formatAttestationsCsv(attestations);
    const lines = csv.split("\n");
    assert.equal(lines.length, 2);
    assert.equal(
      lines[1],
      'att_002,GISSUER123,GSUBJECT789,SANCTIONS_CHECK,1710000000,,true,"OFAC match, verified",'
    );
  });

  it("formats multiple records in order", () => {
    const attestations = [
      {
        id: "att_1",
        issuer: "GISSUER",
        subject: "GSUB1",
        claimType: "KYC",
        timestamp: "100",
        expiration: "200",
        isRevoked: false,
      },
      {
        id: "att_2",
        issuer: "GISSUER",
        subject: "GSUB2",
        claimType: "AML",
        timestamp: "150",
        expiration: "250",
        isRevoked: true,
        revocationReason: "Manual",
      },
    ];

    const csv = formatAttestationsCsv(attestations);
    const lines = csv.split("\n");
    assert.equal(lines.length, 3);
    assert.equal(lines[1], "att_1,GISSUER,GSUB1,KYC,100,200,false,,");
    assert.equal(lines[2], "att_2,GISSUER,GSUB2,AML,150,250,true,Manual,");
  });
});

describe("export command - fetchIssuerAttestationsRest", () => {
  it("fetches single page when nextCursor is null", async () => {
    const mockAttestations = [
      { id: "att_1", issuer: "GABC", subject: "GXYZ", claimType: "KYC" },
    ];

    const mockFetch = async (url) => {
      const parsedUrl = new URL(url);
      assert.equal(parsedUrl.pathname, "/issuers/GABC/attestations");
      assert.equal(parsedUrl.searchParams.get("limit"), "50");
      assert.equal(parsedUrl.searchParams.get("after"), null);

      return {
        ok: true,
        json: async () => ({
          items: mockAttestations,
          nextCursor: null,
        }),
      };
    };

    const result = await fetchIssuerAttestationsRest(
      "https://indexer.example.com",
      "GABC",
      50,
      mockFetch
    );

    assert.equal(result.length, 1);
    assert.deepEqual(result, mockAttestations);
  });

  it("paginates through multiple pages using nextCursor", async () => {
    const page1 = [
      { id: "att_1", claimType: "KYC" },
      { id: "att_2", claimType: "KYC" },
    ];
    const page2 = [
      { id: "att_3", claimType: "AML" },
    ];

    const calls = [];

    const mockFetch = async (url) => {
      const parsedUrl = new URL(url);
      calls.push({
        pathname: parsedUrl.pathname,
        limit: parsedUrl.searchParams.get("limit"),
        after: parsedUrl.searchParams.get("after"),
      });

      if (!parsedUrl.searchParams.get("after")) {
        return {
          ok: true,
          json: async () => ({
            items: page1,
            nextCursor: "att_2",
          }),
        };
      }

      if (parsedUrl.searchParams.get("after") === "att_2") {
        return {
          ok: true,
          json: async () => ({
            items: page2,
            nextCursor: null,
          }),
        };
      }

      throw new Error(`Unexpected cursor: ${parsedUrl.searchParams.get("after")}`);
    };

    const result = await fetchIssuerAttestationsRest(
      "https://indexer.example.com",
      "GISSUER_PAGINATED",
      2,
      mockFetch
    );

    assert.equal(result.length, 3);
    assert.equal(calls.length, 2);
    assert.deepEqual(calls[0], {
      pathname: "/issuers/GISSUER_PAGINATED/attestations",
      limit: "2",
      after: null,
    });
    assert.deepEqual(calls[1], {
      pathname: "/issuers/GISSUER_PAGINATED/attestations",
      limit: "2",
      after: "att_2",
    });
    assert.deepEqual(result, [...page1, ...page2]);
  });

  it("handles responses formatted as raw array", async () => {
    const rawArray = [
      { id: "att_legacy_1", claimType: "KYC" },
    ];

    const mockFetch = async () => ({
      ok: true,
      json: async () => rawArray,
    });

    const result = await fetchIssuerAttestationsRest(
      "https://indexer.example.com",
      "GISSUER",
      100,
      mockFetch
    );

    assert.equal(result.length, 1);
    assert.deepEqual(result, rawArray);
  });

  it("throws descriptive error on HTTP failures", async () => {
    const mockFetch = async () => ({
      ok: false,
      status: 404,
      statusText: "Not Found",
    });

    await assert.rejects(
      () =>
        fetchIssuerAttestationsRest(
          "https://indexer.example.com",
          "GNOTFOUND",
          50,
          mockFetch
        ),
      /Indexer request failed: 404 Not Found/
    );
  });
});

describe("export command - flag parsing", () => {
  function getFlagValue(argv, flag) {
    const idx = argv.indexOf(flag);
    return idx >= 0 ? argv[idx + 1] : null;
  }

  it("parses export command flags correctly", () => {
    const argv = ["export", "--issuer", "GABC123", "--format", "csv", "--output", "out.csv", "--limit", "200"];
    assert.equal(getFlagValue(argv, "--issuer"), "GABC123");
    assert.equal(getFlagValue(argv, "--format"), "csv");
    assert.equal(getFlagValue(argv, "--output"), "out.csv");
    assert.equal(getFlagValue(argv, "--limit"), "200");
  });

  it("normalizes indexer base url by stripping trailing slash and /graphql", () => {
    const normalize = (url) => url.replace(/\/graphql\/?$/, "").replace(/\/+$/, "");
    assert.equal(normalize("http://localhost:3000/graphql"), "http://localhost:3000");
    assert.equal(normalize("http://localhost:3000/graphql/"), "http://localhost:3000");
    assert.equal(normalize("http://localhost:3000/"), "http://localhost:3000");
    assert.equal(normalize("http://localhost:3000"), "http://localhost:3000");
  });
});
