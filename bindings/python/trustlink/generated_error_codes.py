"""
GENERATED FILE — DO NOT EDIT BY HAND.
Run `node scripts/generate-error-codes.mjs` (or `make generate`) to regenerate.
Source of truth: src/errors.rs
"""

from typing import Dict

# Map of contract error code -> error name, generated from src/errors.rs.
CONTRACT_ERRORS: Dict[int, str] = {
    1: "AlreadyInitialized",
    2: "NotInitialized",
    3: "Unauthorized",  # Caller lacks required permissions. Includes rejection when `issuer` equals `subject` in `create_attestation`.
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
    16: "InvalidThreshold",  # Threshold must be >= 1 and <= number of required signers.
    17: "NotRequiredSigner",  # The signer is not in the proposal's required_signers list.
    18: "AlreadySigned",  # The signer has already co-signed this proposal.
    19: "ProposalFinalized",  # The proposal has already been finalized.
    20: "ProposalExpired",  # The proposal has expired without reaching threshold.
    21: "ReasonTooLong",  # The revocation reason exceeds the maximum allowed length of 128 characters.
    22: "CannotEndorseOwn",  # Endorser cannot endorse their own attestation.
    23: "AlreadyEndorsed",  # Endorser has already endorsed this attestation.
    24: "ContractPaused",  # The contract is paused; write operations are temporarily disabled.
    25: "SubjectNotWhitelisted",  # Subject is not on the issuer's whitelist and the issuer has whitelist mode enabled.
    26: "InvalidClaimType",  # Claim type string is empty, too long, or contains disallowed characters.
    27: "InvalidJurisdiction",  # Jurisdiction code is not a valid ISO 3166-1 alpha-2 code.
    28: "RateLimited",  # Issuer has exceeded the minimum issuance interval (rate limit).
    29: "LimitExceeded",  # Storage limit exceeded for issuer or subject.
    30: "ProposalCancelled",  # The proposal has been cancelled by the proposer.
    44: "InvalidSourceReference",  # Source reference string is missing or empty.
}
