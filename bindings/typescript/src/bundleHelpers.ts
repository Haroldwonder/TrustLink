/**
 * TrustLink Bundle Helpers — TypeScript SDK
 *
 * Helper methods for creating and verifying attestation bundles.
 * Bundles allow issuing multiple related attestations atomically with a shared bundle ID.
 */

import { TrustLinkClient } from "./client";
import type { Attestation, AttestationBundle } from "./types";

/**
 * Options for creating an attestation bundle.
 */
export interface CreateBundleOptions {
  /** Issuer address (must be authorized) */
  issuer: string;
  /** Subject address receiving the attestations */
  subject: string;
  /** List of claim types to issue (order determines bundle ID) */
  claimTypes: string[];
  /** Optional expiration time (applied to all attestations) */
  expiration?: bigint;
  /** Optional metadata (shared across attestations in bundle) */
  metadata?: string;
  /** Optional tags (applied to all attestations) */
  tags?: string[];
}

/**
 * Helper to create a bundle of attestations with a shared bundle ID.
 *
 * @param client - TrustLink client instance
 * @param options - Bundle creation options
 * @returns The bundle ID on success
 *
 * @example
 * ```typescript
 * const bundleId = await createAttestationBundle(client, {
 *   issuer: issuerAddress,
 *   subject: subjectAddress,
 *   claimTypes: ["KYC_PASSED", "AGE_VERIFIED", "JURISDICTION_US"],
 *   expiration: BigInt(Math.floor(Date.now() / 1000)) + BigInt(365 * 24 * 60 * 60),
 * });
 * ```
 */
export async function createAttestationBundle(
  client: TrustLinkClient,
  options: CreateBundleOptions
): Promise<string> {
  return client.createAttestationBundle(
    options.issuer,
    options.subject,
    options.claimTypes,
    options.expiration ?? null,
    options.metadata ?? null,
    options.tags ?? null
  );
}

/**
 * Retrieve a bundle and all its attestations.
 *
 * @param client - TrustLink client instance
 * @param bundleId - The bundle ID to retrieve
 * @returns Object with bundle metadata and attestations
 *
 * @example
 * ```typescript
 * const bundleInfo = await getBundleWithAttestations(client, bundleId);
 * console.log(`Bundle has ${bundleInfo.attestations.length} attestations`);
 * console.log(`All valid: ${bundleInfo.bundle.all_valid}`);
 * ```
 */
export async function getBundleWithAttestations(
  client: TrustLinkClient,
  bundleId: string
): Promise<{ bundle: AttestationBundle; attestations: Attestation[] }> {
  const [bundle, attestations] = await Promise.all([
    client.getBundle(bundleId),
    client.getBundleAttestations(bundleId),
  ]);

  return { bundle, attestations };
}

/**
 * Check if a bundle and all its attestations are valid (not revoked).
 *
 * @param client - TrustLink client instance
 * @param bundleId - The bundle ID to check
 * @returns True if all attestations in the bundle are valid
 *
 * @example
 * ```typescript
 * const isValid = await verifyBundleValidity(client, bundleId);
 * if (isValid) {
 *   console.log("Bundle is fully valid");
 * } else {
 *   console.log("One or more attestations in bundle have been revoked");
 * }
 * ```
 */
export async function verifyBundleValidity(
  client: TrustLinkClient,
  bundleId: string
): Promise<boolean> {
  return client.isBundleValid(bundleId);
}

/**
 * Get all bundles created by an issuer.
 *
 * @param client - TrustLink client instance
 * @param issuer - Issuer address
 * @returns Array of bundle IDs created by the issuer
 *
 * @example
 * ```typescript
 * const bundleIds = await getIssuerBundles(client, issuerAddress);
 * console.log(`Issuer has created ${bundleIds.length} bundles`);
 * ```
 */
export async function getIssuerBundles(
  client: TrustLinkClient,
  issuer: string
): Promise<string[]> {
  return client.getIssuerBundles(issuer);
}

/**
 * Get all bundles issued to a subject.
 *
 * @param client - TrustLink client instance
 * @param subject - Subject address
 * @returns Array of bundle IDs issued to the subject
 *
 * @example
 * ```typescript
 * const bundleIds = await getSubjectBundles(client, subjectAddress);
 * console.log(`Subject has received ${bundleIds.length} bundles`);
 * ```
 */
export async function getSubjectBundles(
  client: TrustLinkClient,
  subject: string
): Promise<string[]> {
  return client.getSubjectBundles(subject);
}

/**
 * Verify that a set of attestations belong to the same bundle.
 *
 * @param attestations - Array of attestations to verify
 * @returns The common bundle ID if all share one, or null if not all in same bundle
 *
 * @example
 * ```typescript
 * const bundleId = verifyAttestationsInSameBundle(attestations);
 * if (bundleId) {
 *   console.log(`All attestations are from bundle: ${bundleId}`);
 * } else {
 *   console.log("Attestations are not all from the same bundle");
 * }
 * ```
 */
export function verifyAttestationsInSameBundle(
  attestations: Attestation[]
): string | null {
  if (attestations.length === 0) return null;

  const firstBundleId = attestations[0].bundle_id;
  if (!firstBundleId) return null;

  for (const attestation of attestations) {
    if (attestation.bundle_id !== firstBundleId) {
      return null;
    }
  }

  return firstBundleId;
}

/**
 * Verify a bundle contains expected claim types.
 *
 * @param bundle - The bundle to verify
 * @param expectedClaimTypes - Expected claim types (order matters)
 * @returns True if bundle contains exactly the expected claim types in order
 *
 * @example
 * ```typescript
 * const isExpected = verifyBundleClaimTypes(bundle, ["KYC_PASSED", "AGE_VERIFIED"]);
 * console.log(`Bundle has expected claim types: ${isExpected}`);
 * ```
 */
export function verifyBundleClaimTypes(
  bundle: AttestationBundle,
  expectedClaimTypes: string[]
): boolean {
  if (bundle.claim_types.length !== expectedClaimTypes.length) return false;

  for (let i = 0; i < bundle.claim_types.length; i++) {
    if (bundle.claim_types[i] !== expectedClaimTypes[i]) return false;
  }

  return true;
}

/**
 * Verify a bundle contains expected number of attestations.
 *
 * @param bundle - The bundle to verify
 * @param expectedCount - Expected number of attestations
 * @returns True if bundle has exactly the expected number of attestations
 *
 * @example
 * ```typescript
 * const isExpected = verifyBundleSize(bundle, 3);
 * console.log(`Bundle has expected size: ${isExpected}`);
 * ```
 */
export function verifyBundleSize(bundle: AttestationBundle, expectedCount: number): boolean {
  return (
    bundle.attestation_ids.length === expectedCount &&
    bundle.claim_types.length === expectedCount
  );
}
