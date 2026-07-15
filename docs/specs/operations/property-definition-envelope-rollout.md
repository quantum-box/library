# PropertyDefinition envelope schema rollout

This runbook covers only the additive `fields` schema for the canonical
PropertyDefinition type/config envelope. It does not change runtime reads or
writes, backfill legacy definitions, or enable new Property types.

The migration adds nullable `type_key`, `type_version`, and `type_config`
columns. A legacy row must keep all three columns `NULL`; a canonical row must
set all three. SQL `NULL` means "not backfilled", while the text value `null`
in `type_config` is valid canonical JSON for a Property type with unit config.

## Preflight

Run against `tachyon_apps_database_manager` in a read-only session and retain
the output with the deployment record:

```sql
SELECT VERSION() AS mysql_version,
       DATABASE() AS database_name,
       @@SESSION.sql_mode AS sql_mode;

SELECT column_name
FROM information_schema.columns
WHERE table_schema = DATABASE()
  AND table_name = 'fields'
  AND column_name IN ('type_key', 'type_version', 'type_config');

SELECT constraint_name
FROM information_schema.table_constraints
WHERE constraint_schema = DATABASE()
  AND table_name = 'fields'
  AND constraint_name LIKE 'ck_fields_property_definition_%';

SELECT version, description, success, HEX(checksum) AS checksum,
       installed_on
FROM _sqlx_migrations
ORDER BY version;
```

The two schema queries must return no rows before the first application. Stop
if any object already exists: inspect the actual schema and converge with a new
forward migration rather than editing or replaying this migration.

`CHECK` constraints are enforced from MySQL 8.0.16. Confirm that the deployed
server is a supported MySQL 8 release. Rehearse the `ALTER TABLE fields`
against a production-sized copy because its online-DDL behavior, lock time,
and disk use depend on the deployed MySQL patch and table shape. Drain Property
schema mutations and resolve long-running transactions before applying.

## Apply and verify

Capture a backup point, then apply using the normal migration binary:

```bash
PROD_DATABASE_URL='<admin-dsn>' \
  cargo run -p database-manager --bin database_manager_migrate prod
```

Verify the exact columns and enforced constraints:

```sql
SELECT column_name, column_type, is_nullable,
       character_maximum_length
FROM information_schema.columns
WHERE table_schema = DATABASE()
  AND table_name = 'fields'
  AND column_name IN ('type_key', 'type_version', 'type_config')
ORDER BY ordinal_position;

SELECT tc.constraint_name, tc.enforced, cc.check_clause
FROM information_schema.table_constraints AS tc
JOIN information_schema.check_constraints AS cc
  ON cc.constraint_schema = tc.constraint_schema
 AND cc.constraint_name = tc.constraint_name
WHERE tc.constraint_schema = DATABASE()
  AND tc.table_name = 'fields'
  AND tc.constraint_name LIKE 'ck_fields_property_definition_%'
ORDER BY tc.constraint_name;

SELECT COUNT(*) AS partial_envelope_count
FROM fields
WHERE NOT (
  (type_key IS NULL AND type_version IS NULL AND type_config IS NULL)
  OR
  (type_key IS NOT NULL AND type_version IS NOT NULL
    AND type_config IS NOT NULL)
);

SELECT COUNT(*) AS canonical_row_count
FROM fields
WHERE type_key IS NOT NULL;
```

Expect `varchar(64) NULL`, `smallint unsigned NULL`, and `longtext NULL`; all
four constraints must report `ENFORCED = YES`; both counts must be zero in this
expand-only release. Existing application behavior remains on `datatype`,
`datatype_meta`, and `meta_json`.

## Rollback

The safe application rollback is to deploy the previous runtime and leave this
additive schema in place. No old runtime reads or writes the new columns.

If DDL removal is operationally required, first stop Property schema writes and
prove that no later dual-write or backfill rollout has populated the envelope:

```sql
SELECT COUNT(*) AS populated_envelope_count
FROM fields
WHERE type_key IS NOT NULL
   OR type_version IS NOT NULL
   OR type_config IS NOT NULL;
```

Proceed only when the count is zero, then remove constraints before columns:

```sql
ALTER TABLE fields
  DROP CHECK ck_fields_property_definition_type_config,
  DROP CHECK ck_fields_property_definition_type_key,
  DROP CHECK ck_fields_property_definition_type_version,
  DROP CHECK ck_fields_property_definition_envelope_complete,
  DROP COLUMN type_config,
  DROP COLUMN type_version,
  DROP COLUMN type_key;
```

MySQL DDL commits at statement boundaries. If migration application fails,
inspect `SHOW CREATE TABLE fields` and the migration ledger before taking any
action. Do not delete or modify successful or failed `_sqlx_migrations` rows,
and never overwrite a checksum. Converge from the actual schema with a new
append-only forward migration; do not repair or replay this migration in
place.

Once a later rollout has populated any envelope column, do not use this
rollback. Restore compatibility through a new forward migration instead.
