/**
 * Property / differential fuzz tests for TrustLink error decoding.
 *
 * Asserts every known contract error code (from sdk/error-codes.json) and a
 * stream of random edge-case codes decode to the expected classification.
 * Paired with bindings/python/tests/test_error_decode_fuzz.py and
 * scripts/diff-fuzz-error-decode.sh for cross-SDK equivalence.
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  classifyErrorCode,
  knownErrorCodes,
  parseTrustLinkError,
} from "../src/types";

const __dirname = dirname(fileURLToPath(import.meta.url));
const corpusPath = join(__dirname, "../../error-codes.json");
const corpus: { errors: Array<{ code: number; name: string }> } = JSON.parse(
  readFileSync(corpusPath, "utf8")
);

/** Deterministic PRNG (mulberry32) so CI failures are reproducible. */
function mulberry32(seed: number): () => number {
  let t = seed >>> 0;
  return () => {
    t = (t + 0x6d2b79f5) >>> 0;
    let r = Math.imul(t ^ (t >>> 15), 1 | t);
    r ^= r + Math.imul(r ^ (r >>> 7), 61 | r);
    return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
  };
}

describe("error-decode fuzz / property tests", () => {
  test("SDK error table matches shared corpus (sdk/error-codes.json)", () => {
    const sdk = knownErrorCodes();
    const expected = Object.fromEntries(
      corpus.errors.map((e) => [e.code, e.name])
    );

    expect(sdk).toEqual(expected);

    for (const { code, name } of corpus.errors) {
      expect(classifyErrorCode(code)).toBe(name);
      const parsed = parseTrustLinkError(`Error(Contract, #${code})`);
      expect(parsed).not.toBeNull();
      expect(parsed!.code).toBe(code);
      expect(parsed!.name).toBe(name);
    }
  });

  test("fuzzes random / edge-case codes against corpus classification", () => {
    const rand = mulberry32(0x7e57110); // fixed seed for reproducible CI failures
    const known = new Map(corpus.errors.map((e) => [e.code, e.name]));
    const samples: number[] = [
      ...corpus.errors.map((e) => e.code),
      0,
      -1,
      31,
      43,
      45,
      100,
      255,
      1000,
      Number.MAX_SAFE_INTEGER,
    ];

    for (let i = 0; i < 500; i++) {
      samples.push(Math.floor(rand() * 512));
    }

    for (const code of samples) {
      const expected = known.get(code) ?? null;
      expect(classifyErrorCode(code)).toBe(expected);

      const msg = `HostError: Error(Contract, #${code})`;
      const parsed = parseTrustLinkError(msg);
      if (expected === null) {
        expect(parsed).toBeNull();
      } else {
        expect(parsed!.name).toBe(expected);
        expect(parsed!.code).toBe(code);
      }
    }
  });
});
