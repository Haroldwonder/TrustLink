#!/usr/bin/env bash
# Verify sdk/error-codes.json matches src/errors.rs and both SDK error tables.
# Usage: scripts/check-error-code-sync.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORPUS="$ROOT/sdk/error-codes.json"
ERRORS_RS="$ROOT/src/errors.rs"

python3 - "$CORPUS" "$ERRORS_RS" "$ROOT" <<'PY'
import json, re, sys
from pathlib import Path

corpus_path, errors_rs, root = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
corpus = {e["code"]: e["name"] for e in json.loads(corpus_path.read_text())["errors"]}

# Parse Rust: VariantName = N,
rust = {}
for m in re.finditer(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(\d+)\s*,", errors_rs.read_text(), re.M):
    rust[int(m.group(2))] = m.group(1)

if rust != corpus:
    print("FAIL: sdk/error-codes.json does not match src/errors.rs")
    only_rust = sorted(set(rust) - set(corpus))
    only_corpus = sorted(set(corpus) - set(rust))
    mismatched = sorted(k for k in set(rust) & set(corpus) if rust[k] != corpus[k])
    if only_rust:
        print("  in Rust only:", {k: rust[k] for k in only_rust})
    if only_corpus:
        print("  in corpus only:", {k: corpus[k] for k in only_corpus})
    if mismatched:
        print("  name mismatches:", {k: (rust[k], corpus[k]) for k in mismatched})
    sys.exit(1)

# Python CONTRACT_ERRORS
py_text = (root / "bindings/python/trustlink/types.py").read_text()
py_block = re.search(r"CONTRACT_ERRORS\s*=\s*\{([^}]+)\}", py_text, re.S)
if not py_block:
    print("FAIL: could not find CONTRACT_ERRORS in Python SDK")
    sys.exit(1)
py = {int(c): n for c, n in re.findall(r"(\d+)\s*:\s*\"([^\"]+)\"", py_block.group(1))}
if py != corpus:
    print("FAIL: Python CONTRACT_ERRORS drifted from sdk/error-codes.json")
    print("  python:", py)
    print("  corpus:", corpus)
    sys.exit(1)

# TypeScript ERROR_BY_CODE in sdk/typescript/src/types.ts
ts_text = (root / "sdk/typescript/src/types.ts").read_text()
ts_block = re.search(
    r"const ERROR_BY_CODE\b[^{]*\{([^}]+)\}",
    ts_text,
    re.S,
)
if not ts_block:
    print("FAIL: could not find ERROR_BY_CODE in TypeScript SDK")
    sys.exit(1)
# Map code -> class name like AlreadyInitializedError → AlreadyInitialized
ts_raw = {
    int(c): cls
    for c, cls in re.findall(r"(\d+)\s*:\s*([A-Za-z_][A-Za-z0-9_]*)", ts_block.group(1))
}
ts = {}
for code, cls in ts_raw.items():
    name = cls[:-5] if cls.endswith("Error") else cls
    ts[code] = name
if ts != corpus:
    print("FAIL: TypeScript ERROR_BY_CODE drifted from sdk/error-codes.json")
    print("  typescript:", ts)
    print("  corpus:", corpus)
    sys.exit(1)

print(f"OK: Rust, corpus, Python, and TypeScript agree on {len(corpus)} error codes")
PY
