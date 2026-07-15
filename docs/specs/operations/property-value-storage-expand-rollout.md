# PropertyValue storage expand rollout

This runbook covers only the additive foundation for normalized
`property_values`. It does not enable dual-write, backfill existing values, or
remove `data.value0..value50` / `fields.field_num`.

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

Application behavior should remain on the legacy value columns in this
phase. Exercise concurrent Property creation before enabling any later
dual-write deployment.

## Rollback

The application change remains compatible with the expanded schema, so prefer
rolling back the application binary and leaving additive objects in place.

If DDL removal is required, stop all schema mutation and future normalized
writers first. Confirm `property_values` is empty. Then remove objects in
dependency order:

```sql
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
