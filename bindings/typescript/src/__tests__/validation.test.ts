import { describe, it, expect } from "vitest";
import {
  validateAddress,
  validateClaimType,
  validateNonNegative,
  validatePositive,
  validateAttestationId,
  InvalidAddressError,
  InvalidClaimTypeError,
  InvalidNumericValueError,
  TrustLinkError,
} from "../validation";

const VALID_G_ADDR = "G" + "A".repeat(55);
const VALID_C_ADDR = "C" + "A".repeat(55);

describe("validateAddress", () => {
  it("accepts a valid G... address", () => {
    expect(() => validateAddress(VALID_G_ADDR)).not.toThrow();
  });

  it("accepts a valid C... contract address", () => {
    expect(() => validateAddress(VALID_C_ADDR)).not.toThrow();
  });

  it("rejects an empty address", () => {
    expect(() => validateAddress("")).toThrow(InvalidAddressError);
  });

  it("rejects a malformed address", () => {
    expect(() => validateAddress("not-an-address")).toThrow(InvalidAddressError);
  });
});

describe("validateClaimType", () => {
  it("accepts a valid UPPER_SNAKE_CASE claim type", () => {
    expect(() => validateClaimType("KYC_PASSED")).not.toThrow();
  });

  it("rejects an empty claim type", () => {
    expect(() => validateClaimType("")).toThrow(InvalidClaimTypeError);
  });

  it("rejects a claim type starting with a digit", () => {
    expect(() => validateClaimType("1BAD")).toThrow(InvalidClaimTypeError);
  });
});

describe("validateNonNegative", () => {
  it("accepts zero and positive numbers", () => {
    expect(() => validateNonNegative(0, "field")).not.toThrow();
    expect(() => validateNonNegative(5, "field")).not.toThrow();
  });

  it("rejects negative numbers", () => {
    expect(() => validateNonNegative(-1, "field")).toThrow(InvalidNumericValueError);
  });

  it("rejects NaN", () => {
    expect(() => validateNonNegative(NaN, "field")).toThrow(InvalidNumericValueError);
  });

  it("accepts non-negative bigint and rejects negative bigint", () => {
    expect(() => validateNonNegative(0n, "field")).not.toThrow();
    expect(() => validateNonNegative(-1n, "field")).toThrow(InvalidNumericValueError);
  });
});

describe("validatePositive", () => {
  it("accepts positive numbers", () => {
    expect(() => validatePositive(1, "field")).not.toThrow();
  });

  it("rejects zero and negative numbers", () => {
    expect(() => validatePositive(0, "field")).toThrow(InvalidNumericValueError);
    expect(() => validatePositive(-1, "field")).toThrow(InvalidNumericValueError);
  });
});

describe("validateAttestationId", () => {
  it("accepts a non-empty id", () => {
    expect(() => validateAttestationId("abc123")).not.toThrow();
  });

  it("rejects an empty id", () => {
    expect(() => validateAttestationId("")).toThrow(TrustLinkError);
  });
});
