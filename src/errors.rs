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
    /// Dispute has already been raised for this attestation.
    AlreadyDisputed = 31,
    /// Metadata does not match required format or constraints.
    InvalidMetadata = 32,
    /// Constraint violation for claim type.
    ConstraintViolation = 33,
    /// Request has already been processed.
    RequestAlreadyProcessed = 34,
    /// Request has expired.
    RequestExpired = 35,
    /// Duplicate request.
    DuplicateRequest = 36,
    /// Council proposal has already been executed.
    CouncilProposalExecuted = 37,
    /// Timelock period has not elapsed yet.
    TimelockNotReady = 38,
    /// Attestation is not disputed.
    NotDisputed = 39,
    /// Cannot remove the last admin.
    LastAdminCannotBeRemoved = 40,
    /// Invalid fee token.
    InvalidFeeToken = 41,
    /// Cannot delegate to self.
    CannotDelegateToSelf = 42,
    /// Already approved.
    AlreadyApproved = 43,
    /// `chunk_size` passed to `set_chunk_size` is 0 or otherwise out of range.
    InvalidChunkSize = 45,
    /// Claim type is not in the registry when registration is required.
    NotRegisteredClaimType = 46,
}
