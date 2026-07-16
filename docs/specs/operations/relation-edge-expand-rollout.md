# RelationEdge expand rollout

## Purpose and boundary

`relation_edges` is the normalized, tenant-scoped storage contract for a
RelationDefinition. This release is expand-only. It adds the table, physical
integrity constraints, domain invariants, and read-only forward/backlink
queries; it does not populate or mutate an edge.

The canonical orientation is always the RelationDefinition's source Property:

```text
(tenant, source Database, source Data, Relation,
 target Database, target Data)
```

That six-column scope is the logical identity and primary key. An inverse
Property reads the same edge from `idx_relation_edges_backlink`; it does not
own a mirrored RelationDefinition or mirrored edge. Self Relations and self
loops are valid.

This release does **not** implement:

- legacy CSV or PropertyValue dual-write/backfill;
- cardinality enforcement against concurrent writers;
- Restrict/Nullify execution or any delete cleanup;
- Relation inverse value projection;
- API, `apps/web`, or query-planner integration;
- IndexDefinition projection-state changes.

## Physical contract

`relation_edges` uses three tenant-leading, scoped foreign keys:

- `(tenant_id, source_database_id, relation_id, target_database_id)` binds the
  row to the complete RelationDefinition candidate key;
- `(tenant_id, source_database_id, source_data_id)` binds the source record;
- `(tenant_id, target_database_id, target_data_id)` binds the target record.

All use `ON DELETE RESTRICT`. An edge therefore cannot become orphaned if an
old binary tries to delete a definition, source record, or target record. The
delete fails closed until the cleanup-aware Unit of Work is deployed.

The named forward index begins with the complete RelationDefinition scope, so
it also supports the definition FK without an implicit MySQL index; equality on
that scope plus `source_data_id` serves forward reads. The backlink index serves
target-record reads. They are integrity/query indexes, not proof that an
IndexDefinition physical projection is `READY`.

The migration first adds
`uq_relationships_edge_scope (tenant_id, object_id, id, target_object_id)` and
then creates the edge table. These are separate MySQL DDL statements: if table
creation fails, the harmless candidate key can remain, but a plain sqlx retry
will fail on the duplicate key. No migration statement inserts or derives edge
data.

### Partial DDL recovery

If migration execution failed, inspect both objects before retrying:

```sql
SELECT COUNT(*) AS candidate_key_count
FROM information_schema.STATISTICS
WHERE TABLE_SCHEMA = DATABASE()
  AND TABLE_NAME = 'relationships'
  AND INDEX_NAME = 'uq_relationships_edge_scope';

SELECT COUNT(*) AS edge_table_count
FROM information_schema.TABLES
WHERE TABLE_SCHEMA = DATABASE()
  AND TABLE_NAME = 'relation_edges';
```

If the candidate key exists and `relation_edges` does not, remove only the
partial candidate key and rerun the migration:

```sql
ALTER TABLE relationships
    DROP INDEX uq_relationships_edge_scope;
```

If both objects exist but the sqlx migration record was not committed, first
prove `SELECT COUNT(*) FROM relation_edges` is zero. Then drop the empty table,
drop the candidate key, and rerun the migration from its start. If any edge row
exists, stop: do not repair automatically or erase the evidence.

```sql
DROP TABLE relation_edges;
ALTER TABLE relationships
    DROP INDEX uq_relationships_edge_scope;
```

## Hard rollout gate

Do not insert even one production edge until all old writers/deleters are
drained or feature-gated and one transaction boundary implements every item
below:

1. Record mutation locks the RelationDefinition and all endpoint records in a
   deterministic order, validates targets/cardinality, and updates legacy CSV,
   canonical PropertyValue, RelationEdge, RecordVersion, revision, and outbox
   atomically.
2. RelationDefinition cardinality reconfiguration uses the same definition-row
   mutex as Record writers, locks and validates the complete affected edge
   scope in the same transaction, and rejects a `MANY -> ONE` narrowing when
   persisted edges violate it. The validation must not race a new edge.
3. Record delete removes outgoing edges and implements incoming Restrict or
   Nullify, including source-record version updates. A self loop must not block
   deletion of its own record.
4. Relation schema delete removes its edges before the source Property,
   generated inverse Property, and RelationDefinition.
5. Database delete handles every internal edge before definitions/records and
   still rejects external definitions/backlinks according to policy.
6. Concurrency tests prove forward/reverse `ONE`, concurrent cardinality
   narrowing, delete races, CAS retry, and rollback behavior.

Until that release is active everywhere, this must remain zero:

```sql
SELECT COUNT(*) AS relation_edge_count FROM relation_edges;
```

Any non-zero result before the gate is satisfied is a rollout incident: stop
the edge writer, identify its release/operation, and remove only rows whose
legacy and canonical values have been reconciled. Do not disable foreign keys.

## Later data rollout

The later writer/read modes are independent of this schema migration:

```text
LegacyOnly -> DualWriteLegacyRead -> DualWriteEdgeRead
```

Edge absence cannot distinguish an empty Relation from a Relation that has not
been backfilled. Therefore an edge read must never fall back to CSV merely
because it found zero rows. Read cutover requires a durable checkpoint scoped
to tenant, source Database, and RelationDefinition, including a schema
fingerprint and a completed quarantine/parity pass.

Malformed CSV, wrong tenant/Database targets, duplicate DataIds, missing
targets, cardinality violations, opaque PropertyDefinitions, and canonical vs
legacy disagreement are quarantine cases. Backfill must not silently dedupe or
reinterpret them.

## Verification and rollback

Before merge or deployment:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test -p database_domain
cargo test -p database-manager --lib
cargo test -p database-manager --test relation_edge_schema -- --ignored --test-threads=1
cargo test -p database-manager --test relation_edge_contract -- --ignored --test-threads=1
```

Cargo commands run on the host. A disposable MySQL 8.0.35 container may supply
only the database service for the two ignored integration tests.

Because this release has no writers, rollback is schema-only while the count is
zero:

```sql
DROP TABLE relation_edges;
ALTER TABLE relationships
    DROP INDEX uq_relationships_edge_scope;
```

If the table is non-empty, do not drop it or the constraints. Stop and perform
the same reconciliation required for a rollout incident.
