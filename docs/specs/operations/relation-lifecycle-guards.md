# Relation lifecycle guards

This slice makes Database and RelationDefinition lifecycle operations safe in
the presence of normalized `relation_edges`. It does not enable the dormant
Record edge writer, backfill legacy CSV values, or change any public API.

## Ownership and cleanup

- Deleting a Database removes every edge whose `source_database_id` is that
  Database before deleting legacy indexes, Records, RelationDefinitions,
  Properties, and the Database root.
- An external inbound RelationDefinition still rejects Database deletion
  before any mutation. `RelationOnDelete` is a Record policy and does not
  remove a Database schema dependency.
- Deleting a RelationDefinition removes its complete tenant/source/relation/
  target edge scope before deleting the definition and its source or owned
  inverse Properties.
- Reconfiguring a RelationDefinition locks and restores its complete visible
  edge set against the proposed definition. For forward or reverse
  `MANY -> ONE`, it then locks every source Record and canonical value, treats
  legacy Relation storage as authoritative before cutover, validates both
  legacy and canonical sets, rejects canonical parity mismatches or opaque
  values, and rejects any edge that is absent from legacy storage. A partial
  edge backfill may omit legacy edges, but can never hide a cardinality
  violation because the complete legacy values are also checked.

These cleanup operations are safe before backfill completes: the Database and
RelationDefinition own the entire scoped edge set, so deleting all rows in
that scope cannot confuse an unbackfilled value with an intentionally empty
Relation. Record deletion remains fail-closed until its versioned lifecycle
Unit of Work and durable parity checkpoint are complete.

## Lock order

The lifecycle paths share the order used by Relation schema and dormant Record
writers:

1. endpoint `objects` in primary-key order;
2. `relationships` for the RelationDefinition;
3. source and target `fields`;
4. Relation-targeted `index_definitions` in primary-key order;
5. definition-scoped `relation_edges` in primary-key order;
6. for cardinality narrowing, source `data` rows in primary-key order;
7. matching `property_values` in primary-key order;
8. writes and commit.

Database deletion locks its Database root first. It rejects external inbound
definitions and external owned inverse Properties before removing outgoing or
self edges. A concurrent Record writer must acquire the same endpoint object
lock before its definition and edge locks, so it either commits before the
cleanup or re-reads the deleted schema after the cleanup commits.

## Failure and rollback

All edge cleanup remains inside the existing MySQL transaction. A later
definition, Property, Record, or Database delete failure restores the edge
rows together with every earlier descendant mutation. Cardinality rejection
occurs before inverse Property or generation writes.

The ignored MySQL contracts cover:

- cross-Database outgoing edge cleanup while preserving target Records;
- self-Relation/self-loop cleanup;
- forward and reverse cardinality narrowing rejection without generation
  advancement;
- pre-backfill legacy cardinality validation with zero edge rows, plus
  canonical parity rejection;
- RelationDefinition edge cleanup before its RESTRICT foreign key;
- existing preflight and late-failure atomicity regressions.

## Remaining activation gates

Production Record edge writes and edge-backed reads remain disabled until all
of the following are complete:

- versioned Record delete with Restrict/Nullify semantics;
- Relation-aware Record create and every legacy update path;
- durable CSV/canonical/edge backfill checkpoints, quarantine, and parity;
- IndexDefinition mutation serialization and physical projection updates;
- mixed-fleet drain, cutover evidence, and rollback rehearsal.
