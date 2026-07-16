# Record delete decision Unit of Work

This slice adds an internal, versioned Record deletion boundary to the
Database bounded context. It makes target deletion atomic with Relation
`Restrict`/`Nullify`, Record version checks, legacy and canonical value
updates, normalized edge cleanup, domain events, and the decision journal.

The boundary is dormant. No application factory, REST or GraphQL route, SDK,
environment flag, `apps/web`, `apps/api`, or `apps/client` path can call it.
The legacy `DeleteDataInteractor` is unchanged and continues to fail closed
against normalized edge foreign keys.

## Decision contract

`DecideRecordDeleteCommand` carries the tenant, Database, Record, stable
operation ID, expected Record version, and actor. DELETE uses its own request
fingerprint material while retaining the released PATCH v1 fingerprint bytes.
Reusing one operation ID for a different request or mutation kind is rejected.

The journal is claimed before aggregate locks. A completed operation replays
its stored decision even after the Record row has been removed. A new request
first resolves the following precedence:

1. the current Record snapshot when `expected_version` is stale;
2. a capability rejection for a Relation representation this binary cannot
   update safely;
3. full legacy/canonical/edge parity validation, which rolls back on
   corruption and never becomes a durable decision;
4. `IndexProjectionRequired` for a projection that cannot be updated;
5. `RelationDeleteRestricted` for an actionable inbound Restrict edge;
6. `VersionExhausted` when the target tombstone or a Nullified source cannot
   advance its Record version;
7. an Accepted decision after all mutations and events commit together.

Parity, malformed storage, and impossible locked-row counts are
infrastructure failures. They roll the transaction back instead of recording
a durable domain rejection that could hide repairable corruption.

## Pre-cutover Relation semantics

Legacy Relation CSV remains authoritative until a durable backfill and
cutover checkpoint exists. Deletion therefore scans and locks the relevant
legacy source records; a zero-row or partial `relation_edges` projection
cannot hide an inbound reference.

For each affected Relation Property:

- a canonical PropertyValue, when present, must be a known writable Relation
  envelope and match the legacy target set;
- every normalized edge must be present in the legacy set;
- an inbound edge from another Record applies the RelationDefinition's
  `Restrict` or `Nullify` policy;
- an edge sourced by the deleted Record is unconditional cleanup;
- a self-loop is classified as outgoing first and never restricts or
  Nullifies the Record being deleted.

Nullify removes only the deleted target. When the last target is removed, the
value remains an explicit typed empty Relation: legacy storage contains the
target Database ID and canonical storage contains `data_ids: []`. It is not
converted to `Clear`.

Multiple Nullify actions for one source Record are grouped. Its version is
advanced once and one `database.record.patched.v1` event contains all changed
Relation Properties in Property ID order.

## Transaction and lock order

The dormant adapter requires both canonical PropertyValue writes and
RelationEdge writes. DELETE acquires one pool connection and applies
transaction-scoped `READ COMMITTED` before `BEGIN`. This prevents an initial
non-locking discovery from pinning a stale Repeatable Read snapshot while the
transaction waits for an endpoint writer, without borrowing a second pool
connection. It then claims the operation and uses this deterministic order:

1. discover incident RelationDefinitions without locks;
2. lock all endpoint `objects` in Database ID order;
3. re-read and lock `relationships`, rejecting a newly discovered unlocked
   endpoint with a retryable infrastructure error;
4. lock endpoint `fields`, then `index_definitions`;
5. lock incident and complete forward `relation_edges` in primary-key order;
6. lock the target and affected source `data` rows in Database/Record order;
7. lock matching `property_values` in primary-key order;
8. validate CAS, capabilities, policies, parity, and every next version;
9. persist grouped Nullify values and source versions;
10. delete incident edges, legacy index rows, and the target Record;
11. append outbox events, finalize the decision, and commit.

An active target projection, or a projection on a Nullified Relation
Property/definition, returns `IndexProjectionRequired` because physical Index
maintenance is not yet implemented.

## Event order and rollback

All events share one occurrence timestamp. Nullified source events are emitted
in `(database_id, data_id)` order, followed by the target
`database.record.deleted.v1` event. Event sequence starts at one. The delete
event's aggregate version and the Accepted decision's `record_version` are the
checked successor of the deleted row's last live version.

The outbox intentionally has no foreign key to `data`, so the tombstone event
survives physical deletion. A failure in Nullify persistence, edge or legacy
index cleanup, CAS delete, outbox insertion, or decision finalization restores
all values, versions, edges, indexes, the target row, and the operation claim.

## Remaining activation gates

This slice does not make Relation storage production-ready. Activation still
requires:

- Relation-aware versioned Record create and removal of every legacy mutation
  bypass;
- durable CSV/canonical/edge backfill checkpoints, quarantine, and parity;
- physical Index projections and transactional projection updates;
- Outbox dispatch, retry, dead-letter, and redrive operations;
- mixed-fleet drain, read cutover evidence, and rollback rehearsal;
- a public versioned Record API and `apps/web` optimistic decision handling.
