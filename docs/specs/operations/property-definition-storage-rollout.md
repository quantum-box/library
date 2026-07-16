# PropertyDefinition dual-write/read rollout

This runbook covers the expand phase for canonical Property definitions in the
Database bounded context. New binaries write the legacy
`fields.datatype`/`datatype_meta` columns and the canonical
`type_key`/`type_version`/`type_config` envelope in the same SQL statement.
Legacy columns, `field_num`, and the public compatibility `Property` model stay
in place during this phase.

## Runtime modes

`PROPERTY_DEFINITION_STORAGE_MODE` accepts two states:

- `dual_write_legacy_read` (default): both representations are written; legacy
  columns remain authoritative and the canonical envelope is shadow-decoded for
  parity logs.
- `dual_write_canonical_read`: both representations are written; a present
  canonical envelope is authoritative. Legacy fallback happens only when all
  three canonical columns are `NULL`.

There is deliberately no new-binary `legacy_only` writer. Rollback changes read
precedence to `dual_write_legacy_read`; it does not stop maintaining the
canonical copy.

## Preconditions

1. Apply all Database BC migrations. TiDB must have
   `tidb_enable_check_constraint=ON`; the migration entry points fail closed
   otherwise.
2. Deploy `dual_write_legacy_read` everywhere.
3. Drain binaries that only update legacy columns. A mixed fleet can make an
   already-created canonical envelope stale.
4. Confirm new Property and Relation Property creation writes complete
   envelopes and still creates exactly one RelationDefinition.

## Parity audit

Count rows that still require a definition backfill:

```sql
SELECT COUNT(*) AS missing_canonical
FROM fields
WHERE type_key IS NULL
  AND type_version IS NULL
  AND type_config IS NULL;
```

Partial envelopes must remain impossible:

```sql
SELECT id, tenant_id, object_id
FROM fields
WHERE (type_key IS NULL) + (type_version IS NULL) + (type_config IS NULL)
      NOT IN (0, 3);
```

Before canonical-read, shadow logs must explain every
`missing_canonical`, `mismatch`, `opaque`, and `decode_failure` state. This slice
does not provide the backfill/cutover automation; do not enable canonical-read
until missing rows are backfilled and parity has been independently checked.

## Canonical-read behavior

- A missing envelope falls back to the legacy definition.
- When canonical definitions are authoritative but a PropertyValue still falls
  back to a legacy column, the canonical and legacy definition type/config must
  match exactly. A mismatch fails closed instead of reinterpreting legacy bytes
  as a different canonical type. A present canonical PropertyValue remains
  authoritative and does not depend on decoding the legacy shadow.
- A present built-in envelope with malformed config fails closed. It never
  falls back to a potentially different legacy type.
- An unknown key or version is returned as an opaque `PropertyDefinition`, with
  its JSON config retained. It is read-only and is never projected as String.
- Property update/delete rejects an unknown canonical definition, including
  while legacy-read is active, so an older binary cannot overwrite or discard
  newer metadata.
- Canonical PropertyValue hydration uses the resolved PropertyDefinition. An
  unknown definition/value pair therefore stays opaque and unrelated record
  patches do not rewrite it.
- Record writes take the same exclusive Database/schema lock order as Property
  mutations. Do not replace these locks with `FOR SHARE`: TiDB shared locks are
  not enforced by default, which would reopen a definition/value race.

## Rollback

Set all new binaries back to:

```text
PROPERTY_DEFINITION_STORAGE_MODE=dual_write_legacy_read
```

Keep the additive canonical columns and constraints. Do not null or drop the
canonical envelopes during an application rollback. Investigate parity before
another canonical-read attempt.

## Residual work

- Backfill pre-envelope rows and produce a durable parity/checksum report.
- Adopt the canonical contract in REST/GraphQL and `apps/web` without touching
  `apps/client`.
- Add Boolean only after those public boundaries use the registry contract.
- Move integration-specific `meta_json` concerns into the Integration bounded
  context.
