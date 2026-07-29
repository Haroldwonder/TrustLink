import { collectBindingsTsObservations } from "../src/collectors/bindings-ts";
import { collectPythonObservations } from "../src/collectors/python";
import { collectSdkObservations } from "../src/collectors/sdk";
import { assertObservationsMatch } from "../src/normalize";
import { isRpcAvailable, prepareConformanceContext } from "../src/setup";

const shouldRun = process.env.RUN_CONFORMANCE === "1" || process.env.CI === "true";

(shouldRun ? describe : describe.skip)("cross-SDK conformance", () => {
  let skipReason = "";

  beforeAll(async () => {
    if (!(await isRpcAvailable())) {
      skipReason = "local Soroban RPC is not available";
      return;
    }
  }, 30_000);

  it(
    "sdk, bindings/typescript, and bindings/python observe identical results",
    async () => {
      if (skipReason) {
        throw new Error(skipReason);
      }

      const ctx = await prepareConformanceContext();
      const [sdk, bindingsTs, python] = await Promise.all([
        collectSdkObservations(ctx),
        collectBindingsTsObservations(ctx),
        Promise.resolve(collectPythonObservations(ctx)),
      ]);

      assertObservationsMatch(sdk, bindingsTs, "bindings/typescript");
      assertObservationsMatch(sdk, python, "bindings/python");
    },
    180_000,
  );
});
