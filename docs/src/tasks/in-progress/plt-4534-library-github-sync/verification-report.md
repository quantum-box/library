# PLT-4534 Phase 0–4 Verification Report

## Scope

This report records completed verification for the Phase 0 readiness correction,
the Phase 1 provider-neutral model, and the Phase 2–4 durable inbound, reviewed
ChangeSet, outbound delivery, API, and operator UI implementation.

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

## Phase 2–4 passed on 2026-09-11

- `cargo +nightly-2026-06-04 fmt --all -- --check`
- `cargo +nightly-2026-06-04 run --manifest-path apps/api/Cargo.toml --bin library_codegen`:
  generated `apps/api/schema.graphql`; binding, ChangeSet, delivery queries and
  mutations match the primary-client operations.
- `cargo +nightly-2026-06-04 check -p library-api`
- `cargo +nightly-2026-06-04 test -p integration_domain -p inbound_sync -p outbound_sync --lib`:
  15 + 114 + 4 passed.
- `DEV_DATABASE_URL=mysql://root:@127.0.0.1:15000/library cargo +nightly-2026-06-04 test -p inbound_sync --test external_sync_repository -- --ignored --nocapture`:
  1 passed against MySQL 8.0.46 with the CI-default `utf8mb4_0900_ai_ci`
  database collation. The isolated contract applies both up
  migrations, round-trips binding, object link, ChangeSet, delivery, and durable
  dispatch capability state, verifies tenant isolation/idempotency/claim
  behavior, applies both down migrations, and removes its temporary database.
- `cargo +nightly-2026-06-04 clippy -p integration_domain -p inbound_sync -p outbound_sync -p library-api --all-targets -- -D warnings ...`
- Focused primary-client Vitest: 3 files, 68 tests passed (external-sync API,
  repository settings interaction, and all locale catalogs).
- Primary-client TypeScript check, focused ESLint, and production build passed.
- `npm run type-check:worker`: sync and public-docs Workers passed the locked
  `wasm32-unknown-unknown` workspace check.
- `npm run test:worker`: sync Worker 25/25 and public-docs Worker 14/14 passed.
- Visual brief HTML5 parse passed; `git diff --check` passed.

## Not run in Phase 2–4

- A dedicated GitHub account/repository round trip covering OAuth, webhook,
  Library accept/reject, outbound commit, conflict recovery, rename, and delete
- Preview deployment, preview database migration, deployed Durable Object alarm,
  or authenticated browser verification
- Production migration or feature activation. Production keeps the experimental
  integration and external-sync engine flags disabled.

At the Phase 2–4 checkpoint, the implementation captured an outbox-derived
delivery immediately after the Library transaction committed, but did not yet
have an independent scanner for the narrow process-death window between commit
and capture. The release-hardening section below closes that local implementation
gap; deployed at-least-once behavior remains a separate gate.

## Release hardening passed locally on 2026-09-11

- Added an independent `library.external-sync.v1` consumer over
  `domain_outbox_deliveries`, with bounded registration, `SKIP LOCKED` claim,
  expiring leases, retry backoff, and dead-letter state.
- Scanner capture persists deterministic `OutboundDelivery` rows without holding
  the Record-event lease across provider I/O. A separate bounded CAS claim retries
  due provider deliveries, and every in-flight provider attempt advances its next
  due time before the network call.
- `DEV_DATABASE_URL=mysql://root:@127.0.0.1:15000/library cargo +nightly-2026-06-04 test -p library-api scanner_registration_and_lease_recovery_are_durable -- --ignored --nocapture`:
  passed twice (library and binary test targets) against MySQL. It verifies
  idempotent registration, exclusive lease, expired-lease recovery, retry state,
  and terminal completion, then removes the exact test rows.
- The MySQL contract exposed `ascii_bin` columns as `VARBINARY`; both scanner
  claim and the pre-existing immediate capture lookup now cast event identifiers
  to character values before Rust decoding.
- `npm run test:worker`: sync Worker 26/26 and public-docs Worker 14/14 passed.
  The new scheduled test proves the disabled gate and exact dedicated bearer on
  the scanner request.
- `tachyon manifest validate --file tachyon.yaml`: both CloudApps and
  OAuth2Resource documents passed.
- `EXTERNAL_SYNC_SCANNER_TOKEN` was registered as a secret for `library-api` and
  `library-client-sync`; only key/type metadata was read back. The manifest stores
  references, not the value.
- The active tenant GitHub connection remains active and its allowlist was
  atomically extended from `quantum-box/library` to include the dedicated
  `quantum-box/library-sample` E2E repository.

## Remaining release gates

- Ready PR CI and PR-scoped API / Worker deployment
- Preview API self-URL override so webhook callbacks stay on the same isolated
  preview database
- Dedicated GitHub OAuth, webhook, outbound commit, conflict, rename, and delete
  round trip, including authenticated browser state after reload
- Production manifest activation of the API engine and Worker cron, followed by
  live scanner, save, and reload evidence

Production remains inactive until all of these gates pass. The Tachyon CLI's
standalone `compute apps sync-secrets` operation currently rejects Lambda and
Worker apps as Pages-only; the normal Cloud App apply/build path remains the
deployment gate for the registered secret references.
