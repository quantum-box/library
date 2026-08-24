# RichText migration rollout

Converting a repository's MARKDOWN body property to RICH_TEXT, so that the
stored value is the editor's block document (lossless — blank lines survive)
rather than a Markdown rendering of it.

Nothing in this rollout is a schema change. RICH_TEXT values ride in the
same legacy `value0..value50` LONGTEXT columns as JSON text, and in the same
`property_values` envelope where dual-write is enabled.

## Preconditions

- The API build that knows the `rich_text` type key is deployed everywhere.
  An older API node reading a converted value degrades to an opaque
  read-only value rather than erroring, but writes through an old node are
  rejected — finish the fleet rollout first.
- Reads do not depend on this migration running at all. The legacy text
  boundary (`PropertyDataValue::parse_rich_text`) converts Markdown found
  under a RICH_TEXT property on the fly, and the next write persists the
  document form. The migration converts eagerly instead of lazily; running
  it per-tenant at leisure is fine.
- New repositories already get a RICH_TEXT `content` property
  (`create_repo.rs`), and the GitHub webhook/import paths create RICH_TEXT
  content properties and convert incoming Markdown themselves. This runbook
  is only for repositories that predate the flip.

## Apply and verify

Dry-run first. It prints, per matching property, how many values would be
converted, and writes nothing:

```
DEV_DATABASE_URL=mysql://... \
  cargo run -p database-manager --bin database_manager_rich_text_migrate -- \
  dev <tenant_id> <database_id>
```

The default target is the property named `content` with datatype MARKDOWN;
pass `--property <name>` for a differently named body. Values that already
hold a block document are skipped, so re-running after a partial failure is
safe.

Apply:

```
... database_manager_rich_text_migrate -- dev <tenant_id> <database_id> --apply
```

Each property converts in a single transaction covering its `fields` row,
its legacy value cells, and its canonical `property_values` rows — a failure
leaves that property entirely untouched.

Verify afterwards:

- `SELECT datatype, type_key FROM fields WHERE id = '<property_id>'` shows
  `RICH_TEXT` (and `rich_text` when the envelope columns are populated).
- Open a converted record in the client: the body renders, and a blank line
  typed into it survives a reload.
- `GET /v1beta/repos/{org}/{repo}/data/{id}` still returns Markdown for the
  body (rendered from the document by `compose_markdown`).

## Backfill and parity

The `property_values` canonical rows are updated by the same transaction
when they exist. If value dual-write is enabled *after* this migration ran,
the standard `database_manager_property_value_backfill` operator picks the
converted values up like any others; no special ordering is needed beyond
the usual rule that backfill and parity complete before any switch to
canonical reads.

## Rollback

RICH_TEXT → MARKDOWN is lossy by definition — an empty paragraph has no
Markdown form. Rolling back therefore re-renders rather than restores:

- If values were not yet edited after conversion, the conversion is the
  identity in the other direction (`to_markdown(from_markdown(md))` is
  idempotent) and re-rendering loses nothing.
- If values were edited, blank lines and any editor-only structure in those
  edits are dropped by the re-render.

There is deliberately no rollback binary. If a rollback is genuinely
needed, restore from the pre-migration snapshot taken as part of the normal
change window, or accept the lossy re-render and write the inverse
one-off with `rich_text::to_markdown`.
