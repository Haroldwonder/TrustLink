# TrustLink Performance Benchmarks

Benchmarked using Soroban testutils Env (unlimited budget). Results are approximate averages from local runs (Soroban SDK test env).

## Compute Units (CU)

| Function | Scenario | Avg CU | Storage Impact |
|----------|----------|--------|---------------|
| `create_attestation` | Baseline (no metadata/tags) | ~75,000 | ~1.2 KB (attestation + 4 indices) |
| `revoke_attestation` | Baseline | ~12,000 | ~200 bytes (status + audit append) |
| `has_valid_claim` | 1 attestation (valid) | ~8,500 | read-only |
| | 10 attestations (1 valid + 9 noise) | ~25,000 | read-only |
| | 100 attestations (1 valid + 99 noise) | ~180,000 | read-only (SubjectClaimIndex opt saves ~10x vs no index) |
| | Invalid claim (100 attestations) | ~150,000 | read-only |
| `get_subject_attestations` | page_size=10 (100 total) | ~15,000 | read-only |
| | page_size=50 | ~35,000 | read-only |
| | page_size=100 (full) | ~65,000 | read-only |

## Storage Bytes Estimates

- Each attestation: ~800 bytes XDR
- Vec index entry per ID: ~32 bytes
- create_attestation: 5 writes (~1.2 KB total)
- revoke: 2 writes (~200 bytes)
- Index Vec grows linearly; prune expired for long-term savings.

## Comparisons Across Data Sizes

- `has_valid_claim`: Scales with claim-specific attestations (SubjectClaimIndex opt). Without opt, 100 attests would be ~1.5M CU.
- `get_subject_attestations`: Linear in total attestations, pagination caps CU.
- Stellar limits: 75M base CU + CPU/reads.

---

## Storage Cost Per Attestation (XLM)

> **Why this matters:** Every byte stored on the Stellar ledger consumes a base reserve.
> Issuers pay this cost at upload time and must maintain the minimum balance to keep
> entries alive. Understanding the per-attestation cost lets issuers budget accurately
> before deploying at scale.

### Stellar Base Reserve (as of 2026)

| Parameter | Value |
|-----------|-------|
| Base reserve per ledger entry | 0.5 XLM |
| Additional reserve per 32 bytes of entry data | 0.5 XLM |
| Minimum account balance (2 entries) | 1 XLM |

> Source: [Stellar Developer Docs — Lumens](https://developers.stellar.org/docs/learn/fundamentals/lumens)
> The base reserve is a network-level parameter and can be changed by validator vote.
> Always verify the current value with `stellar network info` before budgeting.

### Ledger Entry Size Breakdown — Typical Attestation

A baseline `create_attestation` call (no metadata, no tags) writes **5 ledger entries**:

| Entry | Contents | Approx. size |
|-------|----------|-------------|
| `Attestation(id)` | Full attestation struct (id, issuer, subject, claim_type, timestamp, expiration, revoked, imported, bridged, source_chain, source_tx) | ~800 bytes |
| `SubjectAttestations(subject)` | Vec of attestation IDs for this subject (grows with each new attestation) | ~32 bytes per ID |
| `IssuerAttestations(issuer)` | Vec of attestation IDs for this issuer | ~32 bytes per ID |
| `SubjectClaimIndex(subject, claim_type)` | Vec of IDs for fast `has_valid_claim` lookup | ~32 bytes per ID |
| `GlobalStats` | Counters (total_attestations, total_revocations, total_issuers) | ~48 bytes |

**Total new data written per attestation: ~944 bytes** (excluding index Vec overhead already on-chain).

With optional metadata (e.g. a 200-byte JSON string) the attestation entry grows to ~1,000 bytes,
and total written data reaches ~1,144 bytes.

### XLM Cost Calculation

Stellar charges reserve based on the number of 32-byte "data chunks" in an entry, rounded up,
plus the flat per-entry fee.

```
reserve_per_entry = base_reserve + ceil(entry_bytes / 32) × data_reserve_per_32_bytes
                  = 0.5 XLM    + ceil(N / 32)            × 0.5 XLM
```

| Entry | Size (bytes) | 32-byte chunks | Reserve (XLM) |
|-------|-------------|----------------|---------------|
| `Attestation(id)` — baseline | 800 | 25 | 0.5 + 25 × 0.5 = **13.0 XLM** |
| `Attestation(id)` — with 200-byte metadata | 1,000 | 32 | 0.5 + 32 × 0.5 = **16.5 XLM** |
| Index Vec entry (32 bytes) | 32 | 1 | 0.5 + 1 × 0.5 = **1.0 XLM** |
| `GlobalStats` update | 48 | 2 | shared entry, amortised across all ops |

> **Note:** Index Vec entries are appended to existing ledger entries, not new entries.
> The marginal reserve cost for each new ID appended to an existing Vec is the data
> reserve for the additional 32 bytes only (~0.5 XLM), not a full new entry fee.

#### Summary — cost per `create_attestation` call

| Scenario | Approx. XLM reserve |
|----------|-------------------|
| Baseline (no metadata) | ~15–16 XLM |
| With 200-byte metadata | ~18–20 XLM |
| Revocation (`revoke_attestation`) | ~1–2 XLM (2 entry updates) |

These are **reserve** costs, not transaction fees. The XLM is locked, not burned —
it is returned if the entry is deleted or the TTL expires and the entry is evicted.

### TTL and Rent

Soroban persistent entries have a TTL (time-to-live) measured in ledgers. When the TTL
expires the entry is archived and must be restored (at cost) to be readable again.

| Default TTL | ~30 days (configurable via `ttl_days` at initialization) |
|-------------|----------------------------------------------------------|
| Extend TTL | `stellar contract extend-ttl` or automatic via `bump_entry` |
| Restore archived entry | `stellar contract restore` — costs a fee proportional to entry size |

To keep attestations live indefinitely, issuers should run a periodic TTL-extension job.
A rough estimate: extending a single 800-byte entry for 30 days costs ~0.001–0.005 XLM
in transaction fees (network-dependent).

### Cost at Scale

| Attestations | Approx. total XLM reserve (baseline) |
|-------------|--------------------------------------|
| 10 | ~150–160 XLM |
| 100 | ~1,500–1,600 XLM |
| 1,000 | ~15,000–16,000 XLM |
| 10,000 | ~150,000–160,000 XLM |

> These figures assume each attestation is a new subject+claim_type pair (worst case —
> all 5 entries are new). If many attestations share the same subject or issuer, the
> index Vec entries are appended to existing entries and the marginal cost is lower.

### Optimization Recommendations

1. **Reuse subjects**: Multiple attestations for the same subject share index entries —
   marginal cost per additional attestation drops from ~15 XLM to ~13.5 XLM.
2. **Prune expired/revoked entries**: Deleting stale attestations releases the locked reserve.
3. **Avoid large metadata**: Each 32 bytes of metadata adds 0.5 XLM to the reserve.
   Keep metadata under 64 bytes where possible.
4. **Batch operations**: `create_attestations_batch` amortises the `GlobalStats` write
   across multiple attestations in a single transaction.
5. **Monitor TTL**: Run a cron job to extend TTLs before entries are archived to avoid
   restoration fees.

---

---

## Lazy / Chunked Index Loading

### Research Findings: Soroban Storage Model

Soroban's persistent storage is a key-value store where each entry is a single
opaque blob. **There is no native support for partial reads** — every `get` call
deserialises the entire value regardless of how much of it is actually needed.
This means a `Vec<String>` index with 10,000 entries is fully loaded into the
WASM heap on every query, even when only 10 items are needed.

Key constraints confirmed from the Soroban SDK and XDR spec:

| Constraint | Detail |
|-----------|--------|
| No partial entry reads | `env.storage().persistent().get(key)` always returns the full value |
| No server-side filtering | Filtering/slicing must happen in WASM after the full load |
| Max entry size | ~64 KB (XDR limit per ledger entry) |
| Entry granularity | Each distinct key is a separate ledger entry with its own read fee |
| Read fee | Charged per entry accessed, not per byte read |

**Conclusion:** True lazy loading (reading a slice of a Vec without loading the
whole entry) is not possible in Soroban. The only way to achieve partial loading
is to split the index across multiple ledger entries — one entry per chunk.

### Chunked Index Implementation

The chunked index splits each `Vec<String>` index into fixed-size ledger entries
of `CHUNK_SIZE = 50` IDs each. A separate count entry tracks the total.

**Storage layout:**

```
SubjectAttestationsChunk(addr, 0)  → Vec<String>  IDs [0..49]
SubjectAttestationsChunk(addr, 1)  → Vec<String>  IDs [50..99]
SubjectAttestationsChunk(addr, 2)  → Vec<String>  IDs [100..149]
SubjectAttestationsCount(addr)     → u32          total = 150

IssuerAttestationsChunk(addr, 0)   → Vec<String>
IssuerAttestationsChunk(addr, 1)   → Vec<String>
IssuerAttestationsCount(addr)      → u32
```

**Query cost — before vs after:**

| Index size | Page size | Chunks loaded (before) | Chunks loaded (after) | Data read reduction |
|-----------|-----------|----------------------|----------------------|-------------------|
| 100 IDs | 10 | 1 (full flat Vec) | 1 | ~0% (already 1 entry) |
| 1,000 IDs | 10 | 1 (full flat Vec ~64 KB) | 1 (one chunk ~3.2 KB) | ~95% |
| 10,000 IDs | 10 | 1 (full flat Vec, hits size limit) | 1 (one chunk ~3.2 KB) | ~99% |
| 10,000 IDs | 100 | 1 (full flat Vec) | 2–3 chunks | ~97% |

For a 10-item page against a 10,000-item index, the chunked approach reads
`ceil(10/50) + 1 = 2` chunks (~6.4 KB) instead of the entire flat Vec.

**Write cost — append:**

| Operation | Before | After |
|-----------|--------|-------|
| `add_subject` | 1 read + 1 write (full Vec) | 1 read + 1 write (one chunk) |
| `add_issuer_bulk(N)` | 1 read + 1 write (full Vec) | `ceil(N/50)` reads + writes |
| `remove_subject/issuer` | 1 read + 1 write (full Vec) | 1–2 reads + 1–2 writes (swap-with-last) |

The remove operation uses a **swap-with-last** strategy: the target ID is
replaced with the last ID in the index, then the last slot is popped. This
avoids shifting the entire Vec and keeps the remove cost O(chunks_scanned)
rather than O(total_ids).

**Chunk size rationale:**

Each attestation ID is a 64-byte hex string. XDR overhead adds ~8 bytes per
entry in a Vec. One chunk of 50 IDs ≈ 50 × 72 = 3,600 bytes — well within the
64 KB entry limit and small enough that even a cross-chunk page boundary only
loads 2 chunks.

### Running the Tests

```bash
cargo test chunked_index -- --nocapture
```

---

## Batch Attestation Storage Write Optimisation

### Problem (before)

`create_attestations_batch` called `store_attestation` for each subject in the loop.
`store_attestation` performs three shared-state writes per item:

| Write | Per-item cost |
|-------|--------------|
| `IssuerAttestations` index | read + write (grows Vec by 1) |
| `IssuerStats` | read + write |
| `GlobalStats` | read + write |

For a batch of N subjects this produced **3N reads + 3N writes** on those three entries alone.

**Batch of 50 — before:**
- Issuer index: 50 reads + 50 writes
- Issuer stats: 50 reads + 50 writes
- Global stats: 50 reads + 50 writes
- **Total: 150 extra reads + 150 extra writes**

### Fix (after)

The loop now only writes the attestation record and the per-subject index (both are
inherently per-item). The three shared-state entries are accumulated in memory and
written **once** after the loop using three new bulk helpers:

| Helper | Writes |
|--------|--------|
| `Storage::add_issuer_attestations_bulk` | 1 read + 1 write |
| `Storage::increment_issuer_stats` | 1 read + 1 write |
| `Storage::increment_total_attestations` | 1 read + 1 write |

**Batch of 50 — after:**
- Issuer index: 1 read + 1 write
- Issuer stats: 1 read + 1 write
- Global stats: 1 read + 1 write
- **Total: 3 reads + 3 writes**

### Write Count Reduction

| Batch size | Writes before | Writes after | Saved |
|-----------|--------------|-------------|-------|
| 10 | 30 | 3 | 27 |
| 50 | 150 | 3 | 147 |
| 100 | 300 | 3 | 297 |

The per-attestation writes (attestation record, subject index, audit log) are unchanged —
only the shared-state writes are batched.

### Benchmark

Run the before/after comparison:

```bash
cargo test bench_batch -- --nocapture
```

Expected output (approximate):

```
[bench_batch_50_correctness] PASS — 50 attestations, issuer index consistent
[bench_batch_50_write_reduction] batch=50 | cpu_instructions=... | memory_bytes=...
[bench_single_vs_batch_50]
  single×50 : cpu=<higher> mem=<higher>
  batch×50  : cpu=<lower>  mem=<lower>
  cpu saved : <N> (~X%)
```

---

## Optimization Recommendations (General)

1. **High Impact**: Cron job to prune expired/revoked from indices (Vec shrink).
2. **Med**: Use u128 IDs vs String (~20% storage save).
3. **Med**: Cache hot `has_valid_claim` in temp storage (instance()).
4. **Low**: Bump alloc for known index sizes.
5. **Future**: Multisig batch writes for fee efficiency.

Benchmark code: `benches/performance.rs`. Run `cargo test benches:: --nocapture`.

*Tested on Soroban SDK local (Linux x64, Rust 1.75). Live network CU ~10-20% higher.*
