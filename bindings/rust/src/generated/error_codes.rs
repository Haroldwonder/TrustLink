//! GENERATED FILE — DO NOT EDIT BY HAND.
//!
//! Run `node scripts/generate-error-codes.mjs` (or `make generate`) to regenerate.
//! Source of truth: src/errors.rs

use serde::{Deserialize, Serialize};

/// Contract-level error codes that map directly to the on-chain `Error` enum.
///
/// `Unknown` is not a contract error: it is the fallback for a code this
/// binding does not recognise, which happens when the contract adds a variant
/// before the bindings are regenerated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum ContractErrorCode {
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
    /// Source reference string is missing or empty.
    InvalidSourceReference = 44,
    /// `chunk_size` passed to `set_chunk_size` is 0 or otherwise out of range.
    InvalidChunkSize = 45,
    /// Claim type is not in the registry when registration is required.
    NotRegisteredClaimType = 46,
    /// The caller's `expected_version` argument does not match the contract's currently deployed version, as returned by `get_version()`. Returned by state-changing entry points that opt into the version-guard pattern documented on [`crate::validation::Validation::require_version_match`].
    VersionMismatch = 47,
    /// Feature not yet implemented.
    NotImplemented = 48,
    /// Unrecognised contract error code.
    Unknown = 99,
}

impl From<u32> for ContractErrorCode {
    fn from(code: u32) -> Self {
        match code {
            1 => Self::AlreadyInitialized,
            2 => Self::NotInitialized,
            3 => Self::Unauthorized,
            4 => Self::NotFound,
            5 => Self::DuplicateAttestation,
            6 => Self::AlreadyRevoked,
            7 => Self::Expired,
            8 => Self::InvalidValidFrom,
            9 => Self::InvalidExpiration,
            10 => Self::MetadataTooLong,
            11 => Self::InvalidTimestamp,
            12 => Self::InvalidFee,
            13 => Self::FeeTokenRequired,
            14 => Self::TooManyTags,
            15 => Self::TagTooLong,
            16 => Self::InvalidThreshold,
            17 => Self::NotRequiredSigner,
            18 => Self::AlreadySigned,
            19 => Self::ProposalFinalized,
            20 => Self::ProposalExpired,
            21 => Self::ReasonTooLong,
            22 => Self::CannotEndorseOwn,
            23 => Self::AlreadyEndorsed,
            24 => Self::ContractPaused,
            25 => Self::SubjectNotWhitelisted,
            26 => Self::InvalidClaimType,
            27 => Self::InvalidJurisdiction,
            28 => Self::RateLimited,
            29 => Self::LimitExceeded,
            30 => Self::ProposalCancelled,
            31 => Self::AlreadyDisputed,
            32 => Self::InvalidMetadata,
            33 => Self::ConstraintViolation,
            34 => Self::RequestAlreadyProcessed,
            35 => Self::RequestExpired,
            36 => Self::DuplicateRequest,
            37 => Self::CouncilProposalExecuted,
            38 => Self::TimelockNotReady,
            39 => Self::NotDisputed,
            40 => Self::LastAdminCannotBeRemoved,
            41 => Self::InvalidFeeToken,
            42 => Self::CannotDelegateToSelf,
            43 => Self::AlreadyApproved,
            44 => Self::InvalidSourceReference,
            45 => Self::InvalidChunkSize,
            46 => Self::NotRegisteredClaimType,
            47 => Self::VersionMismatch,
            48 => Self::NotImplemented,
            _ => Self::Unknown,
        }
    }
}

impl ContractErrorCode {
    /// The numeric code as reported by the contract.
    pub fn code(self) -> u32 {
        self as u32
    }

    /// The variant name, matching the contract's `Error` enum.
    pub fn name(self) -> &'static str {
        match self {
            Self::AlreadyInitialized => "AlreadyInitialized",
            Self::NotInitialized => "NotInitialized",
            Self::Unauthorized => "Unauthorized",
            Self::NotFound => "NotFound",
            Self::DuplicateAttestation => "DuplicateAttestation",
            Self::AlreadyRevoked => "AlreadyRevoked",
            Self::Expired => "Expired",
            Self::InvalidValidFrom => "InvalidValidFrom",
            Self::InvalidExpiration => "InvalidExpiration",
            Self::MetadataTooLong => "MetadataTooLong",
            Self::InvalidTimestamp => "InvalidTimestamp",
            Self::InvalidFee => "InvalidFee",
            Self::FeeTokenRequired => "FeeTokenRequired",
            Self::TooManyTags => "TooManyTags",
            Self::TagTooLong => "TagTooLong",
            Self::InvalidThreshold => "InvalidThreshold",
            Self::NotRequiredSigner => "NotRequiredSigner",
            Self::AlreadySigned => "AlreadySigned",
            Self::ProposalFinalized => "ProposalFinalized",
            Self::ProposalExpired => "ProposalExpired",
            Self::ReasonTooLong => "ReasonTooLong",
            Self::CannotEndorseOwn => "CannotEndorseOwn",
            Self::AlreadyEndorsed => "AlreadyEndorsed",
            Self::ContractPaused => "ContractPaused",
            Self::SubjectNotWhitelisted => "SubjectNotWhitelisted",
            Self::InvalidClaimType => "InvalidClaimType",
            Self::InvalidJurisdiction => "InvalidJurisdiction",
            Self::RateLimited => "RateLimited",
            Self::LimitExceeded => "LimitExceeded",
            Self::ProposalCancelled => "ProposalCancelled",
            Self::AlreadyDisputed => "AlreadyDisputed",
            Self::InvalidMetadata => "InvalidMetadata",
            Self::ConstraintViolation => "ConstraintViolation",
            Self::RequestAlreadyProcessed => "RequestAlreadyProcessed",
            Self::RequestExpired => "RequestExpired",
            Self::DuplicateRequest => "DuplicateRequest",
            Self::CouncilProposalExecuted => "CouncilProposalExecuted",
            Self::TimelockNotReady => "TimelockNotReady",
            Self::NotDisputed => "NotDisputed",
            Self::LastAdminCannotBeRemoved => "LastAdminCannotBeRemoved",
            Self::InvalidFeeToken => "InvalidFeeToken",
            Self::CannotDelegateToSelf => "CannotDelegateToSelf",
            Self::AlreadyApproved => "AlreadyApproved",
            Self::InvalidSourceReference => "InvalidSourceReference",
            Self::InvalidChunkSize => "InvalidChunkSize",
            Self::NotRegisteredClaimType => "NotRegisteredClaimType",
            Self::VersionMismatch => "VersionMismatch",
            Self::NotImplemented => "NotImplemented",
            Self::Unknown => "Unknown",
        }
    }
}

impl core::fmt::Display for ContractErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} (#{})", self.name(), self.code())
    }
}
