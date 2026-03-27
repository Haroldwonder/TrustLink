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
    /// An attestation request has already been fulfilled or rejected.
    RequestAlreadyProcessed = 25,
    /// The attestation request has expired without being acted on.
    RequestExpired = 26,
    /// A duplicate request already exists for this subject/issuer/claim_type combination.
    DuplicateRequest = 27,
    /// The claim type string is empty, too long, or contains invalid characters.
    InvalidClaimType = 28,
}
