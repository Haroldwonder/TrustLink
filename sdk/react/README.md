# @trustlink/react

React hooks for the [TrustLink](https://github.com/Haroldwonder/TrustLink) on-chain attestation contract on Stellar.

## Installation

```bash
npm install @trustlink/react
```

## Usage

```tsx
import { useGlobalStats, useIssuerStats } from "@trustlink/react";

function StatsPanel({ fetchGlobalStats, fetchIssuerStats, issuer }: {
  fetchGlobalStats: () => Promise<{ total_attestations: number; total_revocations: number; total_issuers: number }>;
  fetchIssuerStats: (issuer: string) => Promise<{ total_issued: number; active: number; revoked: number; expired: number }>;
  issuer: string;
}) {
  const { data: global, loading: globalLoading, error: globalError } = useGlobalStats(fetchGlobalStats);
  const { data: stats, loading, error } = useIssuerStats(issuer, fetchIssuerStats);

  if (globalLoading || loading) return <p>Loading…</p>;
  if (globalError) return <p>Error: {globalError.message}</p>;
  if (error) return <p>Error: {error.message}</p>;

  return (
    <div>
      <p>Total attestations: {global?.total_attestations}</p>
      <p>Issuer active: {stats?.active}</p>
    </div>
  );
}
```

Pass a bound or arrow function that calls your RPC client (e.g. `() => trustlinkClient.getGlobalStats()`). Memoise the fetcher with `useCallback` if the parent re-renders often, since the hooks re-fetch when the fetcher reference changes.

## API

### `useGlobalStats(fetchStats: () => Promise<GlobalStats>)`

Fetches contract-wide global statistics. Mirrors the `get_global_stats` contract function.

Returns `{ data: GlobalStats | null, loading, error }`.

`GlobalStats`:

```ts
interface GlobalStats {
  total_attestations: number;
  total_revocations: number;
  total_issuers: number;
}
```

### `useIssuerStats(issuer: string, fetchStats: (issuer: string) => Promise<IssuerStats>)`

Fetches statistics for the given issuer address. Re-fetches when `issuer` or the `fetchStats` reference changes.

Returns `{ data: IssuerStats | null, loading, error }`.

`IssuerStats`:

```ts
interface IssuerStats {
  total_issued: number;
  active: number;
  revoked: number;
  expired: number;
}
```
