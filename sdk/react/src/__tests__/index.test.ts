import { describe, it, expect } from "vitest";
import * as sdkReact from "../index";

describe("sdk/react/src/index exports", () => {
  it("exports useIssuerStats", () => {
    expect(typeof sdkReact.useIssuerStats).toBe("function");
  });
});
