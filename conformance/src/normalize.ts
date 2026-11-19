import type { ConformanceObservation } from "./types";

const ERROR_NAME_TO_CODE: Record<string, number> = {
  AlreadyInitialized: 1,
  NotInitialized: 2,
  Unauthorized: 3,
  NotFound: 4,
  DuplicateAttestation: 5,
  AlreadyRevoked: 6,
  Expired: 7,
};

export function normalizeAttestationIds(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.map((entry) => {
    if (typeof entry === "string") {
      return entry;
    }
    if (entry && typeof entry === "object" && "id" in entry) {
      return String((entry as { id: unknown }).id);
    }
    return String(entry);
  });
}

export function parseErrorObservation(step: string, error: unknown): ConformanceObservation {
  if (error && typeof error === "object") {
    if ("code" in error && typeof (error as { code: unknown }).code === "number") {
      const code = (error as { code: number }).code;
      const rawName =
        "name" in error && typeof (error as { name: unknown }).name === "string"
          ? (error as { name: string }).name
          : `ContractError(${code})`;
      const name = rawName.replace(/Error$/, "");
      return { step, kind: "error", code, name };
    }
  }

  const message = error instanceof Error ? error.message : String(error);
  const contractMatch = message.match(/Error\(Contract,\s*#(\d+)\)/);
  if (contractMatch) {
    const code = Number.parseInt(contractMatch[1], 10);
    return { step, kind: "error", code, name: `ContractError(${code})` };
  }

  const namedCode = ERROR_NAME_TO_CODE[message];
  if (namedCode !== undefined) {
    return { step, kind: "error", code: namedCode, name: message };
  }

  throw new Error(`Unable to normalize error for step ${step}: ${message}`);
}

export function assertObservationsMatch(
  baseline: ConformanceObservation[],
  actual: ConformanceObservation[],
  clientName: string,
): void {
  const baselineJson = JSON.stringify(baseline, null, 2);
  const actualJson = JSON.stringify(actual, null, 2);
  if (baselineJson !== actualJson) {
    throw new Error(
      `Conformance drift detected for ${clientName}.\nExpected:\n${baselineJson}\nActual:\n${actualJson}`,
    );
  }
}
