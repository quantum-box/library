# RelationDefinition control-plane rollout

This runbook covers the additive RelationDefinition control plane only. The
physical `relationships` table keeps its historical name, and Relation values
remain in `data.value0..value50` CSV / normalized `property_values` during this
phase. This rollout does not create `relation_edges`, enforce record
cardinality, materialize inverse values, or enable backlink queries.

## Compatibility floor

Deploy this migration only with a Property-schema writer that creates the
`fields` row and Relation definition in one transaction. PR #149 is the
minimum compatible writer. A pre-UoW binary can commit a Property without its
definition and must not receive Property-schema writes after this migration.

Existing readers remain compatible because `relationships`, `relation_id`,
`target_object_id`, `fields.datatype`, and `fields.datatype_meta` are not
renamed or removed. Old insert statements receive the new defaults:

- forward cardinality: `MANY`
- reverse cardinality: `MANY`
- inverse Property: none
- target delete policy: `RESTRICT`

## Preflight

Run these checks against `tachyon_apps_database_manager`. Any returned row is
a hard stop.

```sql
-- One source Property must own at most one definition.
SELECT tenant_id, object_id, field_id, COUNT(*) AS definition_count
FROM relationships
GROUP BY tenant_id, object_id, field_id
HAVING COUNT(*) > 1;

-- Every definition must belong to a Relation Property.
SELECT r.id, r.tenant_id, r.object_id, r.field_id, f.datatype
FROM relationships AS r
JOIN fields AS f
  ON f.tenant_id = r.tenant_id
 AND f.object_id = r.object_id
 AND f.id = r.field_id
WHERE UPPER(f.datatype) <> 'RELATION';

-- The legacy Property config and definition target must agree.
SELECT r.id, r.target_object_id,
       JSON_UNQUOTE(JSON_EXTRACT(f.datatype_meta, '$.database_id'))
         AS configured_target_object_id
FROM relationships AS r
JOIN fields AS f
  ON f.tenant_id = r.tenant_id
 AND f.object_id = r.object_id
 AND f.id = r.field_id
WHERE COALESCE(
        JSON_UNQUOTE(JSON_EXTRACT(f.datatype_meta, '$.database_id')),
        ''
      ) <> r.target_object_id;
```

Capture the migration ledger and a schema backup before applying:

```sql
SELECT version, description, success, HEX(checksum) AS checksum
FROM _sqlx_migrations
ORDER BY version;

SHOW CREATE TABLE relationships;
```

Do not edit an applied migration or rewrite `_sqlx_migrations`. Repair
preflight data explicitly and introduce every schema correction as a new
append-only migration.

## Apply and verify

Apply with the normal database-manager migration binary:

```bash
cargo run -p database-manager --bin database_manager_migrate prod
```

Verify the additive columns, defaults, constraints, and source ownership:

```sql
SHOW CREATE TABLE relationships;

SELECT forward_cardinality, reverse_cardinality, on_target_delete,
       COUNT(*) AS definition_count
FROM relationships
GROUP BY forward_cardinality, reverse_cardinality, on_target_delete;

SELECT COUNT(*) AS duplicate_source_properties
FROM (
  SELECT tenant_id, object_id, field_id
  FROM relationships
  GROUP BY tenant_id, object_id, field_id
  HAVING COUNT(*) > 1
) AS duplicates;
```

Exercise creation of both a cross-Database Relation and a self Relation. Each
must commit exactly one `fields` row and one `relationships` row. Confirm the
new row is `MANY` / `MANY` / `RESTRICT` with `inverse_field_id IS NULL`.

## Rollback

Prefer rolling back only to a compatible post-#149 application and leaving the
additive schema in place. Before any rollback to a pre-UoW writer, block all
Property-schema writes.

If the additive DDL must be removed, first ensure no definition uses nonlegacy
settings:

```sql
SELECT id
FROM relationships
WHERE forward_cardinality <> 'MANY'
   OR reverse_cardinality <> 'MANY'
   OR inverse_field_id IS NOT NULL
   OR on_target_delete <> 'RESTRICT';
```

Only when that query is empty, remove the additive objects in dependency order
with a new forward migration. Recreate
`fk_relationships_tenant_object_field` only if the rollback binary requires
its old behavior. The later Relation schema mutation UoW makes
`fk_relationships_tenant_source_property_restrict` restrictive so a mixed-fleet legacy
deleter cannot cascade away inverse ownership. Never restore cascading source
ownership while generated inverses exist, and never delete or forge a
migration ledger row to replay this migration.

## Follow-up boundary

RelationEdge persistence, record-write Unit of Work, cardinality enforcement,
inverse materialization, Restrict/Nullify execution, backlink indexes, and CSV
backfill/cutover are separate rollout slices. Legacy `_id` relation-query
normalization and orphan quarantine also belong to the RelationEdge rollout.
Until those land, the new definition columns describe policy but do not
execute it.
