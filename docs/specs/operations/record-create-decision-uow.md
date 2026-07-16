# Record create decision Unit of Work

This slice adds an internal, versioned Record creation boundary to the
Database bounded context. It creates a Record together with normalized
Property values, Relation edges, the initial Record version, a domain event,
and an idempotent operation decision in one transaction.

The boundary is dormant. No application factory, REST or GraphQL route, SDK,
environment flag, `apps/web`, `apps/api`, or `apps/client` path can call it.
Existing create paths remain unchanged, and production RelationEdge writes
remain disabled.

## Decision contract

`DecideRecordCreateCommand` carries caller-selected Record identity, tenant and
Database scope, a stable operation ID, trusted actor, name, and typed Property
commands. CREATE uses mutation-kind-specific request fingerprint material, so
an operation ID cannot be reused by PATCH or DELETE or by a different CREATE
payload.

The operation claim and terminal decision share the Record transaction:

- an identical retry returns the original Accepted or Rejected decision;
- a different request reusing the operation ID returns
  `IDEMPOTENCY_KEY_REUSE` without changing the first journal row;
- an existing Record ID returns `RECORD_ALREADY_EXISTS` and does not rewrite
  that Record;
- malformed, opaque, or representation-mismatched storage remains an
  infrastructure error and rolls the operation claim back.

An accepted CREATE starts at `RecordVersion::INITIAL`. The decision and event
encode versions as decimal strings so JavaScript clients do not lose precision.

## Relation semantics

The dormant adapter requires canonical PropertyValue and RelationEdge writes
to be enabled together. A completely missing canonical PropertyDefinition
envelope is allowed for a pre-backfill row and falls back to its legacy
definition. Once any canonical envelope is present, it must be complete,
known, writable, and exactly match the legacy type and config. Partial,
malformed, opaque, or parity-mismatched envelopes are infrastructure errors;
they never become durable business-policy decisions. A Relation command also
requires its RelationDefinition to be known and writable.

For every Relation value, the transaction:

1. obtains the RelationDefinition serialization mutex;
2. rejects writes through a generated inverse Property;
3. locks and validates every target Record in the configured target Database;
4. rejects missing or wrong-Database targets;
5. validates forward and reverse `ONE` cardinality against the locked edge
   scope;
6. writes the same target set to legacy CSV, canonical PropertyValue, and
   `relation_edges`.

A typed empty Relation stores the target Database ID in legacy storage, an
empty `data_ids` array in canonical storage, and no edge rows. A self Relation,
including a self-loop to the newly created Record, is valid; the pending source
identity is treated as part of the same locked creation transaction rather than
as a missing target.

Active Property or Relation `IndexDefinition` declarations return
`INDEX_PROJECTION_REQUIRED`. This slice does not update a physical index and
must not accept a write that would make an active projection stale.

## Transaction and lock order

CREATE uses one database connection and one transaction. It claims the
operation before aggregate locks, then uses the shared Database mutation lock
order:

1. `record_mutation_operations.operation_id`;
2. non-locking discovery of the source Database, Property definitions,
   RelationDefinitions, and target Databases;
3. endpoint `objects` rows in Database ID order;
4. `relationships`, source `fields`, and `index_definitions` in stable key
   order;
5. relevant forward edges and target backlinks in primary-key order;
6. the requested source identity and target `data` rows in
   `(database_id, data_id)` order;
7. prior immutable aggregate history in `domain_outbox_events` for the
   requested Record identity;
8. validation, Record insert, value/edge writes, event insert, decision
   finalization, and commit.

After endpoint locks are held, schema is re-read. A concurrent schema change
that introduces an endpoint outside the discovered sorted set causes a
retryable infrastructure error; the transaction does not append an out-of-order
lock.

The RelationDefinition row serializes reverse-`ONE` checks. Concurrent creates
that target the same reverse-`ONE` Record therefore cannot both commit.

## Event and rollback

An accepted creation inserts one `database.record.created.v1` event with event
sequence one and aggregate version one. `RecordCreatedEventV1` contains the
canonical Record identity, name, actor, and Property deltas in Property ID
order. The event and Accepted decision describe the exact committed values,
including typed-empty Relations.

A failure while inserting the Record, legacy or canonical values, Relation
edges, outbox event, or terminal decision rolls back all of them, including the
operation claim. The same operation ID can then be retried. A durable Rejected
decision changes no Record, PropertyValue, edge, or outbox state.

## Remaining activation gates

This slice is not a public create API and does not make Relation storage
production-ready. Activation still requires:

- routing every REST, GraphQL, importer, sync, and internal Record creator
  through the versioned boundary;
- routing every update and delete path through their versioned boundaries;
- durable CSV/canonical/edge backfill checkpoints, quarantine, and parity;
- physical Index projections and transactional projection maintenance;
- Outbox consumer registration, dispatch, retry, dead-letter, and redrive;
- an Outbox retention invariant that preserves at least one immutable Record
  identity marker, or a dedicated identity ledger, so a deleted identity can
  never be recreated after event pruning;
- mixed-fleet drain, read cutover evidence, and rollback rehearsal;
- a public versioned Record API and `apps/web` optimistic decision handling.

Until all gates pass, production construction keeps RelationEdge writes
disabled and the normalized edge table must not be populated by this boundary.

## Verification

All Rust commands run on the host. A disposable MySQL 8 service may provide
only the database used by the ignored contract tests:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test -p database_domain
cargo test -p database-manager --lib
cargo test -p database-manager --lib dormant_record_create_uow \
  -- --ignored --test-threads=1
```

The MySQL contracts cover Relation dual-write, typed-empty storage, and the
exact created-event envelopes; replay and cross-kind operation reuse; live and
historical Record ID collision; target and inverse guards; forward/reverse
cardinality serialization; fail-closed PropertyDefinition drift before active
IndexDefinition policy; self-loops; disabled construction; and complete
rollback on outbox failure.
