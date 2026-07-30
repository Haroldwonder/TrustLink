/**
 * W3C Verifiable Credential export helpers for TrustLink attestations.
 *
 * Maps on-chain attestation fields onto a near-conformant W3C VC Data Model
 * 1.1 JSON document suitable for off-chain wallets and verifiers.
 *
 * ## Field mapping
 *
 * | TrustLink field       | VC property                                      |
 * |-----------------------|--------------------------------------------------|
 * | `id`                  | `id` (`urn:trustlink:attestation:<id>`)          |
 * | `issuer`              | `issuer`                                         |
 * | `subject`             | `credentialSubject.id`                           |
 * | `claim_type`          | `type[2]` + `credentialSubject.claimType`        |
 * | `timestamp`           | `issuanceDate` (ISO-8601)                        |
 * | `expiration`          | `expirationDate` (omitted when null)             |
 * | `revoked`             | `credentialStatus.revoked`                       |
 * | `revocation_reason`   | `credentialStatus.revocationReason` (if set)     |
 * | `valid_from`          | `credentialSubject.validFrom` (ISO-8601; see below) |
 * | `metadata`            | `credentialSubject.metadata` (if set)            |
 *
 * ## Unsupported / non-equivalent fields
 *
 * These TrustLink fields have no direct W3C VC 1.1 equivalent. They are
 * preserved under `credentialSubject.trustlink` for round-tripping, but
 * standard VC verifiers will ignore them:
 *
 * - `origin` — TrustLink provenance enum (`Native` / `Imported` / `Bridged`)
 * - `source_chain` / `source_tx` — cross-chain bridge references
 * - `tags` — free-form TrustLink tags (not VC credential schema claims)
 * - `jurisdiction` — regulatory jurisdiction hint
 * - `deleted` — soft-delete flag (distinct from revocation / credentialStatus)
 *
 * Additional limitations:
 *
 * - **No `proof`**: the export is an unsigned credential document. Linked-data
 *   or JWT proofs must be added by the wallet / issuer tooling that holds keys.
 * - **Issuer is a Stellar address**, not a DID (`did:…`). Consumers that require
 *   DIDs should wrap or resolve the address out-of-band.
 * - **`valid_from`**: VC Data Model 1.1 has no `validFrom`; the value is placed
 *   on `credentialSubject.validFrom`. VC 2.0 consumers may remap it to the
 *   top-level `validFrom` property themselves.
 * - **`credentialStatus`**: uses a TrustLink-specific status type
 *   (`TrustLinkRevocationStatus`), not BitstringStatusList / StatusList2021.
 *   Poll the TrustLink contract (or indexer) for live revocation state.
 */

import type { Attestation } from "./types.js";

/** W3C VC Data Model 1.1 `@context` entry. */
export const VC_CONTEXT_V1 = "https://www.w3.org/2018/credentials/v1";

/** TrustLink-specific JSON-LD context URI for extension terms. */
export const TRUSTLINK_VC_CONTEXT = "https://trustlink.io/credentials/v1";

/** Credential status type used for TrustLink revocation state. */
export const TRUSTLINK_REVOCATION_STATUS_TYPE = "TrustLinkRevocationStatus";

/**
 * TrustLink-specific fields nested under `credentialSubject.trustlink`.
 * These have no W3C VC 1.1 equivalent (see module docs).
 */
export interface TrustLinkVcExtension {
  origin: Attestation["origin"];
  sourceChain: string | null;
  sourceTx: string | null;
  tags: string[] | null;
  jurisdiction: string | null;
  deleted: boolean;
  /** Original on-chain attestation id (also encoded in the VC `id`). */
  attestationId: string;
}

/**
 * Credential subject produced by {@link toVerifiableCredential}.
 */
export interface TrustLinkCredentialSubject {
  id: string;
  claimType: string;
  /** ISO-8601 instant from `valid_from`, when set. */
  validFrom?: string;
  metadata?: string;
  trustlink: TrustLinkVcExtension;
}

/**
 * TrustLink-specific credential status entry.
 */
export interface TrustLinkCredentialStatus {
  id: string;
  type: typeof TRUSTLINK_REVOCATION_STATUS_TYPE;
  revoked: boolean;
  revocationReason?: string;
}

/**
 * Near-conformant W3C Verifiable Credential (Data Model 1.1) document
 * derived from a TrustLink attestation. Unsigned — no `proof` property.
 */
export interface VerifiableCredential {
  "@context": string[];
  id: string;
  type: string[];
  issuer: string;
  issuanceDate: string;
  expirationDate?: string;
  credentialSubject: TrustLinkCredentialSubject;
  credentialStatus: TrustLinkCredentialStatus;
}

export interface ToVerifiableCredentialOptions {
  /**
   * Override the default `@context` array.
   * Defaults to `[VC_CONTEXT_V1, TRUSTLINK_VC_CONTEXT]`.
   */
  context?: string[];
  /**
   * Optional URI prefix for the credential `id`.
   * Defaults to `urn:trustlink:attestation:`.
   */
  idPrefix?: string;
}

/**
 * Convert a Unix-seconds timestamp (TrustLink on-chain convention) to an
 * ISO-8601 UTC string suitable for VC date properties.
 */
export function unixSecondsToIso(seconds: bigint | number): string {
  const ms = Number(seconds) * 1000;
  if (!Number.isFinite(ms)) {
    throw new RangeError(`Invalid Unix timestamp: ${seconds}`);
  }
  return new Date(ms).toISOString();
}

/**
 * Map a TrustLink {@link Attestation} to a W3C Verifiable Credential document.
 *
 * The result is intentionally unsigned (no `proof`). See the module
 * documentation for the field mapping and known non-equivalent fields.
 */
export function toVerifiableCredential(
  attestation: Attestation,
  options: ToVerifiableCredentialOptions = {}
): VerifiableCredential {
  if (!attestation.id) {
    throw new Error("Attestation id is required to build a Verifiable Credential");
  }
  if (!attestation.issuer) {
    throw new Error("Attestation issuer is required to build a Verifiable Credential");
  }
  if (!attestation.subject) {
    throw new Error("Attestation subject is required to build a Verifiable Credential");
  }
  if (!attestation.claim_type) {
    throw new Error("Attestation claim_type is required to build a Verifiable Credential");
  }

  const idPrefix = options.idPrefix ?? "urn:trustlink:attestation:";
  const credentialId = `${idPrefix}${attestation.id}`;

  const credentialSubject: TrustLinkCredentialSubject = {
    id: attestation.subject,
    claimType: attestation.claim_type,
    trustlink: {
      origin: attestation.origin,
      sourceChain: attestation.source_chain,
      sourceTx: attestation.source_tx,
      tags: attestation.tags,
      jurisdiction: attestation.jurisdiction,
      deleted: attestation.deleted,
      attestationId: attestation.id,
    },
  };

  if (attestation.valid_from != null) {
    credentialSubject.validFrom = unixSecondsToIso(attestation.valid_from);
  }
  if (attestation.metadata != null) {
    credentialSubject.metadata = attestation.metadata;
  }

  const credentialStatus: TrustLinkCredentialStatus = {
    id: `${credentialId}#status`,
    type: TRUSTLINK_REVOCATION_STATUS_TYPE,
    revoked: attestation.revoked,
  };
  if (attestation.revocation_reason != null) {
    credentialStatus.revocationReason = attestation.revocation_reason;
  }

  const vc: VerifiableCredential = {
    "@context": options.context ?? [VC_CONTEXT_V1, TRUSTLINK_VC_CONTEXT],
    id: credentialId,
    type: ["VerifiableCredential", "TrustLinkAttestation", attestation.claim_type],
    issuer: attestation.issuer,
    issuanceDate: unixSecondsToIso(attestation.timestamp),
    credentialSubject,
    credentialStatus,
  };

  if (attestation.expiration != null) {
    vc.expirationDate = unixSecondsToIso(attestation.expiration);
  }

  return vc;
}
