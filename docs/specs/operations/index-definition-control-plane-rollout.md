# IndexDefinition control-plane rollout

This runbook covers the additive Database bounded-context control plane for
declarative Property and Relation indexes. It creates `index_definitions` and
does not create, rebuild, or query any physical value index.

The legacy `fields.is_indexed` flag and `indexes` table remain untouched. The
migration does not infer or backfill an `IndexDefinition` from either legacy
signal. Treat the new table as a separate source of truth from its first
explicit declaration onward.

## Supported declarations

Each Property or Relation target has at most one canonical definition. A
definition contains a policy, optional uniqueness, contract version,
monotonic generation, and projection lifecycle state.

| Target | `NONE` | `EXACT` | `RANGE` | `FULL_TEXT` | `unique` |
| --- | --- | --- | --- | --- | --- |
| String Property | yes | yes | no | yes | `EXACT` only |
| Integer or Date Property | yes | yes | yes | no | `EXACT` or `RANGE` |
| HTML or Markdown Property | yes | no | no | yes | no |
| Select or Id Property | yes | yes | no | no | `EXACT` only |
| Relation or MultiSelect Property | yes | yes | no | no | no |
| Location or Image Property | yes | yes | no | no | no |
| RelationDefinition target | yes | yes | no | no | no |

The Property type handler is authoritative for capabilities. A future
Property type must declare its `IndexCapabilities` before its index policies
can be accepted. Do not add type-specific SQL checks to the adapter; policy
validation belongs to the domain kernel.

Declaration, reconfiguration, and every projection lifecycle transition read
the canonical `type_key`/`type_version`/`type_config` envelope directly through
the `PropertyDefinitionRepository`, independent of the general Property
rollout read mode. When canonical and legacy columns disagree, capability
validation fails closed until parity is restored. An absent envelope hides the
target from mutation; a partial or malformed envelope fails; and an unknown
key/version stays opaque and read-only. None of those states fall back to
legacy capabilities. Read-only IndexDefinition queries remain available for
diagnosis and do not claim that an invalid target is buildable.

`definition_version = 1` is the only writable contract in this rollout.
An unknown positive version is readable only when it still uses the v1
policy, state, and invariant vocabulary; it remains read-only until that
version is implemented. A future contract that changes those stored enums or
invariants needs an opaque envelope or a reader upgrade before rollout.

## Generation and projection lifecycle

An explicit `NONE` policy is stored as `DISABLED`. Every active policy starts
as `PENDING`:

```text
PENDING -> BUILDING -> READY
                    -> FAILED -> BUILDING
```

Reconfiguration increments `generation` exactly once and resets the state to
`PENDING`, or to `DISABLED` for `NONE`. Every transition into `BUILDING` also
increments `generation`, making it the fencing token for that projection
attempt. `READY` and `FAILED` retain the attempt generation. Writers compare
the expected generation, and lifecycle updates also compare the previous
state; a mismatch is a conflict and must be re-read instead of retried with
stale input. A delayed completion from an earlier failed attempt therefore
cannot mark a retry ready.

Every control-plane command and query carries the authenticated executor and
operator scope. The explicit `tenant_id` must match that operator scope, the
executor must belong to it, and the scoped Database must exist before any
IndexDefinition repository call. Scope failures are returned as the same
not-found response so callers cannot probe ownership.

Projection lifecycle transitions are worker-owned commands. Only a system
executor or a tenant-scoped service account may start, complete, or fail a
projection attempt; an interactive tenant user cannot self-report `READY`.
Each transition also revalidates the current canonical PropertyDefinition or
RelationDefinition before changing state, so a stale declaration cannot move
toward `READY` after its target becomes opaque, malformed, mismatched, or
missing.

This release stores lifecycle intent only. No worker consumes `PENDING`, so it
is expected for active definitions to remain pending until the physical
projection rollout.

## Preflight

Run these queries against `tachyon_apps_database_manager` in a read-only
session and retain the output with the deployment record:

```sql
SELECT VERSION() AS mysql_version,
       DATABASE() AS database_name,
       @@SESSION.sql_mode AS sql_mode;

SHOW VARIABLES LIKE 'tidb_enable_check_constraint';

SELECT table_name
FROM information_schema.tables
WHERE table_schema = DATABASE()
  AND table_name = 'index_definitions';

SELECT constraint_name
FROM information_schema.table_constraints
WHERE constraint_schema = DATABASE()
  AND table_name = 'relationships'
  AND constraint_name = 'uq_relationships_tenant_object_id';

SELECT COUNT(*) AS legacy_indexed_field_count
FROM fields
WHERE is_indexed = TRUE;

SELECT COUNT(*) AS legacy_index_row_count
FROM indexes;

SELECT COUNT(*) AS fields_without_complete_canonical_definition
FROM fields
WHERE type_key IS NULL
   OR type_version IS NULL
   OR type_config IS NULL;

SELECT version, description, success, HEX(checksum) AS checksum,
       installed_on
FROM _sqlx_migrations
ORDER BY version;
```

The two schema queries must return no rows before the first application. Stop
if either object already exists and inspect the actual schema. Converge with a
new forward migration; never edit or replay an applied migration.

On MySQL, the TiDB variable query returns no rows. On TiDB it must return
`ON`. The normal migration entrypoint rejects TiDB with disabled `CHECK`
constraints.

Record the two legacy counts before migration. They are evidence that the
expand step did not mutate existing compatibility state.

`fields_without_complete_canonical_definition` must be zero for every scope
where operators will declare Property indexes. Complete the
PropertyDefinition backfill and parity gate first; do not infer capabilities
from legacy `datatype` columns.

## Apply

Capture a backup point, drain concurrent Database schema mutations, then use
the normal migration entrypoint:

```bash
PROD_DATABASE_URL='<admin-dsn>' \
  cargo run -p database-manager --bin database_manager_migrate prod
```

The migration adds a tenant/database-leading candidate key to
`relationships` and creates `index_definitions`. Rehearse both DDL operations
against a production-sized copy because lock time and disk use depend on the
deployed MySQL patch and table shape.

## Verify

Verify the columns, constraints, scoped foreign keys, and tenant-leading
indexes:

```sql
SELECT column_name, column_type, is_nullable, column_default
FROM information_schema.columns
WHERE table_schema = DATABASE()
  AND table_name = 'index_definitions'
ORDER BY ordinal_position;

SELECT tc.constraint_name, tc.constraint_type,
       rc.delete_rule, cc.check_clause
FROM information_schema.table_constraints AS tc
LEFT JOIN information_schema.referential_constraints AS rc
  ON rc.constraint_schema = tc.constraint_schema
 AND rc.table_name = tc.table_name
 AND rc.constraint_name = tc.constraint_name
LEFT JOIN information_schema.check_constraints AS cc
  ON cc.constraint_schema = tc.constraint_schema
 AND cc.constraint_name = tc.constraint_name
WHERE tc.constraint_schema = DATABASE()
  AND tc.table_name = 'index_definitions'
ORDER BY tc.constraint_name;

SELECT index_name, non_unique,
       GROUP_CONCAT(column_name ORDER BY seq_in_index) AS columns_in_order
FROM information_schema.statistics
WHERE table_schema = DATABASE()
  AND table_name IN ('index_definitions', 'relationships')
  AND index_name IN (
      'uq_index_definitions_tenant_database_id',
      'uq_index_definitions_property_target',
      'uq_index_definitions_relation_target',
      'idx_index_definitions_tenant_database_state',
      'idx_index_definitions_tenant_policy',
      'uq_relationships_tenant_object_id'
  )
GROUP BY table_name, index_name, non_unique
ORDER BY table_name, index_name;
```

Before accepting the first explicit declaration, prove that the migration did
not synthesize rows and that the legacy baselines did not change:

```sql
SELECT COUNT(*) AS definition_count FROM index_definitions;
SELECT COUNT(*) AS legacy_indexed_field_count
FROM fields WHERE is_indexed = TRUE;
SELECT COUNT(*) AS legacy_index_row_count FROM indexes;
```

`definition_count` must be zero. The two legacy counts must match preflight.
Do not insert compatibility rows to make the counts agree.

After declarations begin, monitor by tenant and database:

```sql
SELECT tenant_id, database_id, projection_state, COUNT(*) AS definitions
FROM index_definitions
GROUP BY tenant_id, database_id, projection_state
ORDER BY tenant_id, database_id, projection_state;

SELECT id, tenant_id, database_id, policy, is_unique,
       definition_version, generation, projection_state, updated_at
FROM index_definitions
WHERE projection_state IN ('PENDING', 'BUILDING', 'FAILED')
ORDER BY updated_at;
```

In this control-plane-only release, `PENDING` is informational rather than an
incident. `BUILDING` and `FAILED` are meaningful only after a projection
worker is deployed.

Run the MySQL contracts in a disposable environment:

```bash
cargo test -p database-manager --test index_definition_schema \
  -- --ignored --test-threads=1
cargo test -p database-manager --test index_definition_contract \
  -- --ignored --test-threads=1
```

## Rollback

The safe application rollback is to deploy the previous runtime and leave the
additive table and candidate key in place. Previous code does not read or
write `index_definitions`; legacy behavior remains available unchanged.

For an individual declaration, prefer a generation-checked reconfiguration to
`NONE`. It preserves audit identity, increments generation, and moves the
definition to `DISABLED`. Do not directly flip `projection_state` or copy
`fields.is_indexed` into the new table.

If DDL removal is operationally required, stop all definition writers and
prove there are no declarations:

```sql
SELECT COUNT(*) AS definition_count FROM index_definitions;
```

Only when the count is zero may a new forward migration drop
`index_definitions` and then `uq_relationships_tenant_object_id`. Do not use a
manual down migration, delete SQLx ledger rows, or overwrite migration
checksums.

## Deferred work

This rollout deliberately leaves the following work for later slices:

- physical Property value projections and rebuild workers;
- RelationEdge storage and reverse-lookup projection;
- QuerySpec, planner selection, and capability-aware execution;
- Record transaction/unit-of-work integration with index maintenance;
- legacy `is_indexed` or `indexes` assessment and explicit backfill tooling;
- shadow reads, correctness comparison, `EXPLAIN` evidence, and performance
  gates before any query path is enabled.
