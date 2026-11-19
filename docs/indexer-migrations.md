# Indexer Postgres Schema Migrations (Zero-Downtime)

This runbook covers rolling out a schema change (new column, table, or index) to a **live** TrustLink indexer deployment without downtime. It complements [disaster-recovery.md](./disaster-recovery.md) (backup/restore) and does not replace it — always take a backup before a production migration.

The indexer schema has grown to cover attestations, multisig proposals, requests, templates, delegations, whitelist entries, council actions, audit log, and webhook failures. Every new migration must stay **backward-compatible with the currently-deployed indexer binary for at least one release**.

---

## Principles: expand / contract

Never ship a single migration that both changes shape *and* requires the new binary simultaneously. Split work into two releases:

| Phase | Schema change | Indexer binary | Safe? |
|-------|---------------|----------------|-------|
| **Expand** | Additive only: `CREATE TABLE`, `CREATE INDEX`, `ADD COLUMN ... NULL` or `ADD COLUMN ... NOT NULL DEFAULT ...` | Old binary still running | Yes — old writers ignore new columns/tables |
| **Migrate app** | No schema change (or only concurrent indexes) | Deploy new binary that reads/writes the new shape | Yes — schema already present |
| **Contract** | Drop obsolete columns/tables/indexes | New binary already deployed everywhere | Yes — nothing still depends on old shape |

### Forbidden in an expand-phase migration

- `DROP TABLE` / `DROP COLUMN` / `DROP TYPE` (unless annotated `-- contract-phase` and shipped one release after expand)
- `RENAME TABLE` / `RENAME COLUMN`
- `ALTER COLUMN ... TYPE` without a dual-write window
- `ADD COLUMN ... NOT NULL` **without** a `DEFAULT` (old indexer `INSERT`s omit the column and will fail)
- `SET NOT NULL` on an existing column without backfill + default

CI enforces these rules via `scripts/check-indexer-migration-compat.sh`.

---

## How migrations are applied in production

The indexer container runs Prisma migrate on startup before serving traffic:

```dockerfile
# indexer/Dockerfile
CMD ["sh", "-c", "npx prisma migrate deploy && node dist/index.js"]
```

Helm charts set a longer startup probe so `prisma migrate deploy` can finish before readiness:

```yaml
# indexer/helm/values.yaml — startupProbe allows migrate to complete
```

### Local / staging invocation

```bash
cd indexer
cp .env.example .env   # DATABASE_URL required
npm ci
npx prisma migrate deploy    # apply pending migrations (idempotent)
# or
npm run db:migrate
```

### Creating a new migration

```bash
cd indexer
# 1. Edit prisma/schema.prisma (additive change only for expand)
# 2. Generate SQL
npx prisma migrate dev --name add_foo_column --create-only
# 3. Review indexer/prisma/migrations/<timestamp>_add_foo_column/migration.sql
# 4. Apply locally
npx prisma migrate deploy
# 5. Run unit tests
npm test
```

Prefer hand-reviewed SQL. For large tables, create indexes concurrently in a manual step outside the migration transaction when possible:

```sql
-- Example expand: nullable column (old writers fine)
ALTER TABLE "Attestation" ADD COLUMN "sourceRef" TEXT;

-- Example expand: NOT NULL with default (old writers fine)
ALTER TABLE "Attestation" ADD COLUMN "schemaVersion" INT NOT NULL DEFAULT 1;

-- Example expand: new table (old writers ignore it)
CREATE TABLE "WebhookDelivery" ( ... );
```

---

## Production runbook (tested pattern)

Assumes Kubernetes deploy of `trustlink-indexer` with Postgres (see Helm chart under `indexer/helm/`).

### 0. Preflight (5 min)

```bash
# Backup first — see indexer/scripts/backup.sh and docs/disaster-recovery.md
./indexer/scripts/backup.sh --local

# Confirm current migrate status
kubectl exec -it deploy/trustlink-indexer -- npx prisma migrate status

# Note the running image tag / git SHA
kubectl get deploy trustlink-indexer -o jsonpath='{.spec.template.spec.containers[0].image}{"\n"}'
```

### 1. Expand — ship additive migration with the **old** binary still serving (or with a no-op app change)

Two safe options:

**Option A (recommended): migration-only release**  
Ship a build whose only change is the new migration SQL. The startup `prisma migrate deploy` applies the expand SQL; the binary still uses the old queries.

**Option B: apply migrate as a Job before rolling the deploy**

```bash
kubectl create job --from=cronjob/trustlink-db-migrate expand-$(date +%s) 2>/dev/null || \
kubectl run trustlink-migrate-once --rm -it --restart=Never \
  --image=ghcr.io/haroldwonder/trustlink/indexer:<current-tag> \
  --env="DATABASE_URL=$DATABASE_URL" \
  -- npx prisma migrate deploy
```

Verify:

```bash
kubectl exec -it deploy/trustlink-indexer -- npx prisma migrate status
# Expected: Database schema is up to date
```

Old pods keep serving. New columns/tables exist but are unused.

### 2. Deploy the new indexer binary (reads/writes expanded schema)

```bash
helm upgrade --install trustlink-indexer ./indexer/helm \
  --set image.tag=<new-tag>
kubectl rollout status deploy/trustlink-indexer
curl -sf "$INDEXER_URL/health"
```

### 3. Contract (next release only)

After the new binary has been live for at least one full release cycle and no rollback to the old binary is planned:

```sql
-- contract-phase
-- Safe only after every deployed indexer version ignores these objects.
ALTER TABLE "Attestation" DROP COLUMN "legacyField";
```

Annotate contract migrations with `-- contract-phase` on their own line so CI allows the `DROP`.

---

## Rollback

| Failure point | Action |
|---------------|--------|
| Expand migration fails mid-way | Prisma migrations run in a transaction per file for Postgres DDL where supported. Fix forward or restore from the preflight backup (`indexer/scripts/restore.sh`). |
| New binary unhealthy after expand | Roll back the **Deployment image** only. Schema expand is forward-compatible with the old binary — do **not** drop the new columns. |
| Contract migration already applied | Restore from backup, or re-add dropped objects in a hotfix migration. Prefer avoiding contract until rollback window closes. |

```bash
# Image rollback (schema stays expanded — safe)
helm rollback trustlink-indexer 1
# or
kubectl rollout undo deploy/trustlink-indexer
```

---

## CI gate

On every PR that touches `indexer/prisma/migrations/`:

```bash
scripts/check-indexer-migration-compat.sh --base origin/main
```

This fails the build if a newly added `migration.sql` introduces a backward-incompatible change against a still-running indexer (destructive drops without `-- contract-phase`, renames, `NOT NULL` without `DEFAULT`, unannotated type changes).

Full-repo audit (all migrations):

```bash
scripts/check-indexer-migration-compat.sh
```

---

## Checklist for authors

- [ ] Took a DB backup before production migrate
- [ ] Migration is expand-only **or** annotated `-- contract-phase` after a prior expand release
- [ ] `ADD COLUMN NOT NULL` includes `DEFAULT`
- [ ] No renames; dual-write instead
- [ ] `scripts/check-indexer-migration-compat.sh --base origin/main` passes
- [ ] Staged: expand → deploy new binary → (later release) contract
- [ ] Rollback plan documented in the PR (image rollback vs restore)
