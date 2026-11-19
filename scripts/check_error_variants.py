#!/usr/bin/env python3
"""Verify every Error::Variant reference in src/ is defined in src/errors.rs.

Companion to StorageKey completeness checks: catches the same class of
"referenced but never defined" breakage for contract Error variants.

Exit 0 on success; exit 1 and print missing variants on failure.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"
ERRORS_RS = SRC / "errors.rs"

# Match enum variant definitions: `VariantName = N,` (with optional doc comments above).
VARIANT_DEF = re.compile(r"^\s*([A-Z][A-Za-z0-9]*)\s*=\s*\d+\s*,?\s*$", re.MULTILINE)

# Match Error::Variant in code and docs (Error::X, types::Error::X, crate::types::Error::X).
VARIANT_REF = re.compile(
    r"(?:(?:crate::)?(?:types::)?)?Error::([A-Z][A-Za-z0-9]*)"
)


def defined_variants(text: str) -> set[str]:
    return set(VARIANT_DEF.findall(text))


def referenced_variants(src_dir: Path) -> dict[str, list[str]]:
    """Map variant name -> list of 'path:line' locations (excluding errors.rs defs)."""
    refs: dict[str, list[str]] = {}
    for path in sorted(src_dir.rglob("*.rs")):
        if path.resolve() == ERRORS_RS.resolve():
            continue
        for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for match in VARIANT_REF.finditer(line):
                name = match.group(1)
                rel = path.relative_to(ROOT)
                refs.setdefault(name, []).append(f"{rel}:{lineno}")
    return refs


def main() -> int:
    if not ERRORS_RS.is_file():
        print(f"error: missing {ERRORS_RS}", file=sys.stderr)
        return 1

    defined = defined_variants(ERRORS_RS.read_text(encoding="utf-8"))
    if not defined:
        print("error: no Error variants found in src/errors.rs", file=sys.stderr)
        return 1

    refs = referenced_variants(SRC)
    missing = sorted(name for name in refs if name not in defined)

    print(f"Defined Error variants: {len(defined)}")
    print(f"Referenced Error variants: {len(refs)}")

    if missing:
        print("\nERROR: Error variants referenced in src/ but not defined in src/errors.rs:\n")
        for name in missing:
            print(f"  Error::{name}")
            for loc in refs[name][:5]:
                print(f"    - {loc}")
            if len(refs[name]) > 5:
                print(f"    - ... and {len(refs[name]) - 5} more")
        print(
            "\nAdd the missing variant(s) to the Error enum in src/errors.rs "
            "(or fix the reference)."
        )
        return 1

    print("OK: every Error::Variant reference in src/ resolves to a defined variant.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
