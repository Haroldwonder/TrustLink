import { execFileSync } from "child_process";
import { resolve } from "path";
import type { ConformanceContext, ConformanceObservation } from "../types";

export function collectPythonObservations(ctx: ConformanceContext): ConformanceObservation[] {
  const scriptPath = resolve(__dirname, "../../python/collect.py");
  const output = execFileSync("python3", [scriptPath], {
    input: JSON.stringify(ctx),
    encoding: "utf8",
    stdio: ["pipe", "pipe", "pipe"],
  });

  return JSON.parse(output) as ConformanceObservation[];
}
