# Dormant RelationEdge patch Unit of Work

## Boundary

This slice adds a normalized RelationEdge writer only inside
`VersionedRecordMutationUnitOfWork::decide_patch_atomically`. It is deliberately
dormant: normal `database-manager` factories construct
`RelationEdgeWriteMode::Disabled`, there is no environment parser, and neither
`apps/api` nor `apps/web` can enable it.

The internal/test-only `DualWriteLegacyRead` mode accepts a source Relation
Property patch and commits all of the following in one MySQL transaction:

1. the operation claim or idempotent replay;
2. legacy `data.valueN` Relation CSV;
3. the canonical versioned `property_values` envelope;
4. the normalized `relation_edges` set diff;
5. the source RecordVersion CAS increment;
6. `database.record.patched.v1` in the transactional outbox;
7. the terminal accepted, conflict, or rejected decision.

The mode refuses to start unless canonical PropertyValue dual-write is active.
`Relation([])` is a present typed empty set and emits a `SET` delta, while
`Clear` removes legacy/canonical value storage and emits a `CLEAR` delta. Both
produce zero edges.

Inverse Property writes remain rejected. An inverse is a backlink view of the
source RelationDefinition and never owns a mirrored edge set.

## Lock order and cardinality

Relation patches use this order:

1. `record_mutation_operations.operation_id`;
2. non-locking RelationDefinition discovery;
3. all source/target `objects` rows in `(tenant_id, id)` order;
4. relevant `relationships` rows in Relation id order;
5. source `fields` in `(field_num, id)` order;
6. `index_definitions` in id order;
7. current forward edges and requested-target backlinks in deterministic order;
8. source and requested target `data` rows in `(database_id, data_id)` order;
9. source `property_values` in Property id order.

After endpoint locks are held, definitions are re-read with `FOR UPDATE`. If a
concurrent schema change introduces an endpoint outside the discovered sorted
set, the transaction rolls back. It never appends a newly discovered lock that
could sort before a lock already held.

The RelationDefinition row is the per-Relation serialization mutex. Therefore
two source writers cannot both observe an unoccupied reverse-`ONE` target, and
a forward- or reverse-cardinality violation is a durable
`RELATION_CARDINALITY_EXCEEDED` decision. A stale expected RecordVersion still
wins over Relation, inverse, future-definition, and IndexDefinition capability
rejections.

Active Property or Relation `IndexDefinition` rows remain fail-closed with
`INDEX_PROJECTION_REQUIRED`; this slice does not maintain a physical index.

## Non-activation gates

Do not add an API/config activation path or insert production edges until all
of these gates are complete:

- a durable per-tenant/Database/RelationDefinition backfill checkpoint that
  proves legacy CSV, canonical PropertyValue, and RelationEdge parity; edge
  absence cannot distinguish empty from not-yet-backfilled;
- transactional serialization between IndexDefinition control-plane changes
  and Record writers, plus any required physical exact/unique projection;
- Relation-aware Record create and every legacy update path, so no accepted
  write can bypass edge maintenance;
- cleanup-aware Record delete with outgoing removal, inbound Restrict/Nullify,
  self-loop handling, affected source RecordVersion increments, and outbox
  events;
- Relation schema delete/cardinality narrowing and Database delete using the
  same definition mutex and complete edge scope;
- a mixed-fleet drain proving no old creator, updater, schema mutator, or
  deleter remains;
- backfill quarantine for malformed CSV, duplicates, missing/wrong-scope
  targets, cardinality violations, opaque definitions, and representation
  disagreement.

Until those gates pass, production verification remains:

```sql
SELECT COUNT(*) AS relation_edge_count FROM relation_edges;
```

The required result is zero. A non-zero result is a rollout incident; do not
disable foreign keys or treat an empty edge query as permission to fall back.

## Verification

All Rust commands run on the host. A disposable MySQL 8.0.35 container may
provide only the database service:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test -p database_domain
cargo test -p database-manager --lib
cargo test -p database-manager --lib \
  dormant_relation_writer_dual_writes_and_serializes_cardinality \
  -- --ignored --test-threads=1
```

The MySQL contract covers set/replace/empty/clear parity, exact event deltas,
idempotent replay, stale conflict precedence, missing and wrong-Database
targets, duplicates, inverse/future-definition guards, active Relation index
guards, forward-`ONE` rejection, reverse-`ONE` concurrency, and full rollback
when outbox insertion fails.

Rollback is application-only: keep the additive schema and deploy code with
the writer disabled. Never delete operation decisions or edge rows to retry an
operation. Because no production activation path exists in this slice, a
normal rollback does not require a data rewrite.
