# Database deletion atomicity

Database deletion is a tenant-scoped aggregate operation. The application
calls a `DatabaseDeletionUnitOfWork` output port; only its MySQL adapter owns
the transaction and physical delete order.

## Lock and validation protocol

1. Lock `objects(tenant_id, id)` for the requested Database with
   `FOR UPDATE`. A missing or wrong-tenant id returns the same generic
   `resource not found` result.
2. RelationDefinition writers lock their source and target `objects` rows in
   primary-key order before writing. Holding the Database row therefore closes
   the race between the inbound-reference preflight and deletion without
   requiring RelationEdge storage.
3. Under that lock, read external inbound definitions in
   `(object_id, id)` order with `FOR UPDATE`. Any definition whose source
   Database differs from the target rejects deletion. Self-relations are owned
   by fields inside the aggregate and may cascade with those fields.
4. Reject a source Database that still owns a generated inverse Property in a
   different Database. Delete that Relation schema first so its endpoint-
   ordered UoW removes both Properties. Self Relation inverses stay inside the
   aggregate and may cascade.
5. Delete legacy index projections, records, fields, then the Database row in
   the same transaction. Canonical PropertyValues and source-owned
   RelationDefinitions follow their existing foreign-key cascades.
6. Commit only after the root row was deleted exactly once. Every validation
   or SQL failure explicitly rolls the transaction back before returning.

The operation does not add RelationEdge semantics or reinterpret
`on_target_delete`; Database-level external references remain restrictive for
both `RESTRICT` and `NULLIFY` definitions.

## Regression gate

`database_delete_atomicity` runs against MySQL and verifies:

- external inbound definitions reject before records or fields change;
- external owned inverses reject before the source Database can orphan them;
- self-relations cascade with their owned schema;
- index projections are deleted with their records;
- a forced late root-delete failure restores earlier descendant deletes,
  including index projections;
- deletion waits for the endpoint-ordered RelationDefinition writer and then
  re-runs the inbound preflight without deadlocking;
- a wrong-tenant id stays concealed as not found and leaves the owner tenant's
  aggregate untouched.
