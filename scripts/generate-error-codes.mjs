#!/usr/bin/env node
/**
 * scripts/generate-error-codes.mjs
 *
 * Parses src/errors.rs and emits:
 *   1. error-codes.json  — canonical, language-agnostic artifact (project root)
 *   2. sdk/typescript/src/generated/error-codes.ts  — consumed by the TS SDK
 *   3. bindings/typescript/src/generated/error-codes.ts — consumed by TS bindings
 *   4. bindings/python/trustlink/generated_error_codes.py — consumed by Python SDK
 *   5. bindings/rust/src/generated/error_codes.rs — consumed by the Rust bindings
 *
 * Usage:
 *   node scripts/generate-error-codes.mjs
 *
 * All generated files contain a DO-NOT-EDIT header and are committed to the
 * repository so that consumers don't need Node.js at install time.
 * Run this script (or `make generate`) whenever src/errors.rs changes.
 */

import { readFileSync, writeFileSync, mkdirSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

const ERRORS_RS = resolve(ROOT, "src", "errors.rs");
const JSON_OUT = resolve(ROOT, "error-codes.json");
const TS_SDK_OUT = resolve(ROOT, "sdk", "typescript", "src", "generated", "error-codes.ts");
const TS_BINDINGS_OUT = resolve(ROOT, "bindings", "typescript", "src", "generated", "error-codes.ts");
const PY_OUT = resolve(ROOT, "bindings", "python", "trustlink", "generated_error_codes.py");
const RS_OUT = resolve(ROOT, "bindings", "rust", "src", "generated", "error_codes.rs");

// ---------------------------------------------------------------------------
// Parse src/errors.rs
// ---------------------------------------------------------------------------

const src = readFileSync(ERRORS_RS, "utf8");

function extractEnumBody(source) {
  const start = source.indexOf("pub enum Error");
  if (start === -1) throw new Error("Could not find `pub enum Error` in src/errors.rs");

  let braceDepth = 0;
  let bodyStart = -1;
  let bodyEnd = -1;

  for (let i = start; i < source.length; i++) {
    if (source[i] === "{") {
      if (braceDepth === 0) bodyStart = i + 1;
      braceDepth++;
    } else if (source[i] === "}") {
      braceDepth--;
      if (braceDepth === 0) {
        bodyEnd = i;
        break;
      }
    }
  }

  if (bodyStart === -1 || bodyEnd === -1) {
    throw new Error("Could not extract enum body from src/errors.rs");
  }

  return source.slice(bodyStart, bodyEnd);
}

function parseEnumVariants(body) {
  const errors = [];
  const lines = body.split("\n");
  let pendingDoc = [];

  for (const rawLine of lines) {
    const line = rawLine.trim();

    if (line.startsWith("///")) {
      pendingDoc.push(line.slice(3).trim());
      continue;
    }

    const variantMatch = line.match(/^([A-Za-z][A-Za-z0-9]*)\s*=\s*(\d+),?$/);
    if (variantMatch) {
      const name = variantMatch[1];
      const code = parseInt(variantMatch[2], 10);
      const description = pendingDoc.length > 0 ? pendingDoc.join(" ") : null;
      errors.push({ code, name, description });
      pendingDoc = [];
      continue;
    }

    if (line && !line.startsWith("//")) {
      pendingDoc = [];
    }
  }

  errors.sort((a, b) => a.code - b.code);
  return errors;
}

// ---------------------------------------------------------------------------
// Emitters
// ---------------------------------------------------------------------------

function emitJson(errors) {
  const output = {
    generated: new Date().toISOString(),
    source: "src/errors.rs",
    errors,
  };
  writeFileSync(JSON_OUT, JSON.stringify(output, null, 2) + "\n");
  console.log(`✅ Wrote error-codes.json (${errors.length} errors)`);
}

const TS_HEADER = `/**
 * GENERATED FILE — DO NOT EDIT BY HAND.
 * Run \`node scripts/generate-error-codes.mjs\` (or \`make generate\`) to regenerate.
 * Source of truth: src/errors.rs
 */\n\n`;

function emitTypeScript(errors, outPath) {
  mkdirSync(dirname(outPath), { recursive: true });

  const recordEntries = errors
    .map((e) => {
      const desc = e.description ? ` // ${e.description}` : "";
      return `  ${e.code}: "${e.name}",${desc}`;
    })
    .join("\n");

  const content =
    TS_HEADER +
    `/** Map of contract error code → error name, generated from src/errors.rs. */\n` +
    `export const ERROR_CODES: Readonly<Record<number, string>> = {\n${recordEntries}\n} as const;\n`;

  writeFileSync(outPath, content);
  console.log(`✅ Wrote ${outPath}`);
}

function emitPython(errors, outPath) {
  const lines = [
    `"""`,
    `GENERATED FILE — DO NOT EDIT BY HAND.`,
    `Run \`node scripts/generate-error-codes.mjs\` (or \`make generate\`) to regenerate.`,
    `Source of truth: src/errors.rs`,
    `"""`,
    ``,
    `from typing import Dict`,
    ``,
    `# Map of contract error code -> error name, generated from src/errors.rs.`,
    `CONTRACT_ERRORS: Dict[int, str] = {`,
    ...errors.map((e) => {
      const comment = e.description ? `  # ${e.description}` : "";
      return `    ${e.code}: "${e.name}",${comment}`;
    }),
    `}`,
    ``,
  ];
  writeFileSync(outPath, lines.join("\n"));
  console.log(`✅ Wrote ${outPath}`);
}

function emitRust(errors, outPath) {
  mkdirSync(dirname(outPath), { recursive: true });

  const variants = errors
    .map((e) => {
      const doc = e.description ? `    /// ${e.description}\n` : "";
      return `${doc}    ${e.name} = ${e.code},`;
    })
    .join("\n");

  const fromArms = errors.map((e) => `            ${e.code} => Self::${e.name},`).join("\n");

  const content = `//! GENERATED FILE — DO NOT EDIT BY HAND.
//!
//! Run \`node scripts/generate-error-codes.mjs\` (or \`make generate\`) to regenerate.
//! Source of truth: src/errors.rs

use serde::{Deserialize, Serialize};

/// Contract-level error codes that map directly to the on-chain \`Error\` enum.
///
/// \`Unknown\` is not a contract error: it is the fallback for a code this
/// binding does not recognise, which happens when the contract adds a variant
/// before the bindings are regenerated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum ContractErrorCode {
${variants}
    /// Unrecognised contract error code.
    Unknown = 99,
}

impl From<u32> for ContractErrorCode {
    fn from(code: u32) -> Self {
        match code {
${fromArms}
            _ => Self::Unknown,
        }
    }
}

impl ContractErrorCode {
    /// The numeric code as reported by the contract.
    pub fn code(self) -> u32 {
        self as u32
    }

    /// The variant name, matching the contract\'s \`Error\` enum.
    pub fn name(self) -> &\'static str {
        match self {
${errors.map((e) => `            Self::${e.name} => "${e.name}",`).join("\n")}
            Self::Unknown => "Unknown",
        }
    }
}

impl core::fmt::Display for ContractErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<\'_>) -> core::fmt::Result {
        write!(f, "{} (#{})", self.name(), self.code())
    }
}
`;

  writeFileSync(outPath, content);
  console.log(`✅ Wrote ${outPath}`);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const enumBody = extractEnumBody(src);
const errors = parseEnumVariants(enumBody);

if (errors.length === 0) {
  throw new Error("No error variants found — check the parser.");
}

emitJson(errors);
emitTypeScript(errors, TS_SDK_OUT);
emitTypeScript(errors, TS_BINDINGS_OUT);
emitPython(errors, PY_OUT);
emitRust(errors, RS_OUT);

console.log(`\n✨ Done. ${errors.length} error codes synchronized across all clients.`);
