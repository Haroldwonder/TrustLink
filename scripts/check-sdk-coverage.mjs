#!/usr/bin/env node
/**
 * SDK Contract Coverage Checker
 *
 * Enforces that every public contract entrypoint has a corresponding wrapper
 * in sdk/typescript and bindings/python, or is explicitly listed in the allowlist.
 *
 * Usage:
 *   node scripts/check-sdk-coverage.mjs [options]
 *
 * Options:
 *   --wasm <path>         Path to built contract WASM (optional, tries stellar CLI)
 *   --contract-src <path> Path to contract source (default: ./src/lib.rs)
 *   --allowlist <path>    Path to allowlist JSON (default: ./scripts/sdk-coverage-allowlist.json)
 *   --strict              Fail with exit code 1 if unallowed missing wrappers exist (default: true)
 *   --help                Display this help message
 *
 * Exit codes:
 *   0 - All checks passed
 *   1 - Missing wrappers found without allowlist entry
 *   2 - Failed to parse contract entrypoints or invalid configuration
 */

import { parseArgs } from "util";
import { readFileSync, existsSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import { execSync } from "child_process";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

const DEFAULT_WASM_PATH = join(ROOT, "target/wasm32-unknown-unknown/release/trustlink.wasm");
const DEFAULT_CONTRACT_SRC = join(ROOT, "src/lib.rs");
const DEFAULT_ALLOWLIST_PATH = join(ROOT, "scripts/sdk-coverage-allowlist.json");

/**
 * Extract public contract entrypoints.
 * First tries `stellar contract info interface` / `stellar contract inspect` if WASM exists,
 * otherwise parses contractimpl from src/lib.rs.
 */
function getContractFunctions(wasmPath, contractSrcPath) {
  // 1. Try stellar CLI if WASM exists
  if (wasmPath && existsSync(wasmPath)) {
    try {
      const output = execSync(
        `stellar contract info interface --wasm ${wasmPath}`,
        { encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] }
      );
      // parse exported function names from output
      const matches = [...output.matchAll(/(?:fn|function)\s+([a-zA-Z0-9_]+)/g)];
      if (matches.length > 0) {
        return new Set(matches.map(m => m[1]));
      }
    } catch {
      try {
        const output = execSync(
          `stellar contract inspect --wasm ${wasmPath} --output json`,
          { encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] }
        );
        const spec = JSON.parse(output);
        const fns = spec.functions || spec.methods || [];
        if (fns.length > 0) {
          return new Set(fns.map(f => f.name));
        }
      } catch {
        // Fall back to source
      }
    }
  }

  // 2. Parse contract source src/lib.rs
  if (existsSync(contractSrcPath)) {
    const src = readFileSync(contractSrcPath, "utf-8");
    // Match functions inside #[contractimpl] impl TrustLinkContract
    const contractImplMatch = src.match(/#\[contractimpl\]\s*impl\s+TrustLinkContract\s*\{([\s\S]*?)\n\}/);
    const codeBlock = contractImplMatch ? contractImplMatch[1] : src;
    const matches = [...codeBlock.matchAll(/pub\s+fn\s+([a-zA-Z0-9_]+)\s*\(/g)];
    if (matches.length > 0) {
      return new Set(matches.map(m => m[1]));
    }
  }

  throw new Error(`Unable to extract contract entrypoints from ${wasmPath} or ${contractSrcPath}`);
}

/**
 * Extract wrapped contract methods from TypeScript SDK
 */
function extractTypeScriptMethods(tsSdkDir) {
  const clientPath = join(tsSdkDir, "src/client.ts");
  if (!existsSync(clientPath)) {
    return new Set();
  }
  const content = readFileSync(clientPath, "utf-8");
  const methods = new Set();
  const re = /(?:this\.simulate(?:<[^>]+>)?|this\.contract\.call)\s*\(\s*["']([a-zA-Z0-9_]+)["']/g;
  let m;
  while ((m = re.exec(content)) !== null) {
    methods.add(m[1]);
  }
  return methods;
}

/**
 * Extract wrapped contract methods from Python SDK
 */
function extractPythonMethods(pySdkDir) {
  const methods = new Set();
  const files = [
    join(pySdkDir, "trustlink/client.py"),
    join(pySdkDir, "trustlink/async_client.py")
  ];

  for (const filePath of files) {
    if (!existsSync(filePath)) continue;
    const content = readFileSync(filePath, "utf-8");
    const re = /(?:self\._simulate_with_retry|self\._call_with_retry|self\._invoke|self\._call)\s*\([\s\S]*?["']([a-zA-Z0-9_]+)["']/g;
    let m;
    while ((m = re.exec(content)) !== null) {
      methods.add(m[1]);
    }
  }
  return methods;
}

/**
 * Load allowlist JSON
 */
function loadAllowlist(allowlistPath) {
  if (!existsSync(allowlistPath)) {
    return { typescript: [], python: [] };
  }
  const data = JSON.parse(readFileSync(allowlistPath, "utf-8"));
  return {
    typescript: new Set(data.typescript?.functions || data.typescript || []),
    python: new Set(data.python?.functions || data.python || [])
  };
}

function main() {
  const { values } = parseArgs({
    options: {
      wasm: { type: "string", default: DEFAULT_WASM_PATH },
      "contract-src": { type: "string", default: DEFAULT_CONTRACT_SRC },
      allowlist: { type: "string", default: DEFAULT_ALLOWLIST_PATH },
      "sdk-dir": { type: "string", default: join(ROOT, "sdk/typescript") },
      "python-dir": { type: "string", default: join(ROOT, "bindings/python") },
      strict: { type: "boolean", default: true },
      help: { type: "boolean", default: false }
    }
  });

  if (values.help) {
    console.log(`
SDK Contract Coverage Checker

Verifies that public contract entrypoints are wrapped by SDKs or explicitly allowlisted.

Options:
  --wasm <path>          Path to built WASM (optional)
  --contract-src <path>  Path to contract source (default: src/lib.rs)
  --allowlist <path>     Path to allowlist JSON (default: scripts/sdk-coverage-allowlist.json)
  --strict               Fail on unallowed missing wrappers (default: true)
  --help                 Show this help
`);
    return;
  }

  console.log("=== TrustLink SDK Contract Coverage Checker ===");

  let contractFns;
  try {
    contractFns = getContractFunctions(values.wasm, values["contract-src"]);
  } catch (err) {
    console.error(`ERROR: ${err.message}`);
    process.exit(2);
  }

  console.log(`Contract exports: ${contractFns.size} public functions`);

  const allowlist = loadAllowlist(values.allowlist);
  const tsMethods = extractTypeScriptMethods(values["sdk-dir"]);
  const pyMethods = extractPythonMethods(values["python-dir"]);

  console.log(`TypeScript SDK wraps: ${tsMethods.size} methods`);
  console.log(`Python SDK wraps:     ${pyMethods.size} methods`);
  console.log(`Allowlist entries:    TypeScript: ${allowlist.typescript.size}, Python: ${allowlist.python.size}\n`);

  let failures = 0;

  // Evaluate TypeScript SDK coverage
  const missingInTS = [...contractFns].filter(fn => !tsMethods.has(fn) && !allowlist.typescript.has(fn));
  if (missingInTS.length > 0) {
    console.error(`[TypeScript SDK] FAIL: ${missingInTS.length} function(s) missing wrapper without allowlist entry:`);
    for (const fn of missingInTS) {
      console.error(`  - ${fn}`);
    }
    console.error("");
    failures += missingInTS.length;
  } else {
    console.log(`[TypeScript SDK] PASS: All ${contractFns.size} functions covered or explicitly allowlisted.`);
  }

  // Check for obsolete TS allowlist entries
  const obsoleteTS = [...allowlist.typescript].filter(fn => tsMethods.has(fn));
  if (obsoleteTS.length > 0) {
    console.warn(`[TypeScript SDK] NOTICE: ${obsoleteTS.length} function(s) are implemented but still in allowlist (can be removed):`);
    for (const fn of obsoleteTS) {
      console.warn(`  - ${fn}`);
    }
    console.warn("");
  }

  // Evaluate Python SDK coverage
  const missingInPy = [...contractFns].filter(fn => !pyMethods.has(fn) && !allowlist.python.has(fn));
  if (missingInPy.length > 0) {
    console.error(`[Python SDK] FAIL: ${missingInPy.length} function(s) missing wrapper without allowlist entry:`);
    for (const fn of missingInPy) {
      console.error(`  - ${fn}`);
    }
    console.error("");
    failures += missingInPy.length;
  } else {
    console.log(`[Python SDK] PASS: All ${contractFns.size} functions covered or explicitly allowlisted.`);
  }

  // Check for obsolete Python allowlist entries
  const obsoletePy = [...allowlist.python].filter(fn => pyMethods.has(fn));
  if (obsoletePy.length > 0) {
    console.warn(`[Python SDK] NOTICE: ${obsoletePy.length} function(s) are implemented but still in allowlist (can be removed):`);
    for (const fn of obsoletePy) {
      console.warn(`  - ${fn}`);
    }
    console.warn("");
  }

  if (failures > 0) {
    console.error(`\nFAILED: Found ${failures} unallowed missing SDK wrapper(s).`);
    console.error("Add SDK wrapper methods in sdk/typescript/src/client.ts or bindings/python/trustlink/client.py,");
    console.error("or add intentionally omitted contract functions to scripts/sdk-coverage-allowlist.json.");
    process.exit(1);
  }

  console.log("\nSUCCESS: All public contract entrypoints are wrapped or explicitly allowlisted!");
  process.exit(0);
}

main();
