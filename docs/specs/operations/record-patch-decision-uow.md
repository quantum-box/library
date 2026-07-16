# Record patch decision Unit of Work

This runbook covers the expand-only Database bounded-context boundary for a
versioned Record patch. It adds an atomic CAS decision, operation idempotency,
and a transactional outbox. The existing REST/GraphQL `updateData` path is not
switched by this slice, and no API or `apps/web` input is added yet.

## Atomic contract

`RecordVersion` remains a 1-origin, monotonically increasing Database BC
revision. A patch command contains a caller-generated opaque `operation_id`,
an `expected_version`, and a delta. The trusted actor is derived from the
authenticated executor; no client-provided actor crosses the application
boundary.

One transaction performs the following work:

1. claim and lock `record_mutation_operations.operation_id`;
2. lock the Database and Property definitions in canonical order;
3. lock the Record and canonical Property values;
4. compare `expected_version` with the current Record version;
5. validate and write Property storage according to the active rollout mode
   (dual-write is required before activation);
6. CAS-increment `data.record_version` exactly once;
7. insert `database.record.patched.v1` into `domain_outbox_events`;
8. persist the accepted, conflict, or rejected operation decision.

The mutation, version increment, outbox event, and final decision commit or
roll back together. A conflict persists the current typed Record snapshot but
does not change the Record or emit an event. Infrastructure failures remain
errors and roll back; they are never converted into a durable rejection.

An identical retry returns the original stored decision, including after
later mutations have advanced the Record. Reusing an operation ID with a
different tenant, Database, Record, actor, expected version, or payload returns
`IDEMPOTENCY_KEY_REUSE` without changing the original journal row.

Versions in decision and event JSON are decimal strings so values through
`BIGINT UNSIGNED` maximum survive JavaScript and JSON round trips. A patch at
`18446744073709551615` is rejected as `VERSION_EXHAUSTED`; it never wraps.

## Expand-only schema

Migration `20260716120000_create_record_mutation_journal.sql` creates:

- `record_mutation_operations`: the globally unique, case-sensitive operation
  claim, request fingerprint, and versioned terminal decision;
- `domain_outbox_events`: immutable aggregate-ordered events, with
  `INT UNSIGNED` one-origin `event_sequence`;
- `domain_outbox_deliveries`: per-consumer retry and lease state, kept separate
  from event lifecycle.

Outbox events intentionally have no foreign key to `objects` or `data` so an
event survives later aggregate deletion. They do reference their operation
with `ON DELETE RESTRICT`. Delivery rows reference events with
`ON DELETE CASCADE`.

The migration creates no delivery rows and starts no resident poller. Consumer
registration, dispatcher/lease processing, retention, and dead-letter policy
are follow-up work.

## Projection gates

This writer fails closed for a Relation Property patch until the RelationEdge
writer and cardinality/delete policy join the same transaction. It also fails
closed when a changed Property has an active `IndexDefinition` projection.
For a stale command, CAS conflict takes precedence; a fresh retry then receives
the projection-specific rejection.

Create/delete Record operations, RelationEdge writes, physical index
projection, REST/GraphQL adoption, Photon transport, and `apps/web` optimistic
UI are outside this slice.

The current API default is `legacy_only`. Do not enable any public or internal
caller of this new mutation port until PropertyValue backfill and parity are
complete and `PROPERTY_VALUE_STORAGE_MODE` is
`dual_write_legacy_read` (or `dual_write_canonical_read`). That activation gate
ensures every accepted patch writes both legacy and canonical storage; the UoW
continues to honor the configured rollout mode while it remains dormant.

## Deploy and verify

Run the migration before deploying code that can invoke the new application
port:

```bash
cargo run -p database-manager --bin database_manager_migrate prod
```

Use the normal migration binary rather than executing the SQL file directly.
MySQL 8.0.16 or later is required because the journal's terminal-decision and
nonzero-version invariants depend on enforced `CHECK` constraints. On TiDB,
the migration preflight additionally requires
`tidb_enable_check_constraint = ON`.

Verify schema shape and that no operation is left committed as `PENDING`:

```sql
SHOW CREATE TABLE record_mutation_operations;
SHOW CREATE TABLE domain_outbox_events;
SHOW CREATE TABLE domain_outbox_deliveries;

SELECT operation_id, tenant_id, database_id, data_id, created_at
FROM record_mutation_operations
WHERE decision_kind = 'PENDING';

SELECT operation_id, event_sequence, COUNT(*)
FROM domain_outbox_events
GROUP BY operation_id, event_sequence
HAVING event_sequence = 0 OR COUNT(*) <> 1;
```

Both queries must return no rows after requests have settled. Before enabling
the public mutation surface, exercise accepted, stale conflict, same-operation
concurrency, different-operation concurrency, lost-ack replay, tenant denial,
forced outbox failure, and Relation/Index fail-closed cases against a fresh
MySQL 8 instance.

All Library Rust checks and tests run on the host. A container may provide only
the disposable MySQL service used by ignored integration tests.

## Rollback and recovery

Application rollback is additive: keep all three tables and the
`data.record_version` column. Older binaries ignore them and continue through
the legacy update path. Do not reverse migrations, forge the SQLx migration
ledger, delete operation rows to retry a request, or edit a final decision.

A committed `PENDING` row is an invariant violation because the claim and
decision share one transaction. Stop the new mutation surface, preserve the
row and database logs, and investigate transaction or schema corruption before
manual repair. A normal database/network/outbox failure leaves no operation,
Record change, version increment, or event and can be retried with the same
operation ID.

MySQL does not wrap all statements in a multi-table migration in one
transaction. If deployment stops partway through this migration, first check
`_sqlx_migrations` for version `20260716120000` and inventory the three tables
in `information_schema.TABLES`. If the ledger has no row for this version and
every table that was created contains zero rows, drop only the created tables
in dependency-reverse order (`domain_outbox_deliveries`,
`domain_outbox_events`, `record_mutation_operations`) and rerun the normal
migration binary. If the ledger records the migration, any table contains a
row, or the state is otherwise ambiguous, do not drop or forge anything: stop
the mutation surface, preserve evidence, and investigate before repair.

Before public cutover, every legacy Record writer must either route through the
new UoW or be feature-gated. Running a non-CAS writer beside the new boundary
would preserve database validity but violate the optimistic concurrency and
event completeness contract.
