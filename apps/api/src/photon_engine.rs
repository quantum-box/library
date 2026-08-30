//! Photon Engine sync, mounted at `/api/engine/*`.
//!
//! `apps/client` has always posted document metadata to
//! `${VITE_LIBRARY_API_BASE_URL}/api/engine/push`, and library-api has never
//! served that route. Every push 404'd, and `docsApi`'s
//! `syncDocumentsBestEffort()` swallowed the failure as a `console.warn` -- so
//! documents and attachments have only ever existed in the browser's PGlite.
//! The client was never misconfigured; this module is the missing half.
//!
//! # Why `engine_routes()` rather than `engine_router()`
//!
//! `photon_axum::engine_router()` is the batteries-included form: it brings its
//! own `CorsLayer` and a Swagger UI. library-api already applies a `CorsLayer`
//! to the whole app, and two of them emit two `access-control-allow-origin`
//! headers on one response, which browsers reject outright. `engine_routes()`
//! is upstream's documented seam for a host that supplies its own layers.
//!
//! Both forms register the same handlers, so pushes still land through
//! `StorageAdapter::append_authoritative_operation`: remote-sequence assignment
//! stays in the database, where photon #69 moved it. library-api runs as Lambda
//! and several instances share one TiDB, so a process-local counter would hand
//! out duplicate sequences across replicas, or commit out of order and let a
//! concurrent pull skip a sequence permanently.
//!
//! # Authorization
//!
//! Photon expects to sit behind an edge that has already authenticated the end
//! user, and enforces only a *service* boundary of its own: `PHOTON_AUTH_TOKENS`,
//! a list of static bearer tokens for the edge-to-Photon hop. Here library-api
//! *is* that edge and Photon runs in-process, so that hop does not exist -- and
//! the credential the client actually sends is a Cognito JWT, which no static
//! token list can match. [`AuthConfig::disabled`] therefore stands the static
//! gate down and [`require_engine_caller`] takes its place, because:
//!
//! * nothing else on this router rejects an anonymous caller.
//!   `sdk_auth::caller_token_middleware` only *propagates* the bearer token; it
//!   does not authenticate. Without this middleware the routes would be open to
//!   the internet.
//! * the scope a request names has to be this caller's. The scope travels in
//!   the request -- `kitConfig` builds it from a `tenant_id` URL parameter,
//!   `localStorage` and the stored identity -- so it is a claim, not a fact,
//!   and an authenticated user could otherwise name any scope they liked.
//!
//! It has to be middleware and not an [`EnginePolicy`]: the policy hook is
//! consulted per pushed operation, and `/api/engine/pull` never consults it, so
//! a policy would leave every read unguarded.
//!
//! # One workspace per caller
//!
//! `.env.production` sets neither `VITE_LIBRARY_TENANT_ID` nor
//! `VITE_LIBRARY_WORKSPACE_ID`, so every production client resolves the same
//! `tenant:library:workspace:library-default` for its *Live* room. Syncing
//! durable records under that would give every signed-in user one shared
//! document set, which is why [`ENGINE_ENABLED_ENV`] used to be documented as
//! unshippable rather than merely off.
//!
//! So the Engine does not use it. [`caller_workspace_id`] derives a workspace
//! per caller and [`require_engine_caller`] recomputes it from the
//! *authenticated* identity, rejecting any request that names another. The
//! client's `buildUserWorkspaceId` builds the same string, and the two are
//! pinned to each other by tests on both sides.
//!
//! This separates users; it does not implement sharing. A workspace two people
//! are both meant to reach cannot be expressed by this rule -- it names exactly
//! one caller -- and would need an actual grant check here rather than a naming
//! convention. Live rooms are deliberately left shared, so realtime `records`
//! collaboration is unchanged.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Router,
};
use photon_axum::{
    AllowAllPolicy, AppState, AuthConfig, ServerEngineAdapter,
};
use photon_engine::{MySqlAdapter, PhotonEngine};
use sqlx::sqlite::SqlitePoolOptions;
use tachyon_sdk::auth::ExecutorAction;

use crate::handler::library_executor_extractor::LibraryExecutor;

/// Opt-in switch for the Engine routes, following the precedent set by
/// `LIBRARY_COLLAB_WS_ENABLED` and `LIBRARY_MCP_SSE_ENABLED`.
///
/// Off by default so that enabling it stays a deliberate, per-environment
/// decision: `tachyon.yaml` currently sets it for `preview` only. Turning it on
/// is not undone by turning it back off -- by then clients have pulled whatever
/// the server held.
const ENGINE_ENABLED_ENV: &str = "LIBRARY_PHOTON_ENGINE_ENABLED";

/// The single Photon tenant this deployment serves.
const ENGINE_TENANT_ENV: &str = "LIBRARY_PHOTON_ENGINE_TENANT";

/// The workspace id every caller's own workspace is derived from.
const ENGINE_WORKSPACE_ENV: &str = "LIBRARY_PHOTON_ENGINE_WORKSPACE";

/// Matches `DEFAULT_TENANT_ID` in `apps/client/src/app/kitConfig.ts`. Photon's
/// own default is `photon`, which is not what this deployment's clients send.
const DEFAULT_ENGINE_TENANT: &str = "library";

/// Matches `DEFAULT_WORKSPACE_ID` in `apps/client/src/app/kitConfig.ts`. Note
/// that no caller is ever *served* this workspace: it is the base that
/// [`caller_workspace_id`] suffixes, so the bare value names nobody.
const DEFAULT_ENGINE_WORKSPACE: &str = "library-default";

/// Separator between the base workspace and the caller id.
///
/// The pair to `buildUserWorkspaceId` in `apps/client/src/app/kitConfig.ts`.
/// The two derivations have to produce byte-identical strings -- a client that
/// pushed under a workspace this function does not reproduce would be 403'd on
/// every request -- so both sides pin the literal in a test rather than only
/// describing the rule.
const ENGINE_USER_INFIX: &str = "-user-";

/// Liveness only, and it discloses nothing, so it answers without a credential.
/// `apps/client/scripts/smoke-engine-api.mjs` probes it before pushing.
const ENGINE_HEALTH_PATH: &str = "/api/health";

/// Ceiling on a buffered push body. The middleware has to read the body to see
/// the scope it claims, so the limit belongs here rather than to an extractor.
const MAX_ENGINE_BODY_BYTES: usize = 4 * 1024 * 1024;

fn engine_enabled() -> bool {
    std::env::var(ENGINE_ENABLED_ENV)
        .map(|value| value.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn engine_tenant() -> String {
    std::env::var(ENGINE_TENANT_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_ENGINE_TENANT.to_owned())
}

fn engine_base_workspace() -> String {
    std::env::var(ENGINE_WORKSPACE_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_ENGINE_WORKSPACE.to_owned())
}

/// The one workspace this caller may read and write.
///
/// Photon's scope grammar forbids `:` inside the workspace segment, so the
/// caller cannot be a segment of its own and is folded into the workspace id.
/// `buildUserWorkspaceId` in `kitConfig.ts` builds the same string from the
/// same id: `sign_in` returns the `tachyon_sdk::auth::User` whose `id()` is
/// what [`ExecutorAction::get_id`] yields here, and the client stores exactly
/// that as `userId`.
///
/// Non-alphanumerics are folded to `-` on both sides. That is lossy, so two
/// distinct caller ids could in principle normalize to one workspace -- but
/// Tachyon ids are `[A-Za-z0-9_]` already, so nothing is folded in practice,
/// and the alternative (rejecting ids that need normalizing) would fail closed
/// on an id shape we do not actually issue.
fn caller_workspace_id(caller_id: &str) -> String {
    let normalized: String = caller_id
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || character == '_'
                || character == '-'
            {
                character
            } else {
                '-'
            }
        })
        .collect();
    format!("{}{ENGINE_USER_INFIX}{normalized}", engine_base_workspace())
}

/// Engine sync routes, or an empty router when the feature is off.
pub async fn engine_router(
    library_db: &persistence::Db,
) -> errors::Result<Router> {
    if !engine_enabled() {
        tracing::info!(
            env = ENGINE_ENABLED_ENV,
            "Photon Engine routes disabled"
        );
        return Ok(Router::new());
    }

    let tenant = engine_tenant();
    tracing::warn!(
        env = ENGINE_ENABLED_ENV,
        tenant = %tenant,
        "mounting Photon Engine sync routes"
    );

    // Share library-api's pool instead of dialling TiDB again.
    // `MySqlAdapter::connect` would open a second one, and a Lambda that has to
    // establish two pools before it can answer anything pays for both on every
    // cold start -- the same reason `LibraryApp::new` takes a pool rather than a
    // DSN.
    let state = build_engine_state((*library_db.pool()).clone()).await?;
    Ok(photon_axum::engine_routes()
        .layer(axum::middleware::from_fn(require_engine_caller))
        .with_state(state))
}

/// Takes the pool rather than a `persistence::Db` so a test can hand it one
/// pointed at a throwaway MySQL.
async fn build_engine_state(
    pool: sqlx::MySqlPool,
) -> errors::Result<Arc<AppState>> {
    let adapter = MySqlAdapter::from_pool(pool);

    // `from_pool` does not migrate, so run Photon's own schema here.
    //
    // It is versioned by Photon, in `photon_engine_schema_migrations`, and
    // deliberately not added to library-api's sqlx history: the two advance on
    // different schedules, and a `photon_engine_*` table that library's history
    // also claimed to own would have to be kept in step by hand on every rev
    // bump. Keeping them apart also means this adds nothing for the migration
    // gate to fail on -- the DDL is additive and touches no existing library
    // table, so `preview_migrate` is unaffected either way.
    migrate_engine_schema(&adapter).await?;

    // Photon Live's Yjs rooms live in `AppState::db`. We mount the Engine routes
    // only -- a WebSocket cannot live in the Lambda that serves library-api, so
    // Live stays on the Cloudflare Durable Object -- and no Engine handler reads
    // this pool. It is here because the field is not optional.
    let rooms_db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .map_err(|error| {
            errors::Error::internal_server_error(format!(
                "Photon Live room store could not be opened: {error}"
            ))
        })?;

    Ok(Arc::new(AppState {
        db: rooms_db,
        engine: PhotonEngine::new(ServerEngineAdapter::MySql(adapter)),
        rooms: Default::default(),
        // Stood down on purpose: see the module's authorization note.
        auth: AuthConfig::disabled(),
        // The boundary is `require_engine_caller`, which covers pull as well.
        policy: Arc::new(AllowAllPolicy),
    }))
}

/// MySQL/TiDB error numbers that all mean "someone else already created this".
///
/// 1062 is in the list because the loser of the race fails on the
/// `photon_engine_schema_migrations` primary key rather than on the DDL, if it
/// gets that far.
const ALREADY_EXISTS_ERROR_NUMBERS: [u16; 3] = [
    1050, // ER_TABLE_EXISTS_ERROR
    1061, // ER_DUP_KEYNAME
    1062, // ER_DUP_ENTRY
];

/// How many times a losing migrator re-reads the schema before giving up.
const MIGRATE_ATTEMPTS: u32 = 5;

/// Apply Photon's schema, tolerating a concurrent migrator.
///
/// `MySqlAdapter::migrate` is check-then-act: it reads the highest applied
/// version and applies what is missing. Two Lambda instances cold-starting at
/// once therefore both find the schema absent and both apply it, and the loser
/// fails on `CREATE INDEX` (`CREATE INDEX IF NOT EXISTS` is not MySQL syntax) or
/// on the migrations table's primary key. That is not a rare case for us:
/// library-api runs as Lambda, and a deployment scales out from cold.
///
/// Photon's own note is that a half-applied migration is repaired by running
/// `migrate()` again, so that is what the loser does -- by its next attempt the
/// winner has committed and the call is a no-op. Once the schema exists this
/// costs one `SELECT MAX(version)` and no DDL, so warm deployments pay almost
/// nothing.
///
/// Worth sending upstream: photon could tolerate 1061 itself, or take a
/// `GET_LOCK` around `migrate()`, and then no host would need this.
async fn migrate_engine_schema(
    adapter: &MySqlAdapter,
) -> errors::Result<()> {
    let mut attempt = 1;
    loop {
        let error = match adapter.migrate().await {
            Ok(()) => return Ok(()),
            Err(error) => error,
        };

        if !is_already_exists(&error) || attempt >= MIGRATE_ATTEMPTS {
            return Err(errors::Error::internal_server_error(format!(
                "Photon Engine schema migration failed: {error}"
            )));
        }

        tracing::info!(
            attempt,
            %error,
            "another instance is applying the Photon Engine schema; retrying"
        );
        tokio::time::sleep(std::time::Duration::from_millis(
            100 * u64::from(attempt),
        ))
        .await;
        attempt += 1;
    }
}

fn is_already_exists(error: &photon_engine::EngineError) -> bool {
    let photon_engine::EngineError::Sql(sqlx::Error::Database(database)) =
        error
    else {
        return false;
    };
    database
        .try_downcast_ref::<sqlx::mysql::MySqlDatabaseError>()
        .is_some_and(|mysql| {
            ALREADY_EXISTS_ERROR_NUMBERS.contains(&mysql.number())
        })
}

/// Whether a request's claimed scope is the one this caller is entitled to.
///
/// Split out from the middleware because the middleware itself cannot be
/// exercised without a live `SdkAuthApp` and `LibraryApp` to build a
/// `LibraryExecutor` from, and this comparison is the part that decides whether
/// one signed-in user can read another's documents.
///
/// A `None` claim is a rejection, not an abstention -- see the note at the call
/// site: `engine_debug_state` substitutes Photon's own defaults for a missing
/// `tenant_id`/`workspace_id`, so passing an unreadable claim through would
/// serve whatever those defaults name.
fn scope_belongs_to_caller(
    caller_id: &str,
    claimed: Option<&(String, String)>,
) -> bool {
    let Some((claimed_tenant, claimed_workspace)) = claimed else {
        return false;
    };
    *claimed_tenant == engine_tenant()
        && *claimed_workspace == caller_workspace_id(caller_id)
}

/// Require a credential library-api recognises, and pin the scope to it.
async fn require_engine_caller(request: Request, next: Next) -> Response {
    if request.uri().path() == ENGINE_HEALTH_PATH {
        return next.run(request).await;
    }

    let (mut parts, body) = request.into_parts();

    // Turn an anonymous caller away before doing any work. The extractor below
    // reaches for two extensions and, given a real token, calls out to
    // tachyon-api to verify it; none of that is owed to a request carrying no
    // credential at all.
    if !has_bearer_credential(&parts) {
        return unauthenticated_response();
    }

    let executor =
        match <LibraryExecutor as FromRequestParts<()>>::from_request_parts(
            &mut parts,
            &(),
        )
        .await
        {
            Ok(executor) => executor,
            Err(error) => return error.into_response(),
        };
    if executor.is_none() {
        return unauthenticated_response();
    }

    let bytes =
        match axum::body::to_bytes(body, MAX_ENGINE_BODY_BYTES).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Photon Engine request body is too large",
                )
                    .into_response()
            }
        };

    // The tenant *and* the workspace both have to be this caller's. Checking
    // only the tenant would leave every signed-in user free to name another
    // user's workspace, which is the whole separation: `pull` would hand back
    // their operations and `debug` their collection and record counts.
    //
    // Unlike the tenant check this used to be, a request whose scope cannot be
    // read is now rejected rather than left to Photon. Absence is not
    // innocent here: `engine_debug_state` defaults a missing `workspace_id` to
    // Photon's own `default`, and letting the request through on the grounds
    // that we could not see a claim would serve whatever that default holds.
    // Photon would have answered 400 to most of these anyway; 403 is the
    // stricter reading of the same request.
    let caller_id = executor.get_id();
    let claimed = request_scope(&parts, &bytes);
    if !scope_belongs_to_caller(caller_id, claimed.as_ref()) {
        tracing::warn!(
            claimed = ?claimed,
            expected_tenant = %engine_tenant(),
            expected_workspace = %caller_workspace_id(caller_id),
            "rejecting Photon Engine request for another scope"
        );
        return errors::Error::forbidden(
            "Photon Engine scope names another tenant or workspace",
        )
        .into_response();
    }

    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

fn unauthenticated_response() -> Response {
    errors::Error::unauthenticated(
        "Photon Engine sync requires an authenticated caller",
    )
    .into_response()
}

/// Whether the request carries an `Authorization: Bearer` credential at all.
///
/// Matched case-insensitively, as `sdk_auth::bearer_token_from_headers` and the
/// `Authorization<Bearer>` extractor both do: matching the scheme exactly here
/// would 401 a request the extractor would have authenticated.
fn has_bearer_credential(parts: &Parts) -> bool {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(char::is_whitespace))
        .is_some_and(|(scheme, token)| {
            scheme.eq_ignore_ascii_case("Bearer")
                && !token.trim().is_empty()
        })
}

#[derive(serde::Deserialize)]
struct ScopedBody {
    scope: String,
}

/// The scope a request claims, read from wherever *the handler* will read it.
///
/// That "wherever" is the whole point, and getting it wrong opens the boundary
/// rather than closing it. `push` and `pull` take `Json<..>` and read the body's
/// `scope`; `debug` takes `Query<ListParams>` and reads `tenant_id` and
/// `workspace_id`. Consulting the body first for every request meant a caller
/// could send `GET /api/engine/debug?tenant_id=other` with a body naming the
/// permitted scope: this function saw the body and allowed it, and the handler
/// then ignored that body and served `other` from the query.
///
/// So the source is chosen by method, not by what happens to parse. A method
/// that carries no body cannot be answered from one.
fn request_scope(parts: &Parts, body: &[u8]) -> Option<(String, String)> {
    if body_bearing_method(&parts.method) {
        let scoped = serde_json::from_slice::<ScopedBody>(body).ok()?;
        return parse_workspace_scope(&scoped.scope);
    }

    query_scope(&parts.uri)
}

/// Whether the handler for this method reads the request body.
///
/// GET and HEAD are the ones Photon serves from the query string. Anything else
/// is treated as body-bearing, so a route added upstream is guarded by the
/// stricter of the two readings rather than silently by neither.
fn body_bearing_method(method: &axum::http::Method) -> bool {
    !matches!(*method, axum::http::Method::GET | axum::http::Method::HEAD)
}

/// The `tenant_id` / `workspace_id` pair `engine_debug_state` reads.
///
/// Both must be present. Photon substitutes its own defaults for whichever is
/// missing, and those defaults belong to no caller, so an incomplete pair is
/// reported as no claim at all and the middleware rejects it.
fn query_scope(uri: &axum::http::Uri) -> Option<(String, String)> {
    let query = uri.query()?;
    let mut tenant = None;
    let mut workspace = None;
    for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
        if value.is_empty() {
            continue;
        }
        match key.as_ref() {
            "tenant_id" => tenant = Some(value.into_owned()),
            "workspace_id" => workspace = Some(value.into_owned()),
            _ => {}
        }
    }
    Some((tenant?, workspace?))
}

/// `tenant:{tenant}:workspace:{workspace}` -- the one scope shape Photon's HTTP
/// boundary accepts, parsed to the same strictness as
/// `photon_axum::parse_workspace_scope` so this check cannot pass something the
/// handler would then read differently.
fn parse_workspace_scope(scope: &str) -> Option<(String, String)> {
    let mut parts = scope.splitn(4, ':');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (
            Some("tenant"),
            Some(tenant),
            Some("workspace"),
            Some(workspace),
        ) if !tenant.is_empty()
            && !workspace.is_empty()
            && !workspace.contains(':') =>
        {
            Some((tenant.to_owned(), workspace.to_owned()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[test]
    fn parse_workspace_scope_reads_both_segments() {
        assert_eq!(
            parse_workspace_scope(
                "tenant:library:workspace:library-default-user-usr_01"
            ),
            Some((
                "library".to_owned(),
                "library-default-user-usr_01".to_owned()
            ))
        );
    }

    #[test]
    fn parse_workspace_scope_rejects_every_other_shape() {
        // Each of these would otherwise let a caller past the scope pin with a
        // scope the handler still parses, or parses differently.
        for malformed in [
            "workspace:library:library-default",
            "tenant:library",
            "tenant::workspace:library-default",
            "tenant:library:workspace:",
            "tenant:library:workspace:a:b",
            "",
        ] {
            assert_eq!(
                parse_workspace_scope(malformed),
                None,
                "scope {malformed:?} must not yield a scope"
            );
        }
    }

    /// The client builds this string; this side has to reproduce it byte for
    /// byte or every request 403s. The literal is pinned here and in
    /// `kitConfig.test.ts` so the two cannot drift apart silently.
    #[test]
    fn caller_workspace_id_matches_the_client_derivation() {
        assert_eq!(
            caller_workspace_id("usr_01j91h09tpj5ehwbwfwfxpak2b"),
            "library-default-user-usr_01j91h09tpj5ehwbwfwfxpak2b"
        );
    }

    #[test]
    fn caller_workspace_id_folds_characters_a_scope_cannot_carry() {
        // A `:` would split the scope into a shape `parse_workspace_scope`
        // rejects, so it must not survive into the workspace segment.
        assert_eq!(
            caller_workspace_id("a:b/c"),
            "library-default-user-a-b-c"
        );
    }

    /// Distinct callers must never land on one workspace.
    #[test]
    fn caller_workspace_id_separates_two_callers() {
        assert_ne!(
            caller_workspace_id("usr_alice"),
            caller_workspace_id("usr_bob")
        );
    }

    /// The base workspace names nobody, so a client that kept syncing under
    /// the old shared scope is rejected rather than silently served.
    #[test]
    fn the_bare_base_workspace_belongs_to_no_caller() {
        for caller in ["usr_alice", "usr_bob", ""] {
            assert_ne!(
                caller_workspace_id(caller),
                DEFAULT_ENGINE_WORKSPACE
            );
        }
    }

    #[test]
    fn a_push_is_read_from_its_body_scope() {
        let parts = post_parts(None);
        let body =
            br#"{"scope":"tenant:library:workspace:library-default"}"#;
        assert_eq!(
            request_scope(&parts, body),
            Some(("library".to_owned(), "library-default".to_owned()))
        );
    }

    #[test]
    fn a_debug_request_is_read_from_its_query() {
        let parts =
            parts_with_query(Some("tenant_id=library&workspace_id=w"));
        assert_eq!(
            request_scope(&parts, b""),
            Some(("library".to_owned(), "w".to_owned()))
        );
    }

    /// The debug handler reads the query and ignores the body, so a body
    /// naming the permitted scope must not speak for the request. It used to:
    /// this is the bypass that read another tenant's debug state.
    #[test]
    fn a_debug_body_cannot_speak_for_the_query() {
        let parts =
            parts_with_query(Some("tenant_id=other&workspace_id=w"));
        let body =
            br#"{"scope":"tenant:library:workspace:library-default"}"#;
        assert_eq!(
            request_scope(&parts, body),
            Some(("other".to_owned(), "w".to_owned())),
            "the query is what the debug handler reads, so it is what the \
             scope pin has to see"
        );
    }

    /// `engine_debug_state` defaults a missing `workspace_id` to Photon's own
    /// `default`, which belongs to no caller. Reporting the half-claim as a
    /// whole one would compare the tenant and let the workspace through
    /// unchecked, so an incomplete pair has to read as no claim at all.
    #[test]
    fn a_debug_query_missing_the_workspace_is_no_claim() {
        assert_eq!(
            request_scope(
                &parts_with_query(Some("tenant_id=library")),
                b""
            ),
            None
        );
        assert_eq!(
            request_scope(
                &parts_with_query(Some("workspace_id=library-default")),
                b""
            ),
            None
        );
    }

    /// The mirror image: a query parameter must not speak for a `push`, whose
    /// handler only ever reads the body.
    #[test]
    fn a_push_query_cannot_speak_for_the_body() {
        let parts = post_parts(Some(
            "tenant_id=library&workspace_id=library-default",
        ));
        assert_eq!(request_scope(&parts, b"not json"), None);
    }

    #[test]
    fn request_scope_is_absent_when_nothing_names_one() {
        assert_eq!(request_scope(&post_parts(None), b"not json"), None);
        assert_eq!(
            request_scope(&post_parts(None), br#"{"scope":"bad"}"#),
            None
        );
        assert_eq!(request_scope(&parts_with_query(None), b""), None);
    }

    /// The separation itself: Alice's credential must not reach Bob's
    /// workspace. `require_engine_caller` derives `caller_id` from a verified
    /// token, so this is the whole of the decision it makes afterwards.
    #[test]
    fn one_caller_cannot_name_another_callers_workspace() {
        let bob = ("library".to_owned(), caller_workspace_id("usr_bob"));
        assert!(!scope_belongs_to_caller("usr_alice", Some(&bob)));
        assert!(scope_belongs_to_caller("usr_bob", Some(&bob)));
    }

    #[test]
    fn the_old_shared_workspace_is_no_longer_reachable() {
        // What every client used to sync under. No caller may still name it,
        // or the separation would be opt-in.
        let shared =
            ("library".to_owned(), DEFAULT_ENGINE_WORKSPACE.to_owned());
        assert!(!scope_belongs_to_caller("usr_alice", Some(&shared)));
    }

    #[test]
    fn another_tenant_is_rejected_even_in_the_callers_own_workspace() {
        let other_tenant =
            ("other".to_owned(), caller_workspace_id("usr_alice"));
        assert!(!scope_belongs_to_caller("usr_alice", Some(&other_tenant)));
    }

    /// An unreadable claim fails closed. Photon would otherwise fall back to
    /// its own defaults and serve them.
    #[test]
    fn an_unreadable_claim_is_rejected() {
        assert!(!scope_belongs_to_caller("usr_alice", None));
    }

    /// An anonymous executor's id is the empty string. It must not
    /// accidentally be entitled to anything -- the bearer check already turns
    /// these away, so this pins the second line of defence.
    #[test]
    fn an_empty_caller_id_is_entitled_to_no_real_workspace() {
        let alice =
            ("library".to_owned(), caller_workspace_id("usr_alice"));
        assert!(!scope_belongs_to_caller("", Some(&alice)));
    }

    #[test]
    fn engine_tenant_defaults_to_the_client_default() {
        // `apps/client` sends `library`; Photon's own default is `photon`.
        assert_eq!(DEFAULT_ENGINE_TENANT, "library");
    }

    #[test]
    fn bearer_credential_is_detected_case_insensitively() {
        assert!(has_bearer_credential(&parts_with_auth(Some(
            "Bearer abc"
        ))));
        assert!(has_bearer_credential(&parts_with_auth(Some(
            "bearer abc"
        ))));
        assert!(!has_bearer_credential(&parts_with_auth(Some(
            "Bearer   "
        ))));
        assert!(!has_bearer_credential(&parts_with_auth(Some(
            "Basic abc"
        ))));
        assert!(!has_bearer_credential(&parts_with_auth(None)));
    }

    /// The security property this middleware exists for. Nothing else on the
    /// router rejects an anonymous caller -- `caller_token_middleware` only
    /// propagates the token -- so if this regresses, `/api/engine/*` is open to
    /// the internet.
    #[tokio::test]
    async fn an_anonymous_push_is_rejected_before_it_reaches_the_engine() {
        let response = guarded_probe()
            .oneshot(
                axum::http::Request::post("/api/engine/push")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope":"tenant:library:workspace:library-default","operations":[]}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn an_anonymous_pull_is_rejected_too() {
        let response = guarded_probe()
            .oneshot(
                axum::http::Request::post("/api/engine/pull")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"scope":"tenant:library:workspace:library-default"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    /// Liveness stays open so an unauthenticated probe -- including the smoke
    /// script's -- still answers.
    #[tokio::test]
    async fn health_answers_without_a_credential() {
        let response = guarded_probe()
            .oneshot(
                axum::http::Request::get(ENGINE_HEALTH_PATH)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// The middleware over stand-in handlers, so a rejection is provably the
    /// middleware's and not the Engine's. Reaching a handler answers `200`.
    fn guarded_probe() -> Router {
        use axum::routing::{get, post};

        Router::new()
            .route(ENGINE_HEALTH_PATH, get(|| async { "reached" }))
            .route("/api/engine/push", post(|| async { "reached" }))
            .route("/api/engine/pull", post(|| async { "reached" }))
            .layer(axum::middleware::from_fn(require_engine_caller))
    }

    // -----------------------------------------------------------------
    // MySQL-backed. Skipped unless PHOTON_ENGINE_TEST_DATABASE_URL points at a
    // throwaway database, because these write real tables:
    //
    //   docker run -d --name photon-engine-test -e MYSQL_ROOT_PASSWORD=secret \
    //     -e MYSQL_DATABASE=library -p 43306:3306 mysql:8.4
    //   PHOTON_ENGINE_TEST_DATABASE_URL=mysql://root:secret@127.0.0.1:43306/library \
    //     cargo test -p library-api --lib photon_engine
    // -----------------------------------------------------------------

    const TEST_DATABASE_URL_ENV: &str = "PHOTON_ENGINE_TEST_DATABASE_URL";

    async fn engine_state_or_skip() -> Option<Arc<AppState>> {
        let Ok(url) = std::env::var(TEST_DATABASE_URL_ENV) else {
            eprintln!("skipping: {TEST_DATABASE_URL_ENV} is not set");
            return None;
        };
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .expect("test database should accept connections");
        Some(
            build_engine_state(pool)
                .await
                .expect("engine state should build against a real MySQL"),
        )
    }

    async fn post_json(
        app: &Router,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::post(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = serde_json::from_slice(&bytes)
            .unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    fn upsert_operation(
        scope: &str,
        record_id: &str,
        operation_id: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": operation_id,
            "key": { "scope": scope, "collection": "documents", "record_id": record_id },
            "actor_id": "library-api-test",
            "timestamp": { "wall_time_ms": 1, "counter": 0, "actor_id": "library-api-test" },
            "kind": { "type": "upsert", "value": { "id": record_id, "title": "from the test" } },
            "metadata": {},
        })
    }

    /// The bug this module exists for: a push used to 404. It must now be
    /// accepted, persisted, and readable back by pull.
    #[tokio::test]
    async fn a_push_is_accepted_and_pulled_back() {
        let Some(state) = engine_state_or_skip().await else {
            return;
        };
        let app = photon_axum::engine_routes().with_state(state);

        let scope = "tenant:library:workspace:library-default";
        let record_id = format!("doc-{}", uuid::Uuid::new_v4());
        let operation_id = format!("op_{}", uuid::Uuid::new_v4());

        let (status, push) = post_json(
            &app,
            "/api/engine/push",
            serde_json::json!({
                "scope": scope,
                "operations": [upsert_operation(scope, &record_id, &operation_id)],
                "cursor": null,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "push returned {push}");

        let decision = push["decisions"]
            .as_array()
            .and_then(|decisions| decisions.first())
            .expect("push should return one decision");
        assert_eq!(decision["type"], "accepted", "push returned {push}");
        assert_eq!(decision["operation_id"], operation_id.as_str());
        let remote_sequence = decision["remote_sequence"]
            .as_i64()
            .expect("an accepted operation carries a remote sequence");

        let (status, pull) = post_json(
            &app,
            "/api/engine/pull",
            serde_json::json!({ "scope": scope, "cursor": null }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "pull returned {pull}");
        assert!(
            pull["operations"].as_array().is_some_and(|operations| {
                operations.iter().any(|entry| {
                    entry["operation"]["id"] == operation_id.as_str()
                        && entry["remote_sequence"] == remote_sequence
                })
            }),
            "pull did not return {operation_id} at sequence {remote_sequence}: {pull}"
        );
    }

    /// Sequence allocation has to stay in the database. library-api runs as
    /// Lambda, so several instances share one TiDB: a process-local counter
    /// would hand out one sequence twice, and a pull that ran in the gap would
    /// skip the other permanently. Concurrent pushes through one state stand in
    /// for concurrent instances -- if the counter were in the process, this is
    /// where duplicates would show up.
    #[tokio::test]
    async fn concurrent_pushes_each_get_their_own_sequence() {
        let Some(state) = engine_state_or_skip().await else {
            return;
        };
        let app = photon_axum::engine_routes().with_state(state);

        let scope = "tenant:library:workspace:library-default";
        let pushes = (0..8).map(|_| {
            let app = app.clone();
            let record_id = format!("doc-{}", uuid::Uuid::new_v4());
            let operation_id = format!("op_{}", uuid::Uuid::new_v4());
            async move {
                let (status, push) = post_json(
                    &app,
                    "/api/engine/push",
                    serde_json::json!({
                        "scope": scope,
                        "operations": [upsert_operation(scope, &record_id, &operation_id)],
                        "cursor": null,
                    }),
                )
                .await;
                assert_eq!(status, StatusCode::OK, "push returned {push}");
                push["decisions"][0]["remote_sequence"]
                    .as_i64()
                    .expect("an accepted operation carries a remote sequence")
            }
        });

        let mut sequences = futures::future::join_all(pushes).await;
        sequences.sort_unstable();
        let unique = sequences.len();
        sequences.dedup();
        assert_eq!(
            sequences.len(),
            unique,
            "two concurrent pushes were assigned the same remote sequence"
        );
    }

    /// A scope Photon's boundary refuses is refused before anything is stored,
    /// so the tenant pin above cannot be sidestepped with a malformed scope.
    #[tokio::test]
    async fn a_malformed_scope_is_rejected_by_the_engine() {
        let Some(state) = engine_state_or_skip().await else {
            return;
        };
        let app = photon_axum::engine_routes().with_state(state);

        let (status, _) = post_json(
            &app,
            "/api/engine/pull",
            serde_json::json!({ "scope": "workspace:library-default", "cursor": null }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    fn parts_with_auth(header: Option<&str>) -> Parts {
        let mut builder =
            axum::http::Request::builder().uri("/api/engine/push");
        if let Some(header) = header {
            builder =
                builder.header(axum::http::header::AUTHORIZATION, header);
        }
        builder.body(()).unwrap().into_parts().0
    }

    /// A `POST /api/engine/push`, optionally carrying a query string it has no
    /// business being read from.
    fn post_parts(query: Option<&str>) -> Parts {
        let uri = match query {
            Some(query) => format!("/api/engine/push?{query}"),
            None => "/api/engine/push".to_owned(),
        };
        axum::http::Request::post(uri)
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    /// A `GET /api/engine/debug`, which is what `Request::builder` defaults to.
    fn parts_with_query(query: Option<&str>) -> Parts {
        let uri = match query {
            Some(query) => format!("/api/engine/debug?{query}"),
            None => "/api/engine/debug".to_owned(),
        };
        axum::http::Request::builder()
            .uri(uri)
            .body(())
            .unwrap()
            .into_parts()
            .0
    }
}
