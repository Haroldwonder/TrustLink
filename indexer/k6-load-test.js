import http from 'k6/http';
import { check, sleep } from 'k6';

const BASE_URL = __ENV.BASE_URL || 'http://localhost:4000/graphql';

export const options = {
  // Performance testing with increasing load
  stages: [
    { duration: '30s', target: 10 },   // Ramp up to 10 VUs
    { duration: '1m', target: 50 },    // Ramp up to 50 VUs
    { duration: '1m30s', target: 100 }, // Ramp up to 100 VUs
    { duration: '2m', target: 100 },   // Stay at 100 VUs
    { duration: '30s', target: 0 },    // Ramp down to 0
  ],
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.05'],
  },
};

// Query: List attestations paginated
const LIST_ATTESTATIONS_QUERY = `
  query ListAttestations($limit: Int!, $offset: Int!) {
    attestations(limit: $limit, offset: $offset) {
      id
      issuer
      subject
      claimType
      timestamp
      isRevoked
      metadata
    }
  }
`;

// Query: Get issuer statistics
const ISSUER_STATS_QUERY = `
  query IssuerStats($issuer: String!) {
    issuer(address: $issuer) {
      address
      totalIssued
      active
      revoked
    }
  }
`;

// Query: Subject claims
const SUBJECT_CLAIMS_QUERY = `
  query SubjectClaims($subject: String!) {
    subject(address: $subject) {
      address
      claims {
        claimType
        count
        hasValid
      }
    }
  }
`;

// Query: Search attestations by date range
const ATTESTATIONS_IN_RANGE_QUERY = `
  query AttestationsInRange($subject: String!, $from: Int!, $to: Int!, $limit: Int!) {
    attestationsInRange(subject: $subject, fromTimestamp: $from, toTimestamp: $to, limit: $limit) {
      id
      claimType
      timestamp
      isRevoked
    }
  }
`;

// Mock data generators
function randomAddress() {
  return 'G' + Math.random().toString(36).substring(2, 56).toUpperCase();
}

function randomTimestamp() {
  return Math.floor(Date.now() / 1000) - Math.floor(Math.random() * 2592000); // Last 30 days
}

export default function () {
  const headers = {
    'Content-Type': 'application/json',
  };

  // Test 1: List attestations with pagination
  let res = http.post(
    BASE_URL,
    JSON.stringify({
      query: LIST_ATTESTATIONS_QUERY,
      variables: { limit: 10, offset: 0 },
    }),
    { headers }
  );
  check(res, {
    'list attestations: status 200': (r) => r.status === 200,
    'list attestations: response time < 200ms': (r) => r.timings.duration < 200,
    'list attestations: no errors': (r) => !r.body.includes('errors'),
  });

  sleep(0.1);

  // Test 2: Get issuer statistics
  const issuer = randomAddress();
  res = http.post(
    BASE_URL,
    JSON.stringify({
      query: ISSUER_STATS_QUERY,
      variables: { issuer },
    }),
    { headers }
  );
  check(res, {
    'issuer stats: status 200': (r) => r.status === 200,
    'issuer stats: response time < 200ms': (r) => r.timings.duration < 200,
    'issuer stats: no errors': (r) => !r.body.includes('errors'),
  });

  sleep(0.1);

  // Test 3: Subject claims
  const subject = randomAddress();
  res = http.post(
    BASE_URL,
    JSON.stringify({
      query: SUBJECT_CLAIMS_QUERY,
      variables: { subject },
    }),
    { headers }
  );
  check(res, {
    'subject claims: status 200': (r) => r.status === 200,
    'subject claims: response time < 200ms': (r) => r.timings.duration < 200,
    'subject claims: no errors': (r) => !r.body.includes('errors'),
  });

  sleep(0.1);

  // Test 4: Attestations in date range
  const from = randomTimestamp();
  const to = from + 1296000; // 15 days range
  res = http.post(
    BASE_URL,
    JSON.stringify({
      query: ATTESTATIONS_IN_RANGE_QUERY,
      variables: { subject, from, to, limit: 20 },
    }),
    { headers }
  );
  check(res, {
    'attestations in range: status 200': (r) => r.status === 200,
    'attestations in range: response time < 500ms': (r) => r.timings.duration < 500,
    'attestations in range: no errors': (r) => !r.body.includes('errors'),
  });

  sleep(0.5);
}
