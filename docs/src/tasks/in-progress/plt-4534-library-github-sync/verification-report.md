# PLT-4534 Phase 0–1 Verification Report

## Scope

This report records completed verification for the Phase 0 readiness correction
and the Phase 1 provider-neutral binding/object-link model and persistence layer.

## Phase 0 passed on 2026-09-11

- `cargo +nightly-2026-06-04 fmt --all -- --check`
- `cargo +nightly-2026-06-04 clippy -p inbound_sync_domain -p inbound_sync -p library-api --lib -- -D warnings -A clippy::double_must_use -A clippy::redundant_field_names`
- `cargo +nightly-2026-06-04 test -p inbound_sync_domain --lib`: 31 passed
- `cargo +nightly-2026-06-04 test -p inbound_sync --lib`: 109 passed
- `cargo +nightly-2026-06-04 check -p library-api`
- HTML parse, internal link, relative file link, and inline SVG validation
- `git diff --check`

`scripts/desktop-release/release.mjs check-version` compares committed changes to
`origin/main`, so it is rerun after the implementation commit. The intended desktop
version is `0.1.48`, above the `origin/main` version `0.1.47`.

## Wider check observation

`cargo clippy --all-targets` also inspected pre-existing test code outside this
change and reported `bool_assert_comparison` in
`providers/github/data_handler.rs` and `assertions_on_constants` in
`handler/live.rs`. The changed library targets pass clippy; PR CI remains the gate
for the full workspace and reports these separately if the current nightly enables
them there.

## Not run in Phase 0

- Real GitHub OAuth, webhook delivery, or a test repository scenario
- Library API scenario from binding creation through ChangeSet acceptance
- Browser UI for connection, conflict, and retry state
- Durable consumer invocation and backlog alarms in a production-equivalent runtime
- CRM or other provider adapter implementation

The binding/link schema is now covered by Phase 1 below. The other items require
the durable dispatcher, ChangeSet adapter, outbox consumer, and UI planned for
later PLT-4534 phases.

## Phase 1 passed on 2026-09-11

- Owning component version: `library-api` `1.11.5` on `origin/main` → `1.11.6`
  in this PR (patch bump).
- `cargo +nightly-2026-06-04 check -p integration_domain -p inbound_sync`
- `cargo +nightly-2026-06-04 test -p integration_domain --lib`: 12 passed
- `cargo +nightly-2026-06-04 test -p inbound_sync --lib`: 113 passed
- `cargo +nightly-2026-06-04 test -p inbound_sync --lib external_sync_repository`:
  2 passed (111 filtered)
- `DEV_DATABASE_URL=mysql://root:@127.0.0.1:15000/library cargo +nightly-2026-06-04 test -p inbound_sync --test external_sync_repository -- --ignored --nocapture`:
  1 passed against MySQL 8.0.46
- `cargo +nightly-2026-06-04 clippy -p integration_domain -p inbound_sync --all-targets -- -D warnings -A clippy::double_must_use -A clippy::redundant_field_names -A clippy::bool_assert_comparison -A clippy::assertions_on_constants`
- `cargo +nightly-2026-06-04 check -p library-api`
- The MySQL contract creates an isolated temporary database, applies the Phase 1
  up migration, round-trips a binding and object link through the SQLx adapters,
  verifies tenant isolation and duplicate-identity rejection, applies the down
  migration, and removes the temporary database.

## Not run in Phase 1

- Binding REST / GraphQL API and primary-client UI (not implemented in Phase 1)
- Real GitHub OAuth, webhook, repository, or content mutation
- Durable dispatcher, ChangeSet acceptance, and outbound delivery (later phases)
- Preview deployment or production database migration

Phase 1 proves the local domain and persistence boundary. It does not claim that
continuous synchronization works at an external or deployed surface.
