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
#!/usr/bin/env node
/**
 * SDK-ABI Consistency Checker
 *
 * Compares the contract ABI/spec with SDK method signatures to catch
 * drift between the contract interface and what SDKs expose.
 *
 * Usage: node scripts/check-sdk-abi.mjs [--wasm <path>] [--sdk-dir <path>]
 *
 * Options:
 *   --wasm <path>    Path to the compiled contract WASM (default: ./target/wasm32-unknown-unknown/release/trustlink.wasm)
 *   --sdk-dir <path> Path to SDK directory to check (default: ./sdk/typescript)
 *
 * Exit codes:
 *   0 - All checks passed
 *   1 - Missing functions or arity mismatches in SDKs
 *   2 - Failed to build contract or extract ABI
 */

import { parseArgs } from "util";
import { readFileSync, readdirSync, existsSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

const DEFAULT_WASM_PATH = join(ROOT, "target/wasm32-unknown-unknown/release/trustlink.wasm");
const DEFAULT_ALLOWLIST_PATH = join(__dirname, "sdk-abi-allowlist.json");

/**
 * Load the explicit allowlist of contract functions that are intentionally
 * not wrapped by an SDK. The file is a JSON object of the shape:
 *   { "typescript": ["fn_a", "fn_b"], "python": ["fn_c"] }
 * A top-level "*" key applies to every SDK.
 */
function loadAllowlist(allowlistPath) {
  const allowlist = { "*": new Set() };
  if (!allowlistPath || !existsSync(allowlistPath)) {
    return allowlist;
  }

  let parsed;
  try {
    parsed = JSON.parse(readFileSync(allowlistPath, "utf-8"));
  } catch (err) {
    console.error(`ERROR: Failed to parse allowlist ${allowlistPath}: ${err.message}`);
    process.exit(2);
  }

  for (const [sdk, names] of Object.entries(parsed)) {
    if (!Array.isArray(names)) {
      console.error(`ERROR: Allowlist entry "${sdk}" must be an array of function names`);
      process.exit(2);
    }
    allowlist[sdk] = new Set(names);
  }

  return allowlist;
}

function isAllowlisted(allowlist, sdkName, contractMethod) {
  return (
    allowlist["*"]?.has(contractMethod) ||
    allowlist[sdkName]?.has(contractMethod) ||
    false
  );
}

/**
 * Extract the full list of exported contract function names from the spec.
 * Handles both the `stellar contract info interface` JSON shape and the
 * raw WASM contractspec shape.
 */
function extractContractFunctionNames(spec) {
  const names = new Set();

  const addFrom = (entries) => {
    if (!Array.isArray(entries)) return;
    for (const entry of entries) {
      if (!entry) continue;
      if (typeof entry === "string") {
        names.add(entry);
      } else if (typeof entry.name === "string") {
        names.add(entry.name);
      }
    }
  };

  // stellar contract info interface shape
  addFrom(spec.functions);
  addFrom(spec.methods);
  // Some CLI versions nest the interface under `spec`
  if (spec.spec) {
    addFrom(spec.spec.functions);
    addFrom(spec.spec.methods);
  }

  return names;
}

/**
 * Extract contract spec from WASM using stellar CLI or fallback to parsed WASM
 */
async function getContractSpec(wasmPath) {
  // Try using stellar CLI first
  const { execSync } = await import("child_process");

  // Check if stellar CLI is available
  let spec;
  try {
    const output = execSync(
      `stellar contract info interface --wasm ${wasmPath} --output json`,
      { encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] }
    );
    spec = JSON.parse(output);
    return spec;
  } catch (err) {
    console.warn("stellar CLI not available, attempting to parse WASM directly");
  }

  // Fallback: minimal WASM parsing to extract contract functions
  // This is a simplified approach - in production you'd use wasm-parser
  try {
    const wasmBuffer = readFileSync(wasmPath);
    return parseWasmMinimal(wasmBuffer);
  } catch (err) {
    console.error("Failed to extract contract spec:", err.message);
    process.exit(2);
  }
}

/**
 * Minimal WASM parsing to extract function names
 * This is a workaround when stellar CLI is not available
 */
function parseWasmMinimal(buffer) {
  // Very minimal WASM section parsing - look for custom "contractspec" section
  // which Soroban contracts embed
  const sections = [];
  let offset = 8; // Skip WASM header

  while (offset < buffer.length) {
    const id = buffer[offset++];
    const size = buffer.readUInt32LE(offset);
    offset += 4;

    if (id === 0) {
      // Custom section
      const nameLen = buffer.readUInt32LE(offset);
      offset += 4;
      const name = buffer.slice(offset, offset + nameLen).toString("utf-8");
      offset += nameLen;

      if (name === "contractspecv0") {
        // Found the contract spec!
        const specData = buffer.slice(offset, offset + size - nameLen - 4);
        try {
          // Try to parse as JSON
          return JSON.parse(specData.toString("utf-8"));
        } catch {
          // Try to find JSON within the section
          const str = specData.toString("utf-8");
          const jsonMatch = str.match(/\{[\s\S]*\}/);
          if (jsonMatch) {
            return JSON.parse(jsonMatch[0]);
          }
        }
      }
    }

    offset += size;
  }

  throw new Error("Could not find contractspecv0 section in WASM");
}

/**
 * Extract function signatures from TypeScript SDK client
 */
function extractTypeScriptFunctions(sdkPath) {
  const clientPath = join(sdkPath, "src/client.ts");
  if (!existsSync(clientPath)) {
    return { functions: [], errors: [`Client file not found: ${clientPath}`] };
  }

  const content = readFileSync(clientPath, "utf-8");
  const functions = [];

  // Match async method declarations that call this.simulate or this.simulateForSigning
  // Pattern: async methodName(...): Promise<...> { ... this.simulate("method_name", ... args) ... }
  const methodRegex = /async\s+(\w+)\s*\([^)]*\)\s*:\s*Promise<[^>]+>\s*\{[\s\S]*?this\.simulate\s*\(\s*["']([^"']+)["']/g;

  let match;
  while ((match = methodRegex.exec(content)) !== null) {
    const tsMethodName = match[1];
    const contractMethodName = match[2];

    // Extract parameter count from the method signature
    const sigMatch = content.slice(match.index).match(/async\s+\w+\s*\(([^)]*)\)/);
    const params = sigMatch ? sigMatch[1].trim() : "";
    const paramCount = params ? params.split(",").filter(p => p.trim()).length : 0;

    functions.push({
      tsMethod: tsMethodName,
      contractMethod: contractMethodName,
      paramCount,
      location: `client.ts:${content.slice(0, match.index).split("\n").length}`,
    });
  }

  return { functions, errors: [] };
}

/**
 * Extract function signatures from Python SDK client
 */
function extractPythonFunctions(sdkPath) {
  const clientPath = join(sdkPath, "trustlink/client.py");
  const asyncClientPath = join(sdkPath, "trustlink/async_client.py");

  const functions = [];
  const errors = [];

  for (const path of [clientPath, asyncClientPath]) {
    if (!existsSync(path)) continue;

    const content = readFileSync(path, "utf-8");

    // Match method definitions that call self._invoke or self._call
    // Pattern: async def method_name(...): or def method_name(...)
    const methodRegex = /(?:async\s+)?def\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*[^:]+)?:\s*[\s\S]*?(?:self\._invoke|self\._call)\s*\(\s*["']([^"']+)["']/g;

    let match;
    while ((match = methodRegex.exec(content)) !== null) {
      const pyMethodName = match[1];
      const params = match[2];
      const contractMethodName = match[3];

      const paramCount = params
        .replace(/self/, "")
        .replace(/db[^,]*/, "")
        .split(",")
        .filter(p => p.trim() && p.trim() !== "self").length;

      functions.push({
        pyMethod: pyMethodName,
        contractMethod: contractMethodName,
        paramCount,
        location: `${path.split("/").pop()}:${content.slice(0, match.index).split("\n").length}`,
      });
    }
  }

  return { functions, errors };
}

/**
 * Extract function signatures from Rust SDK client
 */
function extractRustFunctions(sdkPath) {
  const libPath = join(sdkPath, "src/lib.rs");
  if (!existsSync(libPath)) {
    return { functions: [], errors: [] };
  }

  const content = readFileSync(libPath, "utf-8");
  const functions = [];

  // Match pub async fn method_name and extract contract method from client.call
  // Pattern: pub async fn method_name(...) -> ... { ... .invoke(..., "method_name", ...)
  const methodRegex = /pub\s+(?:async\s+)?fn\s+(\w+)\s*<[^>]*>\s*\(([^)]*)\)\s*(?:->\s*[^{]+)?\{[\s\S]*?(?:\.invoke|\.simulate)\s*\(\s*[^,]*,\s*["']([^"']+)["']/g;

  let match;
  while ((match = methodRegex.exec(content)) !== null) {
    const rustMethodName = match[1];
    const params = match[2];
    const contractMethodName = match[3];

    // Count parameters (excluding self, &self, client, etc.)
    const paramCount = params
      .split(",")
      .filter(p => {
        const trimmed = p.trim();
        return trimmed && !trimmed.startsWith("&") && !trimmed.startsWith("self");
      }).length;

    functions.push({
      rustMethod: rustMethodName,
      contractMethod: contractMethodName,
      paramCount,
      location: `lib.rs:${content.slice(0, match.index).split("\n").length}`,
    });
  }

  return { functions, errors: [] };
}

/**
 * Compare the full set of contract entry points against the set of contract
 * methods actually wrapped by an SDK. Any contract function that is neither
 * wrapped nor explicitly allowlisted is reported as a missing wrapper.
 */
function checkCoverage(contractFunctionNames, sdkFunctions, sdkName, allowlist) {
  const wrapped = new Set(sdkFunctions.map((fn) => fn.contractMethod));
  const missing = [];

  for (const contractMethod of contractFunctionNames) {
    if (wrapped.has(contractMethod)) continue;
    if (isAllowlisted(allowlist, sdkName, contractMethod)) continue;
    missing.push(contractMethod);
  }

  return missing;
}

/**
 * Main comparison logic
 */
function checkSDK(spec, sdkFunctions, sdkName) {
  const errors = [];
  const warnings = [];

  // Build a map of contract methods
  const contractMethods = new Map();
  if (spec.functions) {
    for (const fn of spec.functions) {
      contractMethods.set(fn.name, fn.inputs?.length || 0);
    }
  } else if (spec.methods) {
    for (const fn of spec.methods) {
      contractMethods.set(fn.name, fn.params?.length || 0);
    }
  }

  // Check each SDK function against contract
  const sdkMethods = new Set();
  for (const fn of sdkFunctions) {
    const contractMethod = fn.contractMethod;
    sdkMethods.add(contractMethod);

    if (!contractMethods.has(contractMethod)) {
      errors.push({
        type: "unknown_function",
        sdk: sdkName,
        method: fn.tsMethod || fn.pyMethod || fn.rustMethod,
        contractMethod,
        location: fn.location,
        message: `SDK calls non-existent contract method: ${contractMethod}`,
      });
      continue;
    }

    const expectedParams = contractMethods.get(contractMethod);
    if (fn.paramCount !== expectedParams) {
      errors.push({
        type: "arity_mismatch",
        sdk: sdkName,
        method: fn.tsMethod || fn.pyMethod || fn.rustMethod,
        contractMethod,
        expected: expectedParams,
        actual: fn.paramCount,
        location: fn.location,
        message: `Parameter count mismatch for ${contractMethod}: expected ${expectedParams}, got ${fn.paramCount}`,
      });
    }
  }

  // Report contract functions not exposed by SDK (warning only)
  for (const [contractMethod, paramCount] of contractMethods) {
    if (!sdkMethods.has(contractMethod)) {
      warnings.push({
        type: "not_exposed",
        sdk: sdkName,
        contractMethod,
        paramCount,
        message: `Contract method ${contractMethod} not exposed by ${sdkName} SDK`,
      });
    }
  }

  return { errors, warnings };
}

/**
 * Print results in CI-friendly format
 */
function printResults(results) {
  const allErrors = results.flatMap(r => r.errors);
  const allWarnings = results.flatMap(r => r.warnings);

  if (allErrors.length > 0) {
    console.error("\n=== ERRORS ===\n");
    for (const err of allErrors) {
      console.error(`[${err.sdk}] ${err.message}`);
      console.error(`  Location: ${err.location}`);
      if (err.expected !== undefined) {
        console.error(`  Expected ${err.expected} params, got ${err.actual}`);
      }
      console.error("");
    }
  }

  if (allWarnings.length > 0) {
    console.log("\n=== WARNINGS (informational) ===\n");
    for (const warn of allWarnings) {
      console.log(`[${warn.sdk}] ${warn.message}`);
    }
    console.log("");
  }

  console.log(`Summary: ${allErrors.length} error(s), ${allWarnings.length} warning(s)`);

  return allErrors.length === 0;
}

/**
 * Main entry point
 */
async function main() {
  const { values } = parseArgs({
    options: {
      wasm: {
        type: "string",
        default: DEFAULT_WASM_PATH,
      },
      "sdk-dir": {
        type: "string",
        default: join(ROOT, "sdk/typescript"),
      },
      allowlist: {
        type: "string",
        default: DEFAULT_ALLOWLIST_PATH,
      },
      help: {
        type: "boolean",
        default: false,
      },
    },
  });

  if (values.help) {
    console.log(`
SDK-ABI Consistency Checker

Compares contract ABI with SDK method signatures to catch drift.

Usage: node scripts/check-sdk-abi.mjs [options]

Options:
  --wasm <path>      Path to WASM file (default: target/wasm32-unknown-unknown/release/trustlink.wasm)
  --sdk-dir <path>   SDK directory to check (default: sdk/typescript)
  --allowlist <path> Allowlist of intentionally unwrapped functions (default: scripts/sdk-abi-allowlist.json)
  --help             Show this help

Exit codes:
  0 - All checks passed
  1 - Errors found (missing wrappers or arity mismatches)
  2 - Failed to extract contract spec
`);
    return;
  }

  const wasmPath = values.wasm;
  const sdkDir = values.sdkDir;
  const allowlist = loadAllowlist(values.allowlist);

  console.log(`Checking SDK-ABI consistency...`);
  console.log(`WASM: ${wasmPath}`);
  console.log(`SDK: ${sdkDir}`);

  // Check if WASM exists
  if (!existsSync(wasmPath)) {
    console.error(`\nERROR: WASM file not found: ${wasmPath}`);
    console.error("Run 'cargo build --release' first to build the contract.");
    process.exit(2);
  }

  // Get contract spec
  let spec;
  try {
    spec = await getContractSpec(wasmPath);
  } catch (err) {
    console.error(`\nERROR: Failed to extract contract spec: ${err.message}`);
    process.exit(2);
  }

  if (!spec.functions && !spec.methods) {
    console.error("\nERROR: Could not find functions in contract spec");
    console.error("Spec content:", JSON.stringify(spec).slice(0, 500));
    process.exit(2);
  }

  const contractFunctionNames = extractContractFunctionNames(spec);
  const functionCount = contractFunctionNames.size;
  console.log(`\nContract has ${functionCount} function(s)`);

  // Check each SDK
  const results = [];
  const coverageFailures = [];

  // TypeScript SDK
  if (existsSync(join(sdkDir, "src/client.ts"))) {
    console.log("\nChecking TypeScript SDK...");
    const { functions, errors } = extractTypeScriptFunctions(sdkDir);
    if (errors.length > 0) {
      console.error("Errors extracting TypeScript functions:", errors);
    }
    const checkResult = checkSDK(spec, functions, "typescript");
    results.push(checkResult);
    const missing = checkCoverage(contractFunctionNames, functions, "typescript", allowlist);
    if (missing.length > 0) {
      coverageFailures.push({ sdk: "typescript", missing });
    }
    console.log(`  Found ${functions.length} method(s), ${checkResult.errors.length} error(s)`);
  }

  // Python SDK
  const pythonSdkPath = join(ROOT, "bindings/python");
  if (existsSync(join(pythonSdkPath, "trustlink/client.py"))) {
    console.log("\nChecking Python SDK...");
    const { functions, errors } = extractPythonFunctions(pythonSdkPath);
    if (errors.length > 0) {
      console.error("Errors extracting Python functions:", errors);
    }
    const checkResult = checkSDK(spec, functions, "python");
    results.push(checkResult);
    const missing = checkCoverage(contractFunctionNames, functions, "python", allowlist);
    if (missing.length > 0) {
      coverageFailures.push({ sdk: "python", missing });
    }
    console.log(`  Found ${functions.length} method(s), ${checkResult.errors.length} error(s)`);
  }

  // Rust SDK
  const rustSdkPath = join(ROOT, "bindings/rust");
  if (existsSync(join(rustSdkPath, "src/lib.rs"))) {
    console.log("\nChecking Rust SDK...");
    const { functions, errors } = extractRustFunctions(rustSdkPath);
    if (errors.length > 0) {
      console.error("Errors extracting Rust functions:", errors);
    }
    const checkResult = checkSDK(spec, functions, "rust");
    results.push(checkResult);
    console.log(`  Found ${functions.length} method(s), ${checkResult.errors.length} error(s)`);
  }

  // Print results and exit with appropriate code
  const success = printResults(results);

  if (coverageFailures.length > 0) {
    console.error("\n=== MISSING SDK WRAPPERS ===\n");
    for (const { sdk, missing } of coverageFailures) {
      for (const contractMethod of missing) {
        console.error(`[${sdk}] Contract entry point "${contractMethod}" has no SDK wrapper`);
      }
      console.error("");
    }
    console.error(
      "Add the missing wrappers, or list intentionally unwrapped functions in the allowlist file."
    );
  }

  if (!success || coverageFailures.length > 0) {
    console.error("\nFAILED: SDK-ABI mismatch detected");
    process.exit(1);
  }

  console.log("\nOK: All SDK methods match contract ABI and every entry point is wrapped");
  process.exit(0);
}

main().catch(err => {
  console.error("Unexpected error:", err);
  process.exit(2);
});