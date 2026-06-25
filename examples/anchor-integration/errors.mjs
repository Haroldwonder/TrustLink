/**
 * TrustLink contract error codes and parser.
 * Mirrors the error enum in the TrustLink Soroban contract (types.rs).
 */

export const TRUSTLINK_ERROR_NAMES = {
  1: "AlreadyInitialized",
  2: "NotInitialized",
  3: "Unauthorized",
  4: "NotFound",
  5: "DuplicateAttestation",
  6: "AlreadyRevoked",
  7: "Expired",
  8: "InvalidValidFrom",
  9: "InvalidExpiration",
  10: "MetadataTooLong",
  11: "InvalidTimestamp",
  12: "InvalidFee",
  13: "FeeTokenRequired",
  14: "TooManyTags",
  15: "TagTooLong",
  16: "InvalidThreshold",
  17: "NotRequiredSigner",
  18: "AlreadySigned",
  19: "ProposalFinalized",
  20: "ProposalExpired",
  21: "ReasonTooLong",
  22: "CannotEndorseOwn",
  23: "AlreadyEndorsed",
  24: "ContractPaused",
  25: "LimitExceeded",
  26: "SelfAttestation",
  27: "InvalidClaimType",
  28: "RequestNotFound",
  29: "RequestAlreadyFulfilled",
};

/**
 * Decode a Soroban contract error string into a human-readable message.
 * Handles "Error(Contract, #N)" patterns emitted by the TrustLink contract.
 * Returns the original message unchanged when no contract error is detected.
 *
 * @param {unknown} error
 * @returns {string}
 */
export function parseTrustLinkError(error) {
  const msg = error instanceof Error ? error.message : String(error);
  const match = msg.match(/Error\(Contract,\s*#(\d+)\)/);
  if (match) {
    const code = parseInt(match[1], 10);
    const name = TRUSTLINK_ERROR_NAMES[code] ?? "UnknownError";
    return `TrustLink contract error #${code}: ${name}`;
  }
  return msg;
}
