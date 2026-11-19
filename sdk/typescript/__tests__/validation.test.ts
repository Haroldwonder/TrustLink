import {
  validateAddress,
  validateClaimType,
  validateAttestationId,
  validateProposalId,
  validateRequestId,
  validateTemplateId,
  validateNonNegative,
  validatePositive,
  InvalidAddressError,
  InvalidClaimTypeError,
  InvalidNumericValueError,
  TrustLinkValidationError,
} from "../src/validation";

describe("Validation functions", () => {
  describe("validateAddress", () => {
    test("accepts valid Stellar addresses (G...)", () => {
      expect(() => validateAddress("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN")).not.toThrow();
    });

    test("accepts valid contract addresses (C...)", () => {
      expect(() => validateAddress("CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABCD")).not.toThrow();
    });

    test("rejects invalid address format", () => {
      expect(() => validateAddress("invalid")).toThrow(InvalidAddressError);
      expect(() => validateAddress("GABC")).toThrow(InvalidAddressError);
      expect(() => validateAddress("XAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN")).toThrow(InvalidAddressError);
    });

    test("rejects empty address", () => {
      expect(() => validateAddress("")).toThrow(InvalidAddressError);
      expect(() => validateAddress("   ")).toThrow(InvalidAddressError);
    });

    test("rejects non-string input", () => {
      expect(() => validateAddress(null as any)).toThrow(InvalidAddressError);
      expect(() => validateAddress(undefined as any)).toThrow(InvalidAddressError);
    });
  });

  describe("validateClaimType", () => {
    test("accepts valid claim types", () => {
      expect(() => validateClaimType("KYC_PASSED")).not.toThrow();
      expect(() => validateClaimType("IDENTITY_VERIFIED")).not.toThrow();
      expect(() => validateClaimType("CREDIT_SCORE")).not.toThrow();
    });

    test("accepts claim types starting with letter and containing alphanumeric/underscore", () => {
      expect(() => validateClaimType("A1_B2")).not.toThrow();
      expect(() => validateClaimType("Test123")).not.toThrow();
    });

    test("rejects claim types starting with number", () => {
      expect(() => validateClaimType("123_KYC")).toThrow(InvalidClaimTypeError);
    });

    test("rejects claim types with special characters", () => {
      expect(() => validateClaimType("KYC-PASSED")).toThrow(InvalidClaimTypeError);
      expect(() => validateClaimType("KYC.PASSED")).toThrow(InvalidClaimTypeError);
      expect(() => validateClaimType("KYC PASSED")).toThrow(InvalidClaimTypeError);
    });

    test("rejects empty claim type", () => {
      expect(() => validateClaimType("")).toThrow(InvalidClaimTypeError);
      expect(() => validateClaimType("   ")).toThrow(InvalidClaimTypeError);
    });

    test("rejects non-string input", () => {
      expect(() => validateClaimType(null as any)).toThrow(InvalidClaimTypeError);
      expect(() => validateClaimType(undefined as any)).toThrow(InvalidClaimTypeError);
    });
  });

  describe("validateAttestationId", () => {
    test("accepts non-empty strings", () => {
      expect(() => validateAttestationId("attestation-123")).not.toThrow();
      expect(() => validateAttestationId("abc")).not.toThrow();
    });

    test("rejects empty string", () => {
      expect(() => validateAttestationId("")).toThrow(TrustLinkValidationError);
      expect(() => validateAttestationId("   ")).toThrow(TrustLinkValidationError);
    });

    test("rejects non-string input", () => {
      expect(() => validateAttestationId(null as any)).toThrow(TrustLinkValidationError);
      expect(() => validateAttestationId(undefined as any)).toThrow(TrustLinkValidationError);
    });
  });

  describe("validateProposalId", () => {
    test("accepts non-empty strings", () => {
      expect(() => validateProposalId("proposal-123")).not.toThrow();
      expect(() => validateProposalId("abc")).not.toThrow();
    });

    test("rejects empty string", () => {
      expect(() => validateProposalId("")).toThrow(TrustLinkValidationError);
      expect(() => validateProposalId("   ")).toThrow(TrustLinkValidationError);
    });

    test("rejects non-string input", () => {
      expect(() => validateProposalId(null as any)).toThrow(TrustLinkValidationError);
      expect(() => validateProposalId(undefined as any)).toThrow(TrustLinkValidationError);
    });
  });

  describe("validateRequestId", () => {
    test("accepts non-empty strings", () => {
      expect(() => validateRequestId("request-123")).not.toThrow();
      expect(() => validateRequestId("abc")).not.toThrow();
    });

    test("rejects empty string", () => {
      expect(() => validateRequestId("")).toThrow(TrustLinkValidationError);
      expect(() => validateRequestId("   ")).toThrow(TrustLinkValidationError);
    });

    test("rejects non-string input", () => {
      expect(() => validateRequestId(null as any)).toThrow(TrustLinkValidationError);
      expect(() => validateRequestId(undefined as any)).toThrow(TrustLinkValidationError);
    });
  });

  describe("validateTemplateId", () => {
    test("accepts non-empty strings", () => {
      expect(() => validateTemplateId("template-123")).not.toThrow();
      expect(() => validateTemplateId("abc")).not.toThrow();
    });

    test("rejects empty string", () => {
      expect(() => validateTemplateId("")).toThrow(TrustLinkValidationError);
      expect(() => validateTemplateId("   ")).toThrow(TrustLinkValidationError);
    });

    test("rejects non-string input", () => {
      expect(() => validateTemplateId(null as any)).toThrow(TrustLinkValidationError);
      expect(() => validateTemplateId(undefined as any)).toThrow(TrustLinkValidationError);
    });
  });

  describe("validateNonNegative", () => {
    test("accepts non-negative numbers", () => {
      expect(() => validateNonNegative(0, "test")).not.toThrow();
      expect(() => validateNonNegative(1, "test")).not.toThrow();
      expect(() => validateNonNegative(100, "test")).not.toThrow();
    });

    test("accepts non-negative bigints", () => {
      expect(() => validateNonNegative(0n, "test")).not.toThrow();
      expect(() => validateNonNegative(1n, "test")).not.toThrow();
      expect(() => validateNonNegative(100n, "test")).not.toThrow();
    });

    test("rejects negative numbers", () => {
      expect(() => validateNonNegative(-1, "test")).toThrow(InvalidNumericValueError);
      expect(() => validateNonNegative(-100, "test")).toThrow(InvalidNumericValueError);
    });

    test("rejects negative bigints", () => {
      expect(() => validateNonNegative(-1n, "test")).toThrow(InvalidNumericValueError);
      expect(() => validateNonNegative(-100n, "test")).toThrow(InvalidNumericValueError);
    });

    test("rejects NaN", () => {
      expect(() => validateNonNegative(NaN, "test")).toThrow(InvalidNumericValueError);
    });

    test("rejects Infinity", () => {
      expect(() => validateNonNegative(Infinity, "test")).toThrow(InvalidNumericValueError);
      expect(() => validateNonNegative(-Infinity, "test")).toThrow(InvalidNumericValueError);
    });
  });

  describe("validatePositive", () => {
    test("accepts positive numbers", () => {
      expect(() => validatePositive(1, "test")).not.toThrow();
      expect(() => validatePositive(100, "test")).not.toThrow();
    });

    test("accepts positive bigints", () => {
      expect(() => validatePositive(1n, "test")).not.toThrow();
      expect(() => validatePositive(100n, "test")).not.toThrow();
    });

    test("rejects zero", () => {
      expect(() => validatePositive(0, "test")).toThrow(InvalidNumericValueError);
      expect(() => validatePositive(0n, "test")).toThrow(InvalidNumericValueError);
    });

    test("rejects negative numbers", () => {
      expect(() => validatePositive(-1, "test")).toThrow(InvalidNumericValueError);
      expect(() => validatePositive(-100, "test")).toThrow(InvalidNumericValueError);
    });

    test("rejects negative bigints", () => {
      expect(() => validatePositive(-1n, "test")).toThrow(InvalidNumericValueError);
      expect(() => validatePositive(-100n, "test")).toThrow(InvalidNumericValueError);
    });

    test("rejects NaN", () => {
      expect(() => validatePositive(NaN, "test")).toThrow(InvalidNumericValueError);
    });

    test("rejects Infinity", () => {
      expect(() => validatePositive(Infinity, "test")).toThrow(InvalidNumericValueError);
      expect(() => validatePositive(-Infinity, "test")).toThrow(InvalidNumericValueError);
    });
  });
});
