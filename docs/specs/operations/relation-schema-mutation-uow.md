# Relation schema mutation Unit of Work

This runbook covers the versioned mutation boundary for an existing
`RelationDefinition`. It does not create `relation_edges`, rewrite record
values, execute `RESTRICT` / `NULLIFY`, expose backlink queries, change the
Relation target Database, or remove legacy columns.

## Aggregate boundary

The source Relation Property owns exactly one `RelationDefinition`. A
generated inverse is a Property in the target Database that points back to the
source Database, but it does not own a mirrored RelationDefinition. The
following writes commit or roll back together:

- forward and reverse cardinality;
- generated inverse creation, rename, detach, or deletion;
- target-delete policy;
- RelationDefinition generation increment;
- source Relation Property, RelationDefinition, and owned inverse deletion.

Every mutation locks the source and target `objects` rows in ascending id
order before locking either Property schema. Self Relations acquire the one
Database lock once. Database deletion and Relation Property creation use the
same endpoint order.

Generated inverses have `relationships.inverse_owned = TRUE`. Direct Property
update or delete rejects those rows; callers must mutate the owning
RelationDefinition. Compatibility rows with a pre-existing `inverse_field_id`
default to `inverse_owned = FALSE`: removing the inverse detaches it but never
deletes an externally owned Property.

## Version and CAS contract

Existing rows migrate to `definition_version = 1` and `generation = 1`.
Readers retain any positive future version, but the V1 writer fails closed
instead of overwriting it. Reconfigure and explicit delete commands require
the current generation. A successful reconfigure increments it exactly once;
a stale generation returns a conflict.

The legacy Property delete surface has no generation input. It still takes the
same endpoint locks, reads the latest generation, and routes Relation Property
deletion through this UoW so an owned inverse cannot be orphaned.
Database deletion rejects a source Database while it still owns a generated
inverse in another Database; delete those Relation schemas first. Self
Relation inverses remain inside the Database aggregate and cascade safely.

## Apply and verify

Apply migrations with the normal database-manager migration binary:

```bash
cargo run -p database-manager --bin database_manager_migrate prod
```

Verify defaults and ownership coherence:

```sql
SHOW CREATE TABLE relationships;

SELECT definition_version, generation, inverse_owned, COUNT(*)
FROM relationships
GROUP BY definition_version, generation, inverse_owned;

SELECT id, tenant_id, object_id, field_id
FROM relationships
WHERE generation = 0
   OR definition_version = 0
   OR (inverse_owned = TRUE AND inverse_field_id IS NULL);
```

The final query must return no rows. Exercise a cross-Database Relation and a
self Relation. Creating an inverse must add one target `fields` row, keep one
`relationships` row, and increment the source definition generation. Direct
update/delete of the inverse must return a conflict. Deleting the source
Relation must remove both owned Properties and the definition in one commit.

## Rollback and compatibility

Keep the additive columns in place when rolling the application back. Older
readers and Relation creators receive safe defaults, but pre-UoW Property
deleters must be drained before inverse creation is enabled. The migration
changes source ownership from cascading to restrictive, so an old deleter
fails safely instead of removing the only ownership row and orphaning a
generated inverse. The current Property and Database deletion UoWs explicitly
delete RelationDefinitions before fields.

Do not drop the columns or restore cascading source ownership while any row
has `generation <> 1` or `inverse_owned = TRUE`; doing so would lose
concurrency and lifecycle ownership information. Never edit an applied
migration or forge the SQLx migration ledger.

Record cardinality enforcement, inverse value materialization, backlink
indexes, RelationEdge backfill, and `RESTRICT` / `NULLIFY` execution remain
follow-up slices.
