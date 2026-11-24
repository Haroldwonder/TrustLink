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

### `useHasValidClaim(subject: string, claimType: string, options?: HasValidClaimOptions)`

Checks whether `subject` holds a valid (non-revoked, non-expired) claim of `claimType`. This is the most common way to gate UI on a claim.

```tsx
import { useHasValidClaim } from "@trustlink/react";

function PremiumFeature({ subject, checkClaim }: {
  subject: string;
  checkClaim: (subject: string, claimType: string) => Promise<boolean>;
}) {
  const { data: hasClaim, loading, error, refetch } = useHasValidClaim(
    subject,
    "KYC_VERIFIED",
    { checkClaim, pollIntervalMs: 30_000 },
  );

  if (loading) return <p>Checking claim…</p>;
  if (error) return <p>Error: {error.message}</p>;

  return hasClaim ? (
    <PremiumContent onRefresh={refetch} />
  ) : (
    <p>You need a verified KYC claim to access this feature.</p>
  );
}
```

Returns `{ data: boolean | null, loading, error, refetch }`:

- `data` — `true` when the subject has a valid claim of `claimType`, `false` otherwise, `null` before the first result.
- `loading` — `true` while a check is in flight.
- `error` — the last error, or `null`.
- `refetch` — manually re-run the check (stable reference).

`HasValidClaimOptions`:

```ts
interface HasValidClaimOptions {
  /** Fetcher that queries your RPC client, e.g. (s, c) => client.hasValidClaim(s, c). */
  checkClaim: (subject: string, claimType: string) => Promise<boolean>;
  /** Skip the check entirely (e.g. until a wallet is connected). Default: true. */
  enabled?: boolean;
  /** Poll for changes at this interval in milliseconds. Omit to disable polling. */
  pollIntervalMs?: number;
}
```

The hook only re-checks when `subject`, `claimType`, `enabled`, `pollIntervalMs` or the `checkClaim` reference changes, so memoise `checkClaim` with `useCallback` to avoid refetch loops.
