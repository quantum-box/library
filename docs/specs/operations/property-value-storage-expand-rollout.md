# PropertyValue storage expand rollout

This runbook covers the additive foundation, staged dual-write rollout, and
resumable backfill/parity scan for normalized `property_values`. It does not
remove `data.value0..value50` / `fields.field_num`, and it does not enable
canonical-first reads by default.

## Preconditions

Run every query against `tachyon_apps_database_manager` before applying the
new migrations. Any returned row is a hard stop because the corresponding
unique constraint cannot decide which existing definition owns the value.

```sql
-- Two Property definitions already share one legacy value column.
SELECT tenant_id, object_id, field_num, COUNT(*) AS duplicate_count
FROM fields
GROUP BY tenant_id, object_id, field_num
HAVING COUNT(*) > 1;

-- More than one Id Property already exists in a Database.
SELECT tenant_id, object_id, COUNT(*) AS id_count
FROM fields
WHERE UPPER(datatype) = 'ID'
GROUP BY tenant_id, object_id
HAVING COUNT(*) > 1;

-- Scoped child rows must already satisfy the #128 composite boundary.
SELECT d.tenant_id, d.object_id, d.id
FROM data AS d
LEFT JOIN objects AS o
  ON o.tenant_id = d.tenant_id AND o.id = d.object_id
WHERE o.id IS NULL;

-- The forward CHECK migration cannot be applied while an existing canonical
-- envelope is malformed. Do not silently delete or rewrite returned rows.
SELECT tenant_id, database_id, data_id, property_id,
       type_key, type_version, value_encoding_version
FROM property_values
WHERE NOT REGEXP_LIKE(
        type_key,
        '^[a-z][a-z0-9]*(_[a-z0-9]+)*$',
        'c'
      )
   OR type_version = 0
   OR value_encoding_version = 0
   OR NOT JSON_VALID(value);
```

Also capture the migration ledger and a schema backup:

```sql
SELECT version, description, success, HEX(checksum) AS checksum
FROM _sqlx_migrations
ORDER BY version;
```

`data` can be large, and widening 25 columns may rebuild or lock the table on
the deployed MySQL version. Rehearse the exact migration against a
production-sized copy, record its duration and free-space use, and schedule a
write-maintenance window when the rehearsal does not remain online.

Do not edit an already-applied migration to repair preflight failures. Repair
the conflicting data explicitly, record the decision, and rerun the same
append-only migration set.

## Apply and verify

Apply with the normal database-manager migration binary, then verify:

```bash
cargo run -p database-manager --bin database_manager_migrate prod
```

```sql
SELECT column_name, data_type
FROM information_schema.columns
WHERE table_schema = DATABASE()
  AND table_name = 'data'
  AND column_name REGEXP '^value(2[6-9]|[3-4][0-9]|50)$'
ORDER BY CAST(SUBSTRING(column_name, 6) AS UNSIGNED);

SHOW CREATE TABLE property_values;
SHOW CREATE TABLE fields;
```

New deployments default to `legacy_only`. Exercise concurrent Property
creation, then opt in with `PROPERTY_VALUE_STORAGE_MODE` in this order:

1. `dual_write_legacy_read`: write both representations in one transaction,
   continue serving legacy values, and inspect parity logs.
2. `dual_write_canonical_read`: keep dual-writing but serve a canonical row
   when present; only a missing row falls back to legacy.

Treat the mode as a fleet-wide writer capability, not as a per-request
experiment. During a rolling transition, a remaining `legacy_only` writer can
update a legacy column while leaving an already-created canonical row stale;
canonical-first will then serve that stale row rather than fall back. Before
advancing, inventory every API, Lambda, importer, and sync writer, drain all
older replicas, confirm every active writer is in
`dual_write_legacy_read`, and reconcile parity again after the drain.

Do not use or introduce a canonical-only writer in this rollout. Before moving
to canonical-first, verify that `missing_canonical`, `missing_legacy`,
`mismatch`, `opaque`, and `decode_failure` parity states are understood and
that no unexpected state remains.

## Backfill and parity

Start only after every active record writer is confirmed in
`dual_write_legacy_read`. A legacy-only writer can update a row behind the
exclusive cursor after it has been scanned, which invalidates the run. Keep
serving legacy reads throughout the backfill and its verification pass.

Block new Property-schema mutations and drain every in-flight schema writer
before the first dry-run. Keep that freeze in place until apply and the final
zero-seed verification have both completed. Every chunk takes `FOR UPDATE`
locks on the scoped Database row and persisted Property definitions, so an
in-flight schema writer cannot change the definition used by that transaction.
The operational freeze is still required across chunk commits: adding or
changing a Property after an earlier cursor range has committed would make the
completed run describe more than one schema generation. Do not rely on
`FOR SHARE` for this invariant; TiDB ignores shared-lock reads by default unless
shared-lock promotion is enabled.

When a canonical PropertyDefinition envelope is present, the operator requires
it to be a known, writable built-in config that exactly matches the legacy
type/config before decoding any records. A partial, malformed, opaque/future,
or legacy-mismatched definition aborts the chunk before inserts. Only a wholly
absent definition envelope may use the legacy definition directly. Resolve
definition parity explicitly; the PropertyValue backfill must never invent a
value envelope from stale legacy metadata.

The operator is Tenant/Database scoped, processes at most 1,000 records per
transaction, and uses the last processed `DataId` as an exclusive stable
cursor. It defaults to a one-chunk dry-run:

```bash
DEV_DATABASE_URL=mysql://root:@127.0.0.1:3306 \
  cargo run -p database-manager \
  --bin database_manager_property_value_backfill -- \
  dev <tenant_id> <database_id> --batch-size 100 --dry-run
```

Review the value-free counts and checksum, then opt in to inserts explicitly:

```bash
DEV_DATABASE_URL=mysql://root:@127.0.0.1:3306 \
  cargo run -p database-manager \
  --bin database_manager_property_value_backfill -- \
  dev <tenant_id> <database_id> --batch-size 100 --max-chunks 10 --apply
```

If `complete=false`, resume from the exact `next_cursor` and carry the emitted
checksum forward. The checksum is an XOR of framed SHA-256 entry digests, so it
is deterministic across batch boundaries when the same cursor range and
checksum seed are used:

```bash
DEV_DATABASE_URL=mysql://root:@127.0.0.1:3306 \
  cargo run -p database-manager \
  --bin database_manager_property_value_backfill -- \
  dev <tenant_id> <database_id> \
  --after <next_cursor> \
  --checksum-seed <parity_checksum> \
  --batch-size 100 --max-chunks 10 --apply
```

The report never emits Property values. Interpret its fields as follows:

- `expected_values`: non-empty legacy values decoded using the persisted
  Property definition (including the current auto-generated Id projection).
- `missing_values`: expected known envelopes that had no canonical row at the
  start of the chunk. `written_values` is the subset inserted by an apply;
  dry-run always reports zero writes.
- `matched_values`: existing known canonical envelopes equal to the decoded
  legacy value.
- `absent_values`: neither representation contains a value.
- `opaque_values`: a future type or encoding already owns the canonical row.
  The operator includes it in checksum evidence but never updates it. Upgrade
  the binary or make an explicit compatibility decision before cutover.

A corrupt legacy value, unsafe PropertyDefinition envelope,
malformed/known canonical value envelope, or known value parity mismatch
aborts and rolls back the entire chunk. Existing canonical rows are never
updated: a concurrent insert wins by causing the chunk to fail and be retried.
This is intentional fail-closed behavior, not an upsert repair tool.

After apply completes, rerun the full scope as dry-run from an empty cursor and
zero checksum seed. Expected known rows must be `matched_values`, not
`missing_values`; every `opaque_values` entry needs an explicit decision. Save
the cursor range, counts, and final checksum with the deployment evidence.
Only then consider the separate canonical-first cutover step.

## Rollback

First set `PROPERTY_VALUE_STORAGE_MODE=dual_write_legacy_read`. This is the
application rollback state: both representations stay current while reads are
served exclusively from legacy columns. A `legacy_only` deployment is safe
only when canonical freshness is no longer required during the rollback
window.

A failed backfill chunk has already been rolled back. Do not delete rows after
a successful chunk in an attempt to undo the backfill: canonical rows created
by the dual writer and by this operator are intentionally indistinguishable,
and legacy-read remains the safe application rollback path.

Block Property-schema writes before rolling back the application. A binary
that predates the schema-mutation unit of work still uses
`INSERT ... ON DUPLICATE KEY UPDATE` for `fields`; with the new slot and Id
singleton unique keys, a concurrent collision can enter that update path and
overwrite an existing Property definition. Resume schema writes only after a
compatible non-upsert writer is deployed. Leaving the additive objects in
place remains the preferred schema rollback.

If DDL removal is required, stop all record and schema mutation and future
normalized writers first. Confirm `property_values` is empty. Drop the envelope
checks before the table (MySQL 8.0 syntax), then remove objects in dependency
order:

```sql
ALTER TABLE property_values
  DROP CHECK chk_property_values_type_key,
  DROP CHECK chk_property_values_type_version,
  DROP CHECK chk_property_values_encoding_version,
  DROP CHECK chk_property_values_value_json;

DROP TABLE property_values;

ALTER TABLE fields
  DROP INDEX uq_fields_tenant_object_id_singleton,
  DROP INDEX uq_fields_tenant_object_field_num,
  DROP COLUMN id_singleton_marker;

ALTER TABLE data
  DROP INDEX uq_data_tenant_object_id_id;
```

Do not narrow `value26..value50` back to `TEXT` unless all values are proven to
fit in 65,535 bytes. Leaving them as `LONGTEXT` is the safe rollback default.

Do not change `_sqlx_migrations` after a manual DDL rollback: keep successful
and failed rows exactly as recorded, and never delete, forge, or overwrite a
checksum. Re-introduction and convergence must start from the actual schema in
a new append-only forward migration; an already-recorded migration must never
be replayed by rewriting the ledger.
