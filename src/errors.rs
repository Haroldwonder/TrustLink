//! Error definitions for TrustLink.
//!
//! All contract error codes are defined here and re-exported from the crate root.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    /// Caller lacks required permissions. Includes rejection when `issuer` equals `subject` in `create_attestation`.
    Unauthorized = 3,
    NotFound = 4,
    DuplicateAttestation = 5,
    AlreadyRevoked = 6,
    Expired = 7,
    InvalidValidFrom = 8,
    InvalidExpiration = 9,
    MetadataTooLong = 10,
    /// Source reference string is missing or empty.
    InvalidSourceReference = 44,
    InvalidTimestamp = 11,
    InvalidFee = 12,
    FeeTokenRequired = 13,
    TooManyTags = 14,
    TagTooLong = 15,
    /// Threshold must be >= 1 and <= number of required signers.
    InvalidThreshold = 16,
    /// The signer is not in the proposal's required_signers list.
    NotRequiredSigner = 17,
    /// The signer has already co-signed this proposal.
    AlreadySigned = 18,
    /// The proposal has already been finalized.
    ProposalFinalized = 19,
    /// The proposal has expired without reaching threshold.
    ProposalExpired = 20,
    /// The revocation reason exceeds the maximum allowed length of 128 characters.
    ReasonTooLong = 21,
    /// Endorser cannot endorse their own attestation.
    CannotEndorseOwn = 22,
    /// Endorser has already endorsed this attestation.
    AlreadyEndorsed = 23,
    /// The contract is paused; write operations are temporarily disabled.
    ContractPaused = 24,
    /// Subject is not on the issuer's whitelist and the issuer has whitelist mode enabled.
    SubjectNotWhitelisted = 25,
    /// Claim type string is empty, too long, or contains disallowed characters.
    InvalidClaimType = 26,
    /// Jurisdiction code is not a valid ISO 3166-1 alpha-2 code.
    InvalidJurisdiction = 27,
    /// Issuer has exceeded the minimum issuance interval (rate limit).
    RateLimited = 28,
    /// Storage limit exceeded for issuer or subject.
    LimitExceeded = 29,
    /// The proposal has been cancelled by the proposer.
    ProposalCancelled = 30,
    /// A pending attestation request already exists for this subject/issuer/claim.
    DuplicateRequest = 31,
    /// The attestation request was already fulfilled, rejected, or cancelled.
    RequestAlreadyProcessed = 32,
    /// The attestation request TTL has elapsed.
    RequestExpired = 33,
    /// Metadata is present but is not a valid hex-encoded hash (when hash-only mode is on).
    InvalidMetadata = 34,
    /// Metadata does not satisfy the claim type's registered constraints.
    ConstraintViolation = 35,
    /// Claim type is not in the registry when registration is required.
    NotRegisteredClaimType = 36,
    /// Cannot remove the last remaining admin from the council.
    LastAdminCannotBeRemoved = 37,
    /// Issuer cannot delegate a claim type to themselves.
    CannotDelegateToSelf = 38,
    /// The admin-council proposal has already been executed.
    CouncilProposalExecuted = 39,
    /// The admin has already approved this council proposal.
    AlreadyApproved = 40,
    /// The council proposal timelock has not elapsed yet.
    TimelockNotReady = 41,
    /// No active dispute exists for this attestation.
    NotDisputed = 42,
    /// A dispute is already open for this attestation.
    AlreadyDisputed = 43,
    /// The configured fee token address is invalid or unusable.
    InvalidFeeToken = 45,
}
