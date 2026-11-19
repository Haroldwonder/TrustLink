#!/usr/bin/env node
/**
 * scripts/check-error-codes.mjs
 *
 * CI guard: verifies that every generated error-code artifact is in sync with
 * src/errors.rs. Exits 1 if any artifact is stale or missing.
 *
 * Run:
 *   node scripts/check-error-codes.mjs
 *
 * Typical CI usage:
 *   - run: node scripts/check-error-codes.mjs
 */

import { readFileSync, existsSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

// ── Paths ────────────────────────────────────────────────────────────────────

const ERRORS_RS = resolve(ROOT, "src", "errors.rs");
const JSON_ARTIFACT = resolve(ROOT, "error-codes.json");
const TS_SDK_GEN = resolve(ROOT, "sdk", "typescript", "src", "generated", "error-codes.ts");
const TS_BINDINGS_GEN = resolve(ROOT, "bindings", "typescript", "src", "generated", "error-codes.ts");
const PY_GEN = resolve(ROOT, "bindings", "python", "trustlink", "generated_error_codes.py");

// ── Parse src/errors.rs ──────────────────────────────────────────────────────

function parseErrorsRs() {
  const src = readFileSync(ERRORS_RS, "utf8");

  const start = src.indexOf("pub enum Error");
  if (start === -1) throw new Error("Could not find `pub enum Error` in src/errors.rs");

  let braceDepth = 0;
  let bodyStart = -1;
  let bodyEnd = -1;

  for (let i = start; i < src.length; i++) {
    if (src[i] === "{") {
      if (braceDepth === 0) bodyStart = i + 1;
      braceDepth++;
    } else if (src[i] === "}") {
      braceDepth--;
      if (braceDepth === 0) { bodyEnd = i; break; }
    }
  }

  const body = src.slice(bodyStart, bodyEnd);
  /** @type {Map<number, string>} */
  const errors = new Map();

  for (const rawLine of body.split("\n")) {
    const line = rawLine.trim();
    const m = line.match(/^([A-Za-z][A-Za-z0-9]*)\s*=\s*(\d+),?$/);
    if (m) errors.set(parseInt(m[2], 10), m[1]);
  }

  return errors;
}

// ── Parse generated artifacts ────────────────────────────────────────────────

/** Parse error-codes.json → Map<code, name> */
function parseJson() {
  const data = JSON.parse(readFileSync(JSON_ARTIFACT, "utf8"));
  return new Map(data.errors.map((e) => [e.code, e.name]));
}

/**
 * Parse a generated TS file.
 * Looks for lines like:   123: "ErrorName",
 * Returns Map<code, name>
 */
function parseGeneratedTs(path) {
  const src = readFileSync(path, "utf8");
  const errors = new Map();
  for (const line of src.split("\n")) {
    const m = line.trim().match(/^(\d+):\s*"([^"]+)",/);
    if (m) errors.set(parseInt(m[1], 10), m[2]);
  }
  return errors;
}

/**
 * Parse a generated Python file.
 * Looks for lines like:   123: "ErrorName",
 * Returns Map<code, name>
 */
function parseGeneratedPy(path) {
  const src = readFileSync(path, "utf8");
  const errors = new Map();
  for (const line of src.split("\n")) {
    const m = line.trim().match(/^(\d+):\s*"([^"]+)",/);
    if (m) errors.set(parseInt(m[1], 10), m[2]);
  }
  return errors;
}

// ── Comparison ───────────────────────────────────────────────────────────────

/**
 * Returns an array of human-readable diff lines.
 * Empty array means the maps are identical.
 */
function diff(canonical, actual, label) {
  const issues = [];

  for (const [code, name] of canonical) {
    if (!actual.has(code)) {
      issues.push(`  ${label}: missing code #${code} (${name})`);
    } else if (actual.get(code) !== name) {
      issues.push(`  ${label}: code #${code} is "${actual.get(code)}", expected "${name}"`);
    }
  }

  for (const [code, name] of actual) {
    if (!canonical.has(code)) {
      issues.push(`  ${label}: unexpected extra code #${code} (${name}) not in src/errors.rs`);
    }
  }

  return issues;
}

// ── Main ─────────────────────────────────────────────────────────────────────

const canonical = parseErrorsRs();
console.log(`Parsed ${canonical.size} error codes from src/errors.rs`);

const checks = [
  { label: "error-codes.json", path: JSON_ARTIFACT, parse: parseJson },
  { label: "sdk/typescript/src/generated/error-codes.ts", path: TS_SDK_GEN, parse: parseGeneratedTs },
  { label: "bindings/typescript/src/generated/error-codes.ts", path: TS_BINDINGS_GEN, parse: parseGeneratedTs },
  { label: "bindings/python/trustlink/generated_error_codes.py", path: PY_GEN, parse: parseGeneratedPy },
];

let failed = false;

for (const check of checks) {
  if (!existsSync(check.path)) {
    console.error(`❌ ${check.label}: file not found — run 'node scripts/generate-error-codes.mjs'`);
    failed = true;
    continue;
  }

  const actual = check.parse(check.path);
  const issues = diff(canonical, actual, check.label);

  if (issues.length === 0) {
    console.log(`✅ ${check.label} — up to date`);
  } else {
    console.error(`❌ ${check.label} — out of sync:`);
    for (const issue of issues) console.error(issue);
    failed = true;
  }
}

if (failed) {
  console.error(
    "\n⛔ One or more generated artifacts are out of date.\n" +
    "   Run `node scripts/generate-error-codes.mjs` (or `make generate`) and commit the results."
  );
  process.exit(1);
} else {
  console.log("\n✨ All error-code artifacts are in sync with src/errors.rs.");
}
