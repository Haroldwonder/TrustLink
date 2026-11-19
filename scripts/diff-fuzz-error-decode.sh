#!/usr/bin/env bash
# Differential fuzz: feed the same error-code corpus through Python and
# TypeScript SDK decode paths and assert equivalent classification.
#
# Usage: scripts/diff-fuzz-error-decode.sh
# Exit 0 on match; non-zero if either SDK drifts from sdk/error-codes.json
# or the two SDKs disagree on any sample.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORPUS="$ROOT/sdk/error-codes.json"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

if [[ ! -f "$CORPUS" ]]; then
  echo "ERROR: missing shared corpus at $CORPUS" >&2
  exit 1
fi

# ── Build sample set: every known code + 500 deterministic random codes ──────
python3 - "$CORPUS" "$WORKDIR/samples.json" <<'PY'
import json, random, sys
corpus = json.load(open(sys.argv[1]))
known = {e["code"]: e["name"] for e in corpus["errors"]}
rng = random.Random(0x7E57110)
codes = sorted(set(known) | {0, -1, 31, 43, 45, 100, 255, 1000} | {rng.randrange(512) for _ in range(500)})
samples = [{"code": c, "expected": known.get(c)} for c in codes]
json.dump({"samples": samples, "known": known}, open(sys.argv[2], "w"), indent=2)
print(f"Generated {len(samples)} samples ({len(known)} known codes)")
PY

# ── Python decode path ───────────────────────────────────────────────────────
python3 - "$ROOT" "$WORKDIR/samples.json" "$WORKDIR/python.json" <<'PY'
import importlib.util, json, sys
from pathlib import Path

root = Path(sys.argv[1])
types_path = root / "bindings/python/trustlink/types.py"
spec = importlib.util.spec_from_file_location("trustlink_types_fuzz", types_path)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

data = json.load(open(sys.argv[2]))
out = []
for s in data["samples"]:
    code = s["code"]
    name = mod.classify_error_code(code)
    parsed = mod.decode_contract_error(f"Error(Contract, #{code})")
    out.append({
        "code": code,
        "classify": name,
        "parsed_name": None if parsed is None else parsed.message,
        "parsed_code": None if parsed is None else parsed.code,
    })
json.dump(out, open(sys.argv[3], "w"), indent=2)
print(f"Python decoded {len(out)} samples")
PY

# ── TypeScript decode path ───────────────────────────────────────────────────
node --input-type=module - "$ROOT" "$WORKDIR/samples.json" "$WORKDIR/typescript.json" <<'JS'
import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { join } from "node:path";

const root = process.argv[2];
const samples = JSON.parse(readFileSync(process.argv[3], "utf8"));

// Prefer compiled dist/; fall back to loading types via a tiny dynamic import
// of the source through the package's test-time resolution.
let classifyErrorCode, parseTrustLinkError;
try {
  const mod = await import(pathToFileURL(join(root, "sdk/typescript/dist/types.js")).href);
  classifyErrorCode = mod.classifyErrorCode;
  parseTrustLinkError = mod.parseTrustLinkError;
} catch {
  // Build on the fly if dist is missing
  const { execSync } = await import("node:child_process");
  execSync("npx tsc -p tsconfig.json", { cwd: join(root, "sdk/typescript"), stdio: "inherit" });
  const mod = await import(pathToFileURL(join(root, "sdk/typescript/dist/types.js")).href);
  classifyErrorCode = mod.classifyErrorCode;
  parseTrustLinkError = mod.parseTrustLinkError;
}

const out = [];
for (const s of samples.samples) {
  const code = s.code;
  const name = classifyErrorCode(code);
  const parsed = parseTrustLinkError(`Error(Contract, #${code})`);
  out.push({
    code,
    classify: name,
    parsed_name: parsed ? parsed.name : null,
    parsed_code: parsed ? parsed.code : null,
  });
}
writeFileSync(process.argv[4], JSON.stringify(out, null, 2));
console.log(`TypeScript decoded ${out.length} samples`);
JS

# ── Compare ──────────────────────────────────────────────────────────────────
python3 - "$WORKDIR/samples.json" "$WORKDIR/python.json" "$WORKDIR/typescript.json" <<'PY'
import json, sys

samples = json.load(open(sys.argv[1]))
py = {r["code"]: r for r in json.load(open(sys.argv[2]))}
ts = {r["code"]: r for r in json.load(open(sys.argv[3]))}

failures = []
for s in samples["samples"]:
    code = s["code"]
    expected = s["expected"]
    p, t = py[code], ts[code]

    if p["classify"] != expected:
        failures.append(f"code {code}: Python classify={p['classify']!r} expected={expected!r}")
    if t["classify"] != expected:
        failures.append(f"code {code}: TypeScript classify={t['classify']!r} expected={expected!r}")
    if p["classify"] != t["classify"]:
        failures.append(
            f"code {code}: Python/TS drift classify py={p['classify']!r} ts={t['classify']!r}"
        )
    if p["parsed_name"] != t["parsed_name"]:
        failures.append(
            f"code {code}: Python/TS drift parse py={p['parsed_name']!r} ts={t['parsed_name']!r}"
        )

if failures:
    print(f"FAIL: {len(failures)} differential fuzz disagreement(s):")
    for f in failures[:50]:
        print(f"  - {f}")
    if len(failures) > 50:
        print(f"  ... and {len(failures) - 50} more")
    sys.exit(1)

print(f"OK: Python and TypeScript agree on {len(samples['samples'])} samples "
      f"({len(samples['known'])} known codes + edge cases)")
PY
