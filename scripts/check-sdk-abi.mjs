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
      `stellar contract inspect --wasm ${wasmPath} --output json`,
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
  --wasm <path>    Path to WASM file (default: target/wasm32-unknown-unknown/release/trustlink.wasm)
  --sdk-dir <path> SDK directory to check (default: sdk/typescript)
  --help           Show this help

Exit codes:
  0 - All checks passed
  1 - Errors found (missing functions or arity mismatches)
  2 - Failed to extract contract spec
`);
    return;
  }

  const wasmPath = values.wasm;
  const sdkDir = values.sdkDir;

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

  const functionCount = spec.functions?.length || spec.methods?.length || 0;
  console.log(`\nContract has ${functionCount} function(s)`);

  // Check each SDK
  const results = [];

  // TypeScript SDK
  if (existsSync(join(sdkDir, "src/client.ts"))) {
    console.log("\nChecking TypeScript SDK...");
    const { functions, errors } = extractTypeScriptFunctions(sdkDir);
    if (errors.length > 0) {
      console.error("Errors extracting TypeScript functions:", errors);
    }
    const checkResult = checkSDK(spec, functions, "typescript");
    results.push(checkResult);
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

  if (!success) {
    console.error("\nFAILED: SDK-ABI mismatch detected");
    process.exit(1);
  }

  console.log("\nOK: All SDK methods match contract ABI");
  process.exit(0);
}

main().catch(err => {
  console.error("Unexpected error:", err);
  process.exit(2);
});