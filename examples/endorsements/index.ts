import { TrustLinkClient } from '@trustlink/sdk';

/**
 * Endorsements and Confidence Scores Example
 *
 * This example demonstrates two distinctive TrustLink features:
 *
 * 1. **Endorsements** — Independent issuers can endorse an attestation
 *    created by another issuer, increasing its trust weight.
 *
 * 2. **Confidence Scores** — The verifier can query a numeric confidence
 *    score for an attestation. The score increases as more issuers endorse
 *    it, and decays over time according to the configured decay parameters.
 *
 * Decay configuration:
 *   - `decayRate` controls how quickly an endorsement's weight fades.
 *   - `decayInterval` (in seconds) is the period over which decay is applied.
 *   - `halfLife` (in seconds) is an alternative way to express decay: the
 *     time it takes for an endorsement's contribution to be halved.
 *   Endorsements made recently count more than older ones, so confidence
 *   scores reflect both the number and the freshness of endorsements.
 */

async function main() {
  // Connect to the TrustLink network.
  const client = new TrustLinkClient({
    network: 'testnet',
    // Decay configuration for confidence scoring.
    confidence: {
      // Fraction of weight lost per decay interval (0.05 = 5%).
      decayRate: 0.05,
      // Decay is applied every 24 hours (in seconds).
      decayInterval: 24 * 60 * 60,
      // Alternatively, express decay as a half-life of 30 days.
      halfLife: 30 * 24 * 60 * 60,
    },
  });

  // Three independent issuers.
  const issuerA = await client.createIssuer({ name: 'Issuer A' });
  const issuerB = await client.createIssuer({ name: 'Issuer B' });
  const issuerC = await client.createIssuer({ name: 'Issuer C' });

  // A subject whose attestation will be endorsed.
  const subject = await client.createSubject({ name: 'Subject' });

  // Issuer A creates an attestation about the subject.
  const attestation = await issuerA.createAttestation({
    subject: subject.id,
    claim: 'completed-kyc',
    data: { level: 'full' },
  });

  console.log(`Attestation created by Issuer A: ${attestation.id}`);

  // The verifier reads the confidence score before any endorsements.
  const beforeScore = await client.getConfidenceScore(attestation.id);
  console.log(`Confidence score before endorsements: ${beforeScore}`);

  // Issuers B and C each endorse Issuer A's attestation.
  const endorsementB = await issuerB.endorseAttestation(attestation.id);
  console.log(`Issuer B endorsed the attestation: ${endorsementB.id}`);

  const endorsementC = await issuerC.endorseAttestation(attestation.id);
  console.log(`Issuer C endorsed the attestation: ${endorsementC.id}`);

  // The verifier re-reads the confidence score after the endorsements.
  const afterScore = await client.getConfidenceScore(attestation.id);
  console.log(`Confidence score after endorsements: ${afterScore}`);

  // Report the number of endorsements on the attestation.
  const endorsements = await client.getEndorsements(attestation.id);
  console.log(`Endorsement count: ${endorsements.length}`);

  console.log(
    `Confidence increased from ${beforeScore} to ${afterScore} ` +
      `after ${endorsements.length} endorsements.`,
  );
}

main().catch((error) => {
  console.error('Error running endorsements example:', error);
  process.exit(1);
});