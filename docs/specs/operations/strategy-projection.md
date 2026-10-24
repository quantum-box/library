# Library strategy projection operations (PLT-6072–6075)

## Runtime and ownership

`apps/api/src/router.rs` mounts `handler::strategy::router` using the existing
Library MySQL pool. `LIBRARY_STRATEGY_ENABLED=true` enables the routes. The
expand migration is `apps/api/migrations/20261024000000_strategy_projection.up.sql`;
run it through the existing Library migration gate before serving this binary
(the legacy writer fence also uses the new table when the feature is disabled).
Disable the flag to remove these routes. Do not drop the tables on rollback.

Set `TACHYON_API_URL` to the trusted Tachyon API origin. HTTPS is required except
loopback. For an isolated test/internal HTTP service, explicitly set
`LIBRARY_STRATEGY_ALLOW_INSECURE_HTTP=true`; never set that for internet traffic.
No client-supplied destination URL is accepted and redirects are disabled.
Responses are bounded to 4 MiB with a 15-second timeout. Failure is closed.

All calls use the existing `Authorization: Bearer …` and `x-operator-id` request
context. Library forwards the caller token to Tachyon; it does not persist it.
A scheduler must use a tenant-scoped service principal authorized for the
Library repo and Tachyon subscription, not a person's saved token. Reading
requires Library repo access and a currently active Tachyon subscription.
Connecting, pulling, switching and changing local context additionally require
`library:UpdateRepo`; upstream acknowledges require Tachyon's integration write
permission. Nothing here grants access merely because field can read an OKR.

## Bootstrap and automatic read-through synchronization

1. Choose a **private** Library repository for online PM projections. Keep MVV
   documents in their normal document repository. The private projection store
   deliberately does not enter database-manager search, sharing, export, MCP,
   or offline replication. Its own routes send `Cache-Control: private, no-store`.
2. In Tachyon create a subscription with `consumer_id`, `workspace_id` and an
   explicit `scope` (`all`, or selected Initiative/Project IDs). Example route:
   `POST /v1beta/{tenant}/pm-sync/subscriptions`.
3. Register its returned identity through Library:
   `POST /v1beta/repos/{org}/{repo}/pm-projection` with
   `{"subscription_id":"…","workspace_id":"…","generation":1}`.
   This validates upstream authorization and creates a **paused** binding.
4. Inspect `GET …/pm-projection/migration-inventory` before changing writers.
   Every old ID is quarantined until workspace provenance is reviewed. Names
   and URLs are not identity evidence.
5. Deploy this fence-aware version to all workers and drain older revisions
   before activating: old binaries do not know about the fence.
   `POST …/pm-projection/activate` acquires a database fence shared with all
   production legacy Linear data-handler writes, waits for the current writer
   transaction to drain, and switches to `tachyon`. Queued legacy attempts then
   fail closed; they cannot resume writes while this mode is selected.
6. Tachyon's durable reconcile worker maintains its feed. Each authenticated
   Library PM list, single-record and resolve request automatically reads one
   bounded feed page using that request's current Bearer, then commits records,
   relations, deduplication and checkpoint atomically before returning. field
   refreshes therefore advance synchronization without an operator repeating
   commands. No token is retained. `sync_pending=true` on a list response means
   another page remains; clients should show partial status and refresh again.
   Reader-triggered refresh performs no upstream ack, since read permission
   must not imply integration write permission.
7. An optional service-principal scheduler may invoke `POST …/pm-projection/pull`
   to refresh while idle and acknowledge upstream after commit. The principal
   needs both Library write and Tachyon ack permission. There is no hidden
   process-local timer. Failure after commit and before ack is safe to retry.
8. On an expired feed cursor, automatic read-through loads a complete authorized
   snapshot and resumes at its pinned generation/sequence. A changed or revoked
   grant instead closes reads and pauses locally; an operator must reapprove
   the scope, invoke `POST …/pm-projection/resync`, then activate. That explicit
   endpoint also supports recovery. Snapshot replacement preserves stable IDs,
   local notes and document references. Resource and relation pages use bounded
   `offset`/`limit` requests pinned to one generation/sequence; changed snapshots
   fail instead of mixing revisions. Every returned resource and relation end
   also receives live batched `validate-resources` authorization, even when the
   changes feed is empty.

`GET …/pm-projection?after={id}&limit=50` returns one page of records, relations,
pending relation count and `next_after`. Shared Projects are one record with
multiple edges. `POST …/pm-projection/resolve` accepts the complete
`{provider,workspace_id,kind,external_id}` reference and returns
`{record_id,resource_ref}`. Read it through
`GET …/pm-projection/records/{record_id}`. The shorter `…/pm-projection/{id}`
is an alias. `POST …/pm-projection/{id}/context` accepts only `note` and
`document_ids`, never provider fields. Referenced documents must be readable.

## Pause, revoke, roll back

`POST …/pm-projection/pause` drains the legacy fence, stops the new writer,
and removes provider payloads from local projection rows. Identity and local
context remain. A live authorization denial or changed generation also pauses
and purges the projection; transient network failure closes the read without
claiming that the object was deleted. A successful `resync` restores provider
fields after renewed authorization.

`POST …/pm-projection/rollback` drains and changes the fence to `legacy`, hides
new projections, and permits the existing legacy data handler again. Restart
legacy webhook/pull processing using its existing configuration after examining
queued failures. Do not run both writers or delete published history.

## Reviewed legacy adoption

Apply both expand migrations, including
`apps/api/migrations/20261024010000_pm_reviewed_adoption.up.sql`.
No migration runs against a live repository automatically. Inventory all old
Linear records first and review their exact IDs and property ownership.

`POST …/pm-projection/migrations/dry-run` accepts:

```json
{"workspace_id":"workspace-id","review_note":"Reviewed source identity and property ownership","entries":[{"retire_only":false,"resource_ref":{"provider":"linear","workspace_id":"workspace-id","kind":"project","external_id":"source-id"},"endpoint_id":"old-endpoint-id","legacy_data_id":"existing-data-id","expected_record_version":"7","local_name":"Local context","clear_property_ids":["provider-property-id"],"preserve_property_ids":["notes-property-id","relation-property-id"],"workspace_attestation":null}]}
```

The review must cover **every** legacy Linear sync record in this repository.
Include obsolete issue copies with `retire_only:true` and `kind:"issue"`; these
are scrubbed without creating an Initiative/Project projection. The endpoint,
repository, tenant, external kind/ID and old data ID must match persisted sync
provenance. Duplicate or unknown mappings fail closed. If historical metadata
lacks `workspace_id`, supply a reviewed `workspace_attestation` containing
`workspace_id` and a concrete `evidence` description; a known mismatch cannot be
overridden. Names and URLs never establish provenance.

Classify every current property exactly once as clear or preserve. Relation
properties must be preserved. Reviewers must place all provider-owned historical
fields in `clear_property_ids`, including provider descriptions; keep independently
authored fields in preserve. `local_name` replaces the provider-owned record title.
Dry-run validates versions and schema, persists the reviewed plan and returns
`id`, `digest`, and `status:"reviewed"`; it does not change records.

The same reviewer explicitly calls
`POST …/pm-projection/migrations/{id}/apply` with `{"digest":"returned digest"}`.
It drains the writer fence, pauses synchronization, rechecks every record, and
clears approved provider fields through database-manager's versioned patch API.
That updates both canonical and legacy field storage and emits normal mutation
sync events. Old source releases are revoked. Local fields and relations retain
their old record IDs; the online projection adopts those IDs. Independent
projection notes and document references are preserved too.

This is a resumable per-record operation across two databases, not a distributed
transaction. The stable operation ID permits safe replay after an interruption
between the field scrub and projection adoption. Version/schema conflicts leave
the writer paused; investigate and review again rather than forcing an update.
Do not delete a partially applied plan. `GET …/migrations/{id}` shows its state.
After all entries commit, a fresh authorized snapshot is loaded and Tachyon is
activated. If refresh fails, replay the same apply request after fixing access.

`POST …/migrations/{id}/rollback` with the same digest restores the original
projection IDs and leaves the writer paused. It **does not restore provider
payloads or revoked releases**. Local documents and relations stay intact.
The separate legacy-writer rollback above needs current upstream authorization;
using it intentionally resumes the old import behavior, including generic copies.
Previously downloaded offline copies cannot be remotely erased while disconnected;
normal mutation events remove cleared fields when those clients reconnect.
Do not certify a historical repository as migrated before this reviewed plan
has applied successfully. No production cleanup was performed by this change.

## Published document versions for field

`POST /v1beta/repos/{org}/{repo}/data/{data_id}/releases` with
`{"expected_record_version":"…"}` captures Markdown from the current authorized
record. Repeating the same version is idempotent. A changed source version
requires explicit republishing and a new release ID.

`GET /v1beta/repos/{org}/{repo}/releases?after={id}&limit=50` lists
non-revoked release metadata for name-based selection (without body). Each
source-document ACL is checked; hidden items may produce an empty page with a
continuation cursor. `GET …/data/{data_id}/releases` lists at most 100 recent releases;
`GET /v1beta/repos/{org}/{repo}/releases/{id}` returns
`{id,data_id,source_version,title,body,published_by,published_at,revoked}`.
The underlying document is authorized again on every read. A deleted source
returns 404, no longer authorized returns 403, and a revoked release returns 410
without its content. `POST …/releases/{id}/revoke` revokes it permanently.
The old release text is immutable; revoking does not rewrite its history.
field should render Markdown safely and must not persist/cache upstream content
beyond its current authorized view.

## Verification

- `cargo test -p strategy_projection --lib` checks the wire contract and identity.
- The ignored `packages/strategy_projection/tests/mysql.rs` is destructive **only
  inside an explicitly named disposable `strategy_test` database**. Set
  `LIBRARY_STRATEGY_TEST_DATABASE_URL` to that isolated DB and run
  `cargo test -p strategy_projection --test mysql -- --include-ignored`.
- `cargo check -p library-api --lib` verifies the production route wiring.
- `cargo test -p library-api --lib handler::strategy::tests` checks authorization
  failures and response caching. The upstream real-provider grant smoke test is
  a separate environment check, not performed against production here.

The explicit HTTP option is only for the PM consumer; the existing application
configuration may enforce additional production URL restrictions. Legacy writer
fences use a separate two-connection pool so lock waiters cannot starve normal
record reads/writes.

Library native Initiative and Project lists, projection detail, local context
editing and synchronization status are implemented in `apps/client`; see
`apps/client/docs/strategy-projection.md` for UI checks. Provider fields are
read-only and fetched online. field consumes the same dedicated API.

The ignored API test `handler::strategy::migration_api::tests` runs the real
versioned mutation engine against a disposable `strategy_migration_test` database
with `LIBRARY_MIGRATION_TEST_DATABASE_URL`. It verifies field removal in canonical
and legacy stores, preserved notes/relations, idempotent resume, stable IDs and
rollback. Never point either destructive test at a shared database.


## Recorded backend verification — 2026-10-24

Executed in `/private/tmp/strategy-library`. The table records the commands
actually used, including direct execution of Cargo-built test binaries (their
hash suffixes are build-specific). All listed commands exited successfully.

For API builds, the environment was:

```sh
export SWAGGER_UI_DOWNLOAD_URL=file:///private/tmp/strategy-swagger-ui-v5.17.14.zip
```

The ZIP was a local copy of the public Swagger UI build dependency, used because
the build-time download failed in the restricted environment. It is not a
runtime configuration or a committed dependency override.

| Check | Exact command | Result |
| --- | --- | --- |
| API library check | `cargo check -p library-api --lib` | PASS, 20.15 s; this check preceded the last inventory-under-fence hardening. The final API test compilation below includes that hardening. |
| Final API compilation | `cargo test -p library-api --lib handler::strategy --no-run` | PASS; initial compile 28.86 s, final recheck 1.36 s. Produced `target/debug/deps/library_api-12928e864fe919e6`. No tests executed by `--no-run`. |
| Projection test compilation | `cargo test -p strategy_projection --lib --test mysql --no-run` | PASS, 3.19 s. No tests executed by `--no-run`. |
| Projection, migration validation and wire golden tests | `target/debug/deps/strategy_projection-3f4597f49fdc214e --nocapture` | 11 passed, 0 failed, 0 ignored, 0 filtered. |
| Transactional MySQL projection test | `LIBRARY_STRATEGY_TEST_DATABASE_URL=mysql://root@127.0.0.1:13369/strategy_test target/debug/deps/mysql-2f1825b514910561 --include-ignored --nocapture` | 1 passed, 0 failed, 0 ignored, 0 filtered; 0.48 s. |
| Final API authorization/read-through tests | `LIBRARY_STRATEGY_TEST_DATABASE_URL=mysql://root@127.0.0.1:13369/strategy_test target/debug/deps/library_api-12928e864fe919e6 handler::strategy::tests --include-ignored --nocapture` | 3 passed, 0 failed, 0 ignored, 513 filtered; 0.26 s. |
| Final L4 actual MySQL migration test | `LIBRARY_MIGRATION_TEST_DATABASE_URL=mysql://root@127.0.0.1:13369/strategy_migration_test target/debug/deps/library_api-12928e864fe919e6 handler::strategy::migration_api --include-ignored --nocapture` | 1 passed, 0 failed, 0 ignored, 515 filtered; 12.00 s. |
| Final whitespace/conflict-marker diff check | `git diff --check` | PASS, no output. |

The four executed test groups above total **16 passed, 0 failed, 0 ignored**.
Filtered counts are other API tests excluded by the scoped name filter, not
successful runs. An earlier run of `handler::strategy::tests` without
`--include-ignored` reported 2 passed and 1 ignored; the final invocation above
explicitly ran that database/network test, so it is not left skipped.

The database tests used an ephemeral `mysql:8.4` Docker container named
`library-strategy-tests`, exposed only on `127.0.0.1:13369`, with no production
or shared database access. A fresh `strategy_migration_test` schema was used for
each migration test run. The test rewrites one historical migration's fixed
`tachyon_apps_database_manager.` qualifier to `strategy_migration_test.` in
memory, so every schema change stays within the disposable database. No source
migration was changed for that test. The container was removed after testing
with `docker rm -f library-strategy-tests`.

Coverage established by the actual database tests:

- Atomic checkpoint/replay, conflicting replay and sequence-gap rollback,
  pending/shared relations, lifecycle revocation, immutable publication,
  stable IDs through resync and writer-fence draining.
- Automatic authorized read-through, pinned two-page snapshot recovery,
  `no-store` headers, and individual resource revocation even when the change
  feed is empty.
- L4 reviewer checks; detection of legacy records arriving after dry-run;
  activation blocked while applying; real database-manager versioned updates;
  provider field removal from both canonical `property_values` and legacy
  `valueN` storage; independent Markdown and Relation preservation; operation-ID
  replay after partial progress; prior published-release revocation; stable-ID
  adoption and sanitized rollback with the writer left paused.

Scope limits: the full 516-test API suite, whole-workspace tests, backend clippy,
CI, and a live Linear/Tachyon provider smoke test were not run in this backend
verification. The upstream in the API integration test is a local HTTP fixture;
wire golden data comes from the actual Tachyon producer. Native client checks
are recorded separately in `apps/client/docs/strategy-projection.md` by its
owner. No production rollout, production migration, merge or commit was
performed as part of these checks.
