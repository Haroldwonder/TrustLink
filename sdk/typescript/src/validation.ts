/**
 * TrustLink SDK - Input Validation
 *
 * Client-side validation helpers to catch invalid inputs before RPC calls.
 */

// ─── Error Classes ───────────────────────────────────────────────────────────

/**
 * Base error class for TrustLink SDK errors.
 */
export class TrustLinkValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TrustLinkValidationError";
  }
}

/**
 * Error thrown when an address format is invalid.
 */
export class InvalidAddressError extends TrustLinkValidationError {
  constructor(address: string, reason?: string) {
    super(
      `Invalid Stellar address: "${address}"${reason ? ` (${reason})` : ""}`
    );
    this.name = "InvalidAddressError";
  }
}

/**
 * Error thrown when a claim type is invalid.
 */
export class InvalidClaimTypeError extends TrustLinkValidationError {
  constructor(claimType: string, reason?: string) {
    super(
      `Invalid claim type: "${claimType}"${reason ? ` (${reason})` : ""}`
    );
    this.name = "InvalidClaimTypeError";
  }
}

/**
 * Error thrown when a numeric value is out of valid range.
 */
export class InvalidNumericValueError extends TrustLinkValidationError {
  constructor(value: number | bigint, field: string, reason?: string) {
    super(
      `Invalid ${field}: ${value}${reason ? ` (${reason})` : ""}`
    );
    this.name = "InvalidNumericValueError";
  }
}

// ─── Validation Helpers ─────────────────────────────────────────────────────

// Stellar address pattern: starts with G, followed by 56 base32 chars
const STELLAR_ADDRESS_REGEX = /^G[A-Z0-9]{55}$/i;

// Contract address pattern: starts with C, followed by 56 base32 chars
const CONTRACT_ADDRESS_REGEX = /^C[A-Z0-9]{55}$/i;

/**
 * Validates a Stellar address format (G... address).
 * @param addr - The address string to validate
 * @throws InvalidAddressError if the address is invalid
 */
export function validateAddress(addr: string): void {
  if (!addr || typeof addr !== "string") {
    throw new InvalidAddressError(String(addr), "address is required");
  }

  const trimmed = addr.trim();

  if (trimmed.length === 0) {
    throw new InvalidAddressError(addr, "address cannot be empty");
  }

  if (!STELLAR_ADDRESS_REGEX.test(trimmed)) {
    // Also accept contract addresses (C...)
    if (!CONTRACT_ADDRESS_REGEX.test(trimmed)) {
      throw new InvalidAddressError(
        addr,
        "must be a valid Stellar address (G... or C...)"
      );
    }
  }
}

/**
 * Validates a claim type string.
 * @param claimType - The claim type to validate
 * @throws InvalidClaimTypeError if the claim type is invalid
 */
export function validateClaimType(claimType: string): void {
  if (!claimType || typeof claimType !== "string") {
    throw new InvalidClaimTypeError(String(claimType), "claim type is required");
  }

  const trimmed = claimType.trim();

  if (trimmed.length === 0) {
    throw new InvalidClaimTypeError(claimType, "cannot be empty");
  }

  // Claim types should be UPPER_SNAKE_CASE typically
  // Allow alphanumeric and underscore, must start with letter
  if (!/^[a-zA-Z][a-zA-Z0-9_]*$/.test(trimmed)) {
    throw new InvalidClaimTypeError(
      claimType,
      "must start with a letter and contain only letters, numbers, and underscores"
    );
  }
}

/**
 * Validates a non-negative numeric value.
 * @param value - The numeric value to validate
 * @param fieldName - Name of the field for error messages
 * @throws InvalidNumericValueError if the value is invalid
 */
export function validateNonNegative(value: number | bigint, fieldName: string): void {
  if (typeof value === "number") {
    if (isNaN(value)) {
      throw new InvalidNumericValueError(value, fieldName, "is NaN");
    }
    if (!isFinite(value)) {
      throw new InvalidNumericValueError(value, fieldName, "is not finite");
    }
    if (value < 0) {
      throw new InvalidNumericValueError(value, fieldName, "must be non-negative");
    }
  } else if (typeof value === "bigint") {
    if (value < 0n) {
      throw new InvalidNumericValueError(value, fieldName, "must be non-negative");
    }
  }
}

/**
 * Validates a positive numeric value.
 * @param value - The numeric value to validate
 * @param fieldName - Name of the field for error messages
 * @throws InvalidNumericValueError if the value is invalid
 */
export function validatePositive(value: number | bigint, fieldName: string): void {
  if (typeof value === "number") {
    if (isNaN(value)) {
      throw new InvalidNumericValueError(value, fieldName, "is NaN");
    }
    if (!isFinite(value)) {
      throw new InvalidNumericValueError(value, fieldName, "is not finite");
    }
    if (value <= 0) {
      throw new InvalidNumericValueError(value, fieldName, "must be positive");
    }
  } else if (typeof value === "bigint") {
    if (value <= 0n) {
      throw new InvalidNumericValueError(value, fieldName, "must be positive");
    }
  }
}

/**
 * Validates an attestation ID format.
 * @param id - The attestation ID to validate
 * @throws TrustLinkValidationError if the ID is invalid
 */
export function validateAttestationId(id: string): void {
  if (!id || typeof id !== "string") {
    throw new TrustLinkValidationError(`Invalid attestation ID: ${id}`);
  }

  const trimmed = id.trim();
  if (trimmed.length === 0) {
    throw new TrustLinkValidationError("Attestation ID cannot be empty");
  }
}

/**
 * Validates a proposal ID format.
 * @param id - The proposal ID to validate
 * @throws TrustLinkValidationError if the ID is invalid
 */
export function validateProposalId(id: string): void {
  if (!id || typeof id !== "string") {
    throw new TrustLinkValidationError(`Invalid proposal ID: ${id}`);
  }

  const trimmed = id.trim();
  if (trimmed.length === 0) {
    throw new TrustLinkValidationError("Proposal ID cannot be empty");
  }
}

/**
 * Validates a request ID format.
 * @param id - The request ID to validate
 * @throws TrustLinkValidationError if the ID is invalid
 */
export function validateRequestId(id: string): void {
  if (!id || typeof id !== "string") {
    throw new TrustLinkValidationError(`Invalid request ID: ${id}`);
  }

  const trimmed = id.trim();
  if (trimmed.length === 0) {
    throw new TrustLinkValidationError("Request ID cannot be empty");
  }
}

/**
 * Validates a template ID format.
 * @param id - The template ID to validate
 * @throws TrustLinkValidationError if the ID is invalid
 */
export function validateTemplateId(id: string): void {
  if (!id || typeof id !== "string") {
    throw new TrustLinkValidationError(`Invalid template ID: ${id}`);
  }

  const trimmed = id.trim();
  if (trimmed.length === 0) {
    throw new TrustLinkValidationError("Template ID cannot be empty");
  }
}
