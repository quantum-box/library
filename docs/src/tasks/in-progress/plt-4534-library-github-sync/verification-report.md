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
- The MySQL contract exposed `ascii_bin` columns as `VARBINARY`; scanner and
  immediate-capture lookups now decode those identifiers explicitly instead of
  relying on SQL casts.
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

## Preview deployment passed on 2026-09-12

- Ready PR [#357](https://github.com/quantum-box/library/pull/357) deployed the
  current scanner head `4946aec` to the isolated Preview API.
- Tachyon build `bld_01m28gpn642xmfhr2xfjj1wr8r` succeeded for
  `library-api`; the public Preview API and client origins remained reachable.
- Preview-only branch configuration enables the experimental integration and
  API engine while leaving the Worker cron disabled. Production configuration
  was not changed.
- The internal scanner route rejected an unauthenticated request with HTTP 401.
  One generated token was registered only as a Preview branch secret for the
  API and sync Worker; its value was neither persisted in the repository nor
  printed in verification output.
- Two consecutive authenticated calls to
  `POST /internal/external-sync/outbound-scan` returned HTTP 200. Both returned
  `registered=0`, `record_events_processed=0`,
  `record_events_retried=0`, and `outbound_deliveries_retried=0`, proving the
  deployed empty-backlog path and repeated no-op behavior.
- Preview TiDB exposed projection-pruning failures when scanner statements
  returned only an expression or aggregate while filtering on other columns.
  Registration, claim, lookup, and latest-version statements now select the
  columns used by their predicates/order and decode `ascii_bin` values in Rust.
  The focused MySQL durability test, API check, and clippy passed after the
  compatibility changes.

The zero-count scan does not prove event capture or provider delivery. Those
remain part of the dedicated GitHub round trip below.

## Post-merge packaging hardening on 2026-09-12

- PR #357 merged as `5fbf998510c3e8847f70dfb83f09eba08e3253e2` after its
  required checks passed and all review threads were resolved.
- Tachyon production API build `bld_01m28mjhegxf8rqjz1xgybn73g` succeeded;
  deployment `dep_01m28mtr7az6j4vg6v9r3gd2tf` is active and
  `https://library-api.txcloud.app/health` returned HTTP 200.
- The matching client build `bld_01m28mjb4qqfra8b0tj7jdb4e5` succeeded.
- The matching sync Worker build `bld_01m28mj4wpdrn8ftqbvg73cy1d` stopped in
  manifest apply because a Preview-target scanner credential could not be used
  for production. No production scanner activation occurred.
- The follow-up manifest resolves production with no scanner credential and
  Preview with `EXTERNAL_SYNC_SCANNER_TOKEN`; manifest validation plus both
  production and Preview dry-runs pass.
- Desktop Release run `34621815769` created tag `library-v0.1.53` and then failed
  with `ENOBUFS` while listing full release objects and assets through the
  default 1 MiB `execFileSync` buffer. Release lookup now projects only
  `id`, `tag_name`, `draft`, and `prerelease` in `gh` before Node receives the
  paginated output. The owning client version advances from `0.1.53` to
  `0.1.54` for the follow-up Ready PR.

## Regression verification on 2026-09-12

Before the dedicated repository round trip, new use-case and processor tests
reproduced three release-blocking defects:

1. Re-accepting an accepted or rejected ChangeSet reached the apply adapter
   before the terminal-decision check. Validate acceptance first, then apply and
   persist the decision only after successful application.
2. Push and merged-PR file handling swallowed GitHub read / ChangeSet persistence
   failures as skipped files. The review path now propagates them to the durable
   consumer so the event is retried instead of marked complete.
3. Reviewed content was fetched from a moving branch while retaining the webhook
   commit as its revision. Push upserts and merged-PR modifications / renames now
   fetch the immutable event commit. The configured branch remains the binding
   scope and subsequent delivery target.

Verification:

- Before the fix: `inbound_sync` had 116 passing and 6 failing tests. The failures
  exercised terminal acceptance, push/PR failure propagation, and immutable
  revision reads. Two other new tests cover failed acceptance without a decision
  save and rejecting a pending change without provider/data writes.
- After the fix: `cargo test -p inbound_sync -p integration_domain -p outbound_sync
  --lib --locked` passed: 122 + 15 + 4 tests.
- Client `externalSyncApi`, `RepositorySettingsView`, and locale-catalog tests:
  22 + 46 passed. `npm run type-check` passed.
- `cargo fmt --all -- --check` and `cargo clippy -p inbound_sync --all-targets
  --locked -- -D warnings -A clippy::double_must_use
  -A clippy::redundant_field_names` passed. Two existing boolean assertions in
  the GitHub data-handler tests were updated to satisfy Clippy.
- PR #360 Preview API `/health`: HTTP 200. Unauthenticated
  `POST /internal/external-sync/outbound-scan`: HTTP 401.
- Authenticated Preview browser sign-in succeeded. Its observed REST and GraphQL
  requests targeted `https://pr360--library-api.txcloud.app`, confirming API
  isolation for this browser session.

These checks do not establish a GitHub provider round trip or production
activation. No GitHub file has been created during this checkpoint.

### Preview fixture created on 2026-09-12

- Existing `test333` appeared in the platform organization list, but repository
  creation returned `NotFoundError: organization not found in create repo`:
  the organization was not registered in the Preview Library database.
- Created the empty, dedicated `GitHub Sync Test` organization with the user's
  authorization. Its slug is `github-sync-test-20260912`, and its operator ID is
  `tn_01m2a44pskazsq0b0nzns9nqxs`. No existing organization was imported.
- With explicit approval for this new destination, created the private Library
  repository
  [`github-sync-test-20260912/plt-4534-github-sync-e2e`](https://pr360--library-client.txcloud.app/github-sync-test-20260912/plt-4534-github-sync-e2e),
  ID `rp_01m2a4f5szabzzvtjhg999czne`. The overview showed zero records; the
  settings page loaded the saved synthetic-data description and private visibility.
- The external-sync query loaded successfully and showed no configured binding.
  At this checkpoint the primary-client section supported existing-binding operations but
  exposes no action to connect GitHub or create an initial binding. Connection
  setup and provider round-trip verification remain outstanding.

### Primary-client connection setup on 2026-09-12

- Added the repository-settings `Connect GitHub` dialog, with the runtime gate,
  account authorization, repository/ref validation against GitHub, an optional
  file pattern, and creation of the reviewed Markdown binding. Existing scopes
  are reused when retrying after a lost response. Missing organization context
  never falls back to the platform tenant.
- The shared GitHub callback proxy expects base64 JSON with a `returnUrl`.
  The opt-in `proxyCompatible` authorization mode wraps the signed payload in
  that format. Exchange verifies the signature, matching return URL, tenant,
  and expiry; the client also matches the exact saved state, repository, and
  origin and consumes it once. The code is removed from browser history before
  exchange. Legacy signed callbacks remain supported.
- `connectGithubSync` checks the caller's setup policy and the runtime gate,
  requires an unexpired OAuth token for this tenant, and creates or resumes the
  tenant's connection without replacing its ID or metadata. Credentials remain
  on the API side. The desktop flow opens the corresponding web settings page
  for authorization and allows refreshing the connection on return.
- Local browser verification used synthetic responses only. The Japanese
  connection form rendered correctly, rejected an empty repository, saved the
  selected scope, and showed that GitHub webhook setup remains necessary.
- Focused client tests: 117 passed across six files, including 22 setup API /
  OAuth checks, six setup dialog tests, existing sync/settings tests, and i18n.
  TypeScript and the production client build passed. The build reported existing
  dependency warnings for PGlite `eval` and the PDF.js import split.
- CI at `43b96b4` passed Rust check, Clippy, format, and 1,312 workspace tests
  (67 skipped), followed by the GA database regressions. The OAuth tests cover
  the proxy envelope, signature, tenant, expiry, redirect tampering, and legacy
  callbacks. Their module follows the production code so that the existing
  source-based tenant-seeding guards still inspect the full implementation.
  The client job, including client and Photon Live E2E, also passed; every
  applicable CI check succeeded for this implementation commit.
- The deployed Preview client showed the Japanese `Connect GitHub` dialog and
  loaded the dedicated organization's disconnected account state. During API
  rollout the old schema rejected the new authorization argument; the dialog
  showed an error and allowed retry. All three Preview services subsequently
  deployed successfully. The public schema generated by the running API was
  compared with the checked-in SDL; only argument formatting differed and was
  aligned. The local OAuth state tests also passed (3/3).
- After API rollout, authorization still failed. A credential-free probe of
  the authorization-URL endpoint returned `GitHub OAuth not configured. Please
  configure GitHub provider in IAC manifest.` No token was exchanged and no
  live binding was created. This generic error did not establish that credentials
  were missing; the subsequent diagnosis below identifies a tenant mismatch.
  No client secret was requested or copied.
- This setup step does not configure GitHub webhooks, create a Library webhook
  receiver, import documents, or prove provider delivery. Those operations and
  the dedicated provider round trip remain subsequent verification gates.

### Direct Library authorization diagnosis on 2026-09-12

- Authorization URLs now percent-encode the state so `+` in signed/base64 state
  survives query parsing. The provider regression test also verifies that
  reserved characters cannot alter the requested scope or URL fragment.
  `cargo test -p github_provider --lib --locked`: 1 passed; formatting passed.
- The intended user flow is Library settings → GitHub authorization → the same
  Library settings page. A Tachyon administrator session is needed only to repair
  the Preview deployment configuration, not for each Library user's connection.
- The Library platform tenant `tn_01j91h09tpj5ehwbwfwfxpak2b` has a GitHub provider
  with all required fields populated. The configuration probe recorded only
  provider names, field presence/types, and the public callback URL; no secret
  values were printed or stored. The initial missing-credentials diagnosis was
  incorrect.
- Bootstrap now preserves safe error context and retries an empty provider
  response instead of caching it for the Lambda lifetime. Five focused bootstrap
  tests passed. Preview then reported that it was looking up the historical
  fallback tenant `tn_01j702qf86pc2j35s0kv0gv3gy`, which lacks a GitHub provider.
- Explicit Preview `LIBRARY_TENANT_ID` and `GITHUB_REDIRECT_URI` values are in
  `tachyon.yaml`. API build, IaC check, Preview deployment, and Rust CI succeeded
  for `1f139e1` (client CI also passed), but the serving API still reported the old tenant. Tachyon's
  deployment source explains why: Preview builds run IaC **plan**, and Lambda
  deployment reads persisted environment variables rather than the branch's
  manifest overlay. Deployment success therefore does not prove those values
  have been applied. After the user's approval and administrator sign-in, the
  Library API environment settings confirmed `LIBRARY_TENANT_ID` existed only
  for production. Added both declared values with Target `Preview`, and the
  saved rows showed the intended values and target.
- Preview API build `bld_01m2ac30zq03a10ytbxcw5p9tr` deployed `eaef41d`
  successfully. A fresh synthetic authorization probe returned no errors, an
  authorization URL on `github.com`, and the configured shared callback. The
  live Library button also navigated to GitHub, confirming the runtime repair.
  Rust and client CI passed for this head, including Photon Live E2E.
- GitHub returned its 404 page at the authorization endpoint. This is distinct
  from the earlier Library configuration error; App registration and callback
  validity are not yet established. The organization GitHub Apps list is
  accessible. After the user completed GitHub administrator reauthentication,
  the Client IDs of all four Apps owned by `quantum-box` were compared; none
  matched the configured ID. The signed-in personal account has no GitHub Apps.
  This does not prove global deletion or exclude another owner. That inspection
  did not change existing Apps, and no actual OAuth code was exchanged.
- After the user's explicit approval, registered the private
  `Library GitHub Sync Preview` App under `quantum-box` (App ID `4919004`, public
  Client ID `Iv23li7ynxvi6z7hrk8y`). GitHub displayed registration success.
  Permissions are Contents read/write, Pull requests read, and mandatory Metadata
  read; the exact shared callback has wildcard matching off, user tokens expire,
  and App webhooks are off. Only `quantum-box` can install the App.
- Saved the new public Client ID as `LIBRARY_GITHUB_OAUTH_CLIENT_ID` with Target
  `Preview` in the Library API's persisted environment settings. Prepared
  `LIBRARY_GITHUB_OAUTH_CLIENT_SECRET` with Target `Preview` and Secret checked,
  leaving its value and submission for the user. No new secret was generated,
  read, or copied by the agent. GitHub requires a private key before installation;
  key provisioning and installation into the dedicated repository remain pending.
  A Preview redeployment after credential provisioning is still required.
- The user then requested reusing Tachyon's existing GitHub App through the
  integration API instead of provisioning another App. Removed only the public
  Preview Client ID added above and confirmed neither deployment credential
  override is saved. Closed the unsubmitted secret form; the dedicated App
  remains registered but uninstalled. Secret/private-key provisioning is paused.
- Added deployment-specific `LIBRARY_GITHUB_OAUTH_CLIENT_ID` and
  `LIBRARY_GITHUB_OAUTH_CLIENT_SECRET` overrides, paired with `GITHUB_REDIRECT_URI`.
  State signing, code exchange, and refresh share the same resolver. Partial or
  blank credentials fail instead of mixing with another App's IaC credentials;
  a redirect-only override retains existing behavior. Added two regression tests
  for CI. Per the user's instruction, local Rust verification is limited to
  formatting. For `d840d59`, CI formatting, check, and Clippy passed; Rust tests,
  client tests, and the API build were still running at this checkpoint.
- The registered callback `https://library.n1.tachy.one/oauth/github/callback`
  currently serves the v1 SPA without forwarding to Preview. The shared callback
  `https://api.n1.tachy.one/v1/integrations/callback/github` returned HTTP 307 to
  the exact dedicated Preview settings URL with a synthetic code and state.
  This proves forwarding only. GitHub's acceptance of the callback, actual user
  authorization, code exchange, and connection persistence remain unverified.

### Shared Tachyon App investigation on 2026-09-12

- Tachyon's `packages/integration/src/adapter/axum/callback_handler.rs`
  `handle_github_oauth_proxy` already forwards GitHub's code and state to the
  base64 JSON `returnUrl`. It does not exchange or refresh GitHub user tokens.
  Library's `github_exchange_token` currently performs the exchange locally and
  saves the result through Tachyon's auth API.
- Tachyon's `POST /v1/integrations/{id}/connect` takes the GitHub App installation
  path for GitHub. Its internal installation-token endpoint is not a user OAuth
  code-exchange endpoint. Reusing the existing App without giving Library its
  client secret therefore requires a tenant-authorized OAuth broker in Tachyon,
  plus Library call-site and refresh changes.
- Proposed flow: Library initiates a tenant/user-bound session; Tachyon builds
  authorization using its existing App, verifies signed expiring state and the
  registered return destination at callback, exchanges/stores the token, and
  redirects to Library with an opaque completion reference. Library verifies
  completion in its existing authenticated context. No token belongs in the
  return URL, and GitHub refresh remains with the credential owner.
- The previously inspected Tachyon Cloud App has an `app.n1.tachy.one` callback;
  that host did not resolve from this machine during the follow-up probe. The
  working `api.n1.tachy.one` callback and the existing App's registered callback
  still need to be aligned before a live shared-App authorization test. No
  existing GitHub App settings or Tachyon implementation were changed here.

### Dedicated repository verification still required

Use synthetic documents only, in the private Preview Library fixture and the
dedicated `quantum-box/library-sample` GitHub repository. Record the Preview SHA,
binding / ChangeSet / delivery IDs, GitHub commits, statuses, and browser results;
never record bearer tokens, OAuth codes, or webhook secrets.

| Step | Required evidence |
| --- | --- |
| Connect / import | Active GitHub OAuth connection with access to the dedicated repository; one imported document and its binding/link |
| GitHub change | Provider webhook delivery, durable job completion, pending ChangeSet, unchanged Library content before acceptance |
| Accept / reject | Acceptance changes Library content, rejection preserves it, reload preserves the decision, repeated acceptance performs no writes |
| Library edit | Save survives reload, delivery reaches `delivered`, GitHub commit contains the saved content |
| Retry | Failed dispatch remains retryable; redelivery converges without duplicate changes or commits |
| Conflict | Concurrent remote edit produces a visible conflict and preserves both contents until an explicit resolution |
| Rename | Merged-PR rename produces a reviewed rename; acceptance preserves the Library data ID and changes its link path |
| Delete | Remote removal produces a reviewed tombstone; accepting it does not hard-delete Library data |
| Activation | After all prior gates pass, enable production engine/scanner with production credentials and a fixed scan start timestamp, then verify live scan/save/reload |

An endpoint-specific signed simulation may validate the Preview consumer path,
but must be labeled separately from provider-originated webhook delivery.

## Remaining release gates

- Follow-up Ready PR [#359](https://github.com/quantum-box/library/pull/359)
  merged as `f7330125ce169a917241577e8f2e31bbfbe795ce`; all main checks passed.
- Production sync Worker deployment `dep_01m28pfcy57e5mhpg2gq09zjkb` is active,
  and `/api/health` returned HTTP 200. Library `0.1.54` was published, and the
  public updater metadata reports `0.1.54` for every supported platform.
- Dedicated GitHub OAuth, webhook, outbound commit, conflict, rename, and delete
  round trip, including authenticated browser state after reload
- Provider-originated webhook delivery requires a governance-managed GitHub
  repository setting; an endpoint-specific signed Preview simulation must be
  recorded separately and must not be reported as provider delivery.
- Production manifest activation of the API engine and Worker cron, followed by
  live scanner, save, and reload evidence

Production remains inactive until all of these gates pass. The Tachyon CLI's
standalone `compute apps sync-secrets` operation currently rejects Lambda and
Worker apps as Pages-only; the normal Cloud App apply/build path remains the
deployment gate for the registered secret references.

## Shared Tachyon GitHub App implementation — 2026-09-12

The Preview setup now starts a broker session in Tachyon integration-api using
the signed-in caller, operator, Library platform, exact return URL and a browser
SHA-256 proof. Tachyon exchanges the GitHub code, keeps App/refresh secrets, and
returns an opaque session. The original browser confirms it once via the new
`githubSyncAuthUrl` and `githubSyncCompleteOauth` GraphQL mutations.

`LIBRARY_GITHUB_OAUTH_BROKER_ENABLED=true` selects broker token reads/deletes;
it remains off until the Tachyon broker is deployed and the Preview consumer
origin is registered. Legacy token APIs remain available to other consumers.
The abandoned deployment-specific Client ID/Secret override was removed from
code and the Preview environment; the separately registered Preview App has no
credential configured or installation used by this flow.

- Focused client tests: 28 passed (OAuth proof/callback and setup dialog).
- Client TypeScript: passed.
- Rust: local formatting and diff checks only, as requested. Added CI regressions
  for caller/tenant forwarding, refresh-secret exclusion, no retry on uncertain
  rotation, and 404/error distinction.
- Generated GraphQL SDL, Rust CI and Preview build: pending this commit's build.
- Real authorization / repository round trip: pending broker rollout. No live
  OAuth or sync completion is claimed; production sync flags remain off.

### Broker verification update

- Implementation commit `2a722b8`: GitHub CI Rust check, Clippy, formatting and
  Rust tests passed. Tachyon API Preview build `bld_01m2ag3k906z2n1fp7437hhrch`
  succeeded.
- CI run `34687045912` generated the GraphQL SDL. Its only schema change is the
  two broker mutations with the exact names/arguments used by the client. The
  generated artifact (not a manual SDL edit) is committed with this update.
- A permanent CI check now regenerates the API SDL and checks the committed
  schema, with an artifact to make future drift reviewable.
- Shared App inspection: the ROOT_ID public Client ID matches Tachyon Cloud;
  user token expiration is enabled; Contents and Pull requests already have
  read/write access. The broker uses its own server-owned callback setting so
  the shared IaC localhost callback and existing App callback are not replaced.
- Tachyon PR #9795 adds the authenticated public API proxy as well as broker
  storage and configuration. Actual OAuth remains pending its reviewed rollout.

### Token-export review correction

The broker token endpoint is restricted to registered service-account identities.
Library now retains its process credential when creating a caller-scoped SDK:
caller policy evaluation still uses the original user, then an explicitly allowed
result permits the server to retrieve the token with its own credential. Denied
and missing policy results cannot trigger that read. The SDK regression covers
both credentials and verifies that the refresh secret is never returned.
Tachyon separately verifies the process service account against its consumer
registry. Existing `library-api-service` was found through CLI; its runtime token
identity must match before activating Preview.


### Platform credential preflight

The persisted Preview server credential verifies as `library-api-service`
(`sa_01kr0ggcbt50j4bjryhgs9ewqs`) in the Library platform. Authentication in the
child organization failed with 401 because public API keys are tenant scoped.
The corrected SDK checks the original caller in the organization, then reads
through its process credential with the platform as acting tenant and the
organization as the broker's typed `tenant_id` query. Background reads/deletes
use the same delegation. Tachyon verifies the registered reader and actual
platform ancestry. The runtime policy manifest adds Get/Save/DeleteOAuthToken;
its live policy currently denies these actions and must be applied before
Preview activation. Local Rust verification remains formatting only; CI and
real authorization on the corrected path are pending.

## Shared App callback and organization hydration on 2026-09-13

- Saved the API callback on the existing Tachyon Cloud GitHub App after GitHub
  identity confirmation. The App settings reported a successful update; the
  existing callback remained unchanged and wildcard matching is disabled for
  the new `https://api.n1.tachy.one/v1/integrations/callback/github` entry.
- The Preview browser now reaches the Tachyon Cloud authorization screen and
  returns to the original Library repository settings. The previous
  `redirect_uri` mismatch is resolved.
- Live verification then exposed a client initialization race: the repository
  settings can render before workspace organization metadata is available.
  Consuming the callback at that point removes its one-use browser proof before
  the expected organization can be checked. The broker consequently has no
  completed connection.
- Added a regression that first renders the callback without an operator ID,
  then hydrates the expected organization. It failed before the fix. The dialog
  now preserves the callback until the organization is available and completes
  it once under StrictMode.
- Focused setup, scope/proof, and repository-settings tests: 49 passed.
  TypeScript, focused ESLint, and whitespace checks passed. No local Rust
  compilation was run. Desktop version is 0.1.57.
- A fresh Preview OAuth completion and repository synchronization remain to be
  verified after this client fix is deployed. Library main merge and production
  synchronization activation remain outside this rollout.


## Shared App connection verified; webhook setup repair — 2026-09-13

Preview head `1193c89f97976010d8fd1e1cf0daeff5c3c51aff` passed all CI and
deployed. The real browser completed Library → shared Tachyon Cloud GitHub App
→ Tachyon callback → Library, showed the connected GitHub account, and retained
the connection after a full reload. The saved scope is
`quantum-box/library-sample`, `e2e/plt-4534-external-sync`, `external-sync/*.md`.
Saving that scope again retained exactly one binding. Production remains off.

Preparing the remaining provider delivery test exposed a pre-existing signing
key mismatch: registration returned a random seed, while endpoint verification
used its SHA-256 digest from the legacy `secret_hash` column as the HMAC key.
New GitHub registrations now return the same random-derived verification key.
The column is secret key material, not a publicly safe password digest. Existing
keys and other provider registrations are unchanged. A regression test signs
a payload with the registration result, verifies against the stored key, and
rejects a modified body. The Preview has no global GitHub webhook secret override.

The client now exposes notification setup per saved binding. It loads the exact
organization/repository/branch/path scope, reconciles existing endpoints before
creating one, and presents the payload URL plus a masked, one-session signing
key. Instructions keep TLS verification enabled and subscribe only to push and
pull request events. Endpoint creation explicitly does not claim delivery success
or import data. Reopening does not reveal the key or create another endpoint.

Focused client tests: 39 passed across GitHub OAuth, webhook setup, and external
sync API suites. Rust compilation and tests are delegated to CI; locally only
the changed Rust file is formatted, per the user's resource constraint.
Actual GitHub delivery, accept/reject, outbound, conflict, rename, and tombstone
verification remain pending deployment of this repair.


### Copy and signing-key recovery follow-up

The c674047 Preview created endpoint `whe_01m2cpzgf9qcmmacxnz8j4t8m3`
with the intended organization, repo and scope. The live form exposed a missing
copy action: a masked input alone is insufficient to transfer the signing key.
The client now has an explicit clipboard button and reports clipboard failures.
If a key is lost, an explicit two-step rotation invalidates it and returns a new
key for updating GitHub. Rotation is tenant-owned, uses the existing endpoint
update permission, and changes only the key with compare-and-swap; concurrent
settings edits cannot restore the old key. A database regression covers a
foreign tenant, successful rotation, stale rotation and preserved endpoint data.
No real GitHub webhook or file write occurred before this follow-up.


### Mobile navigation regression exposed by CI

CI run `34742373854` passed Rust and deployed all three Previews, but its mobile
workspace test opened an `optimistic-record-*` ID before the created record
projection settled. The captured page showed Data not found; the reported CSS
mismatch was a consequence of that empty state. Table cards/rows now mark
pending creation as busy and disable opening the temporary ID. The shared
selection handler also rejects temporary IDs from other views. A regression
test exercises mouse and keyboard activation before and after the canonical ID
arrives; the record/context suites passed 22 tests.
