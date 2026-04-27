# @trustlink/react

React hooks for the [TrustLink](https://github.com/afurious/TrustLink) on-chain attestation contract on Stellar.

## Installation

```bash
npm install @trustlink/react @trustlink/sdk
```

## Usage

```tsx
import { useTrustLink, useHasValidClaim, useSubjectAttestations } from "@trustlink/react";

const CONTRACT_ID = "C...";

function KycGate({ subject }: { subject: string }) {
  const client = useTrustLink({ contractId: CONTRACT_ID, network: "testnet" });
  const { data: valid, loading, error } = useHasValidClaim(client, subject, "kyc_passed");

  if (loading) return <p>Checking…</p>;
  if (error)   return <p>Error: {error.message}</p>;
  return <p>KYC: {valid ? "✅ passed" : "❌ not passed"}</p>;
}

function AttestationList({ subject }: { subject: string }) {
  const client = useTrustLink({ contractId: CONTRACT_ID, network: "testnet" });
  const { data: attestations, loading } = useSubjectAttestations(client, subject);

  if (loading) return <p>Loading…</p>;
  return (
    <ul>
      {attestations?.map((a) => (
        <li key={a.id}>{a.claim_type} — {a.revoked ? "revoked" : "active"}</li>
      ))}
    </ul>
  );
}
```

## API

### `useTrustLink(options: TrustLinkClientOptions): TrustLinkClient`

Creates (and memoises) a `TrustLinkClient`. Pass the result to the other hooks.

### `useHasValidClaim(client, subject, claimType)`

Returns `{ data: boolean | null, loading, error, refetch }`.

### `useSubjectAttestations(client, subject, { start?, limit? })`

Returns `{ data: Attestation[] | null, loading, error, refetch }`.  
Defaults: `start = 0`, `limit = 50`.
