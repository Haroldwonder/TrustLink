//! Error definitions for TrustLink.
//!
//! All contract error codes are defined here and re-exported from the crate root.
//! 
//! This enum is limited to 50 variants by Soroban's #[contracterror] constraint.
//! Rarely-used errors or errors that apply to specific entry points only should
//! be consolidated where semantically equivalent.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    // Core state errors (1-10)
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

    // Validation errors (11-30)
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

    // Attestation/dispute state errors (31-39)
    /// Dispute has already been raised for this attestation.
    AlreadyDisputed = 31,
    /// Attestation is in an invalid state (revoked, deleted, or otherwise cannot be transferred/disputed).
    InvalidAttestation = 32,
    /// Attestation is not disputed.
    NotDisputed = 33,

    // Metadata and constraints (34-36)
    /// Metadata does not match required format or constraints.
    InvalidMetadata = 34,
    /// Constraint violation for claim type.
    ConstraintViolation = 35,
    /// Source reference string is missing or empty.
    InvalidSourceReference = 36,

    // Request lifecycle (37-39)
    /// Request has already been processed.
    RequestAlreadyProcessed = 37,
    /// Request has expired.
    RequestExpired = 38,
    /// Duplicate request.
    DuplicateRequest = 39,

    // Admin and governance (40-43)
    /// Cannot remove the last admin.
    LastAdminCannotBeRemoved = 40,
    /// Invalid fee token.
    InvalidFeeToken = 41,
    /// Cannot delegate to self.
    CannotDelegateToSelf = 42,
    /// Already approved.
    AlreadyApproved = 43,

    // Council/governance operations (44-46)
    /// Council proposal has already been executed.
    CouncilProposalExecuted = 44,
    /// Timelock period has not elapsed yet.
    TimelockNotReady = 45,
    /// `chunk_size` passed to `set_chunk_size` is 0 or otherwise out of range.
    InvalidChunkSize = 46,

    // Template and registry (47-50)
    /// Claim type is not in the registry when registration is required.
    NotRegisteredClaimType = 47,
    /// Issue #1579: Template ID is empty, too long, or contains disallowed characters.
    InvalidTemplateId = 48,
    /// Issue #1566: A resource with this identifier already exists (e.g., template already exists).
    ConflictError = 50,
    /// Issue #1567: Attestation cannot be disputed (revoked, expired, etc.).
    InvalidAttestation = 51,
    /// Claim type has been deprecated; new attestations cannot be issued for it.
    ClaimTypeDeprecated = 54,
}
