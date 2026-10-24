//! Online-only strategy read boundary. Every PM read checks both current
//! Library ACL and current Tachyon authorization; there is no offline cache.
#[path = "strategy_migration.rs"]
mod migration_api;
use crate::{
    handler::library_executor_extractor::LibraryExecutor,
    usecase::{LibraryOrg, ViewDataInputData, ViewRepoInputData},
    LibraryApp,
};
use axum::{
    extract::{Extension, Path, Query},
    http::{header, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::MySqlPool;
use std::{sync::Arc, time::Duration};
use strategy_projection::{
    store::{Binding, DocumentRelease, Store},
    ChangePage, ResourceRef,
};
use tachyon_sdk::auth::{CheckPolicyForResourceInput, ExecutorAction};

#[derive(Clone)]
pub struct StrategyState {
    store: Store,
    client: reqwest::Client,
    origin: Option<url::Url>,
}
impl StrategyState {
    pub fn new(pool: MySqlPool) -> Self {
        let allow_http =
            std::env::var("LIBRARY_STRATEGY_ALLOW_INSECURE_HTTP")
                .as_deref()
                == Ok("true");
        let origin = std::env::var("TACHYON_API_URL")
            .ok()
            .and_then(|v| url::Url::parse(&v).ok())
            .filter(|u| {
                u.scheme() == "https"
                    || (u.scheme() == "http"
                        && (allow_http
                            || matches!(
                                u.host_str(),
                                Some("localhost" | "127.0.0.1")
                            )))
            });
        Self {
            store: Store::new(pool),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("HTTP client"),
            origin,
        }
    }
    async fn upstream(
        &self,
        executor: &LibraryExecutor,
        tenant: &str,
        sub: &str,
        tail: &str,
        body: Option<Value>,
    ) -> errors::Result<Value> {
        let mut url = self.origin.clone().ok_or_else(|| {
            errors::Error::service_unavailable(
                "Tachyon origin not configured",
            )
        })?;
        {
            let mut segments = url.path_segments_mut().map_err(|_| {
                errors::Error::invalid("invalid Tachyon URL")
            })?;
            segments.clear().extend([
                "v1beta",
                tenant,
                "pm-sync",
                "subscriptions",
                sub,
            ]);
            if !tail.is_empty() {
                segments.extend(tail.split('/'));
            }
        }
        let token =
            executor.original_token.as_deref().ok_or_else(|| {
                errors::Error::permission_denied("Caller token required")
            })?;
        let mut request = if body.is_some()
            && !["changes", "resources", "relations"].contains(&tail)
        {
            self.client.post(url)
        } else {
            self.client.get(url)
        };
        request =
            request.bearer_auth(token).header("x-operator-id", tenant);
        if let Some(body) = body {
            if ["changes", "resources", "relations"].contains(&tail) {
                let mut query =
                    vec![("limit".to_string(), "100".to_string())];
                for key in ["generation", "sequence", "offset"] {
                    if let Some(value) =
                        body.get(key).and_then(Value::as_u64)
                    {
                        query.push((key.into(), value.to_string()));
                    }
                }
                request = request.query(&query);
            } else {
                request = request.json(&body);
            }
        }
        let response = request.send().await.map_err(|_| {
            errors::Error::service_unavailable("Tachyon unavailable")
        })?;
        if matches!(response.status().as_u16(), 401 | 403 | 404) {
            return Err(errors::Error::permission_denied(
                "Subscription is not accessible",
            ));
        }
        if response.status() == reqwest::StatusCode::CONFLICT {
            // Error bodies are bounded by the same reader as success bodies.
            let error = bounded_json(response).await?;
            let code = error
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("Conflict");
            return Err(errors::Error::conflict(code));
        }
        if !response.status().is_success() {
            return Err(errors::Error::service_unavailable(
                "Tachyon request failed",
            ));
        }
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(Value::Null);
        }
        bounded_json(response).await
    }
    /// Read-through refresh needs upstream read permission only. It never
    /// stores the caller token and never acknowledges with elevated rights.
    async fn refresh_before_read(
        &self,
        repo: &str,
        executor: &LibraryExecutor,
    ) -> errors::Result<bool> {
        let b = self.authorized_binding(repo, executor).await?;
        let result=self.upstream(executor,&b.tenant_id,&b.subscription_id,"changes",Some(json!({"generation":b.generation,"sequence":b.checkpoint}))).await;
        match result {
            Ok(value) => {
                let wire: strategy_projection::wire::Page =
                    serde_json::from_value(value).map_err(|_| {
                        errors::Error::service_unavailable(
                            "Invalid PM page",
                        )
                    })?;
                let pending = wire.has_more;
                let page =
                    wire.into_page(&b.workspace_id).map_err(map_error)?;
                self.store
                    .apply_page(repo, &page)
                    .await
                    .map_err(map_error)?;
                Ok(pending)
            }
            Err(errors::Error::Conflict { ref message, .. })
                if message.ends_with("ResyncRequired") =>
            {
                self.restore_snapshot(repo, executor, &b).await?;
                Ok(false)
            }
            Err(error @ errors::Error::Forbidden { .. }) => {
                self.store
                    .pause_projection(repo)
                    .await
                    .map_err(map_error)?;
                Err(error)
            }
            Err(error) => Err(error),
        }
    }
    async fn restore_snapshot(
        &self,
        repo: &str,
        executor: &LibraryExecutor,
        b: &Binding,
    ) -> errors::Result<(u64, u64)> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut pinned = None;
        for tail in ["resources", "relations"] {
            let mut offset = 0;
            for page_number in 0..100 {
                let mut query = json!({"offset":offset});
                if let Some((generation, sequence)) = pinned {
                    query["generation"] = json!(generation);
                    query["sequence"] = json!(sequence);
                }
                let page = self
                    .upstream(
                        executor,
                        &b.tenant_id,
                        &b.subscription_id,
                        tail,
                        Some(query),
                    )
                    .await?;
                let generation = page
                    .get("generation")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        errors::Error::service_unavailable(
                            "missing generation",
                        )
                    })?;
                let sequence = page
                    .get("sequence")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| {
                        errors::Error::service_unavailable(
                            "missing sequence",
                        )
                    })?;
                if page.get("complete").and_then(Value::as_bool)
                    != Some(true)
                    || pinned.is_some_and(|p| p != (generation, sequence))
                {
                    return Err(errors::Error::conflict(
                        "Snapshot incomplete or changed",
                    ));
                }
                pinned = Some((generation, sequence));
                if tail == "resources" {
                    nodes.extend(
                        serde_json::from_value::<
                            Vec<strategy_projection::Projection>,
                        >(
                            page.get("nodes").cloned().ok_or_else(
                                || {
                                    errors::Error::service_unavailable(
                                        "missing nodes",
                                    )
                                },
                            )?,
                        )
                        .map_err(|_| {
                            errors::Error::service_unavailable(
                                "invalid nodes",
                            )
                        })?,
                    );
                } else {
                    edges.extend(
                        serde_json::from_value::<
                            Vec<strategy_projection::wire::WireRelation>,
                        >(
                            page.get("edges").cloned().ok_or_else(
                                || {
                                    errors::Error::service_unavailable(
                                        "missing edges",
                                    )
                                },
                            )?,
                        )
                        .map_err(|_| {
                            errors::Error::service_unavailable(
                                "invalid edges",
                            )
                        })?,
                    );
                }
                match page.get("next_offset").and_then(Value::as_u64) {
                    None => break,
                    Some(next) if next > offset && page_number < 99 => {
                        offset = next
                    }
                    _ => {
                        return Err(errors::Error::conflict(
                            "Snapshot page limit or nonadvancing cursor",
                        ))
                    }
                }
            }
        }
        let (generation, sequence) = pinned.ok_or_else(|| {
            errors::Error::conflict("empty snapshot metadata")
        })?;
        let after = self
            .upstream(executor, &b.tenant_id, &b.subscription_id, "", None)
            .await?;
        let mut current = b.clone();
        current.generation = generation;
        validate_subscription(&after, &current)?;
        if after.get("sequence").and_then(Value::as_u64) != Some(sequence) {
            return Err(errors::Error::conflict("Snapshot changed; retry"));
        }
        self.store
            .replace_snapshot(repo, generation, sequence, nodes, edges)
            .await
            .map_err(map_error)?;
        Ok((generation, sequence))
    }
    async fn validate_resources(
        &self,
        repo: &str,
        executor: &LibraryExecutor,
        refs: Vec<ResourceRef>,
    ) -> errors::Result<()> {
        let binding = self.store.binding(repo).await.map_err(map_error)?;
        for chunk in refs.chunks(100) {
            let result = self
                .upstream(
                    executor,
                    &binding.tenant_id,
                    &binding.subscription_id,
                    "validate-resources",
                    Some(json!({"resource_refs":chunk})),
                )
                .await;
            match result {
                Ok(value)
                    if value.get("authorized").and_then(Value::as_bool)
                        == Some(true)
                        && value
                            .get("generation")
                            .and_then(Value::as_u64)
                            == Some(binding.generation) => {}
                Ok(_) | Err(errors::Error::Forbidden { .. }) => {
                    self.store
                        .pause_projection(repo)
                        .await
                        .map_err(map_error)?;
                    return Err(errors::Error::permission_denied(
                        "Resource access was revoked",
                    ));
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
    async fn authorized_binding(
        &self,
        repo: &str,
        executor: &LibraryExecutor,
    ) -> errors::Result<Binding> {
        let b = self.store.binding(repo).await.map_err(map_error)?;
        if b.writer != "tachyon" {
            return Err(errors::Error::permission_denied(
                "Projection is paused",
            ));
        }
        let value = match self
            .upstream(executor, &b.tenant_id, &b.subscription_id, "", None)
            .await
        {
            Ok(value) => value,
            Err(error @ errors::Error::Forbidden { .. }) => {
                self.store
                    .pause_projection(repo)
                    .await
                    .map_err(map_error)?;
                return Err(error);
            }
            Err(error) => return Err(error),
        };
        if let Err(error) = validate_subscription(&value, &b) {
            self.store.pause_projection(repo).await.map_err(map_error)?;
            return Err(error);
        }
        Ok(b)
    }
}
async fn bounded_json(
    mut response: reqwest::Response,
) -> errors::Result<Value> {
    const MAX_UPSTREAM_BODY: usize = 4 * 1024 * 1024;
    if response
        .content_length()
        .is_some_and(|size| size > MAX_UPSTREAM_BODY as u64)
    {
        return Err(errors::Error::service_unavailable(
            "PM response too large",
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        errors::Error::service_unavailable("Incomplete PM response")
    })? {
        if bytes.len().saturating_add(chunk.len()) > MAX_UPSTREAM_BODY {
            return Err(errors::Error::service_unavailable(
                "PM response too large",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| {
        errors::Error::service_unavailable("Invalid Tachyon response")
    })
}

fn validate_subscription(value: &Value, b: &Binding) -> errors::Result<()> {
    if value.get("state").and_then(Value::as_str) != Some("active")
        || value.get("generation").and_then(Value::as_u64)
            != Some(b.generation)
        || value.get("workspace_id").and_then(Value::as_str)
            != Some(b.workspace_id.as_str())
    {
        return Err(errors::Error::permission_denied(
            "Subscription scope or generation changed",
        ));
    }
    Ok(())
}
fn map_error(e: strategy_projection::Error) -> errors::Error {
    match e {
        strategy_projection::Error::Invalid(s) => errors::Error::invalid(s),
        strategy_projection::Error::Conflict(s) => {
            errors::Error::conflict(s)
        }
        strategy_projection::Error::Database(sqlx::Error::RowNotFound) => {
            errors::Error::not_found("Strategy resource")
        }
        other => errors::Error::internal_server_error(other.to_string()),
    }
}
async fn authorize(
    app: &LibraryApp,
    executor: &LibraryExecutor,
    org: &LibraryOrg,
    organization: &str,
    repository: &str,
    write: bool,
    private: bool,
) -> errors::Result<(String, String)> {
    if executor.is_none() {
        return Err(errors::Error::permission_denied(
            "Authentication required",
        ));
    }
    let repo = app
        .view_repo
        .execute(&ViewRepoInputData {
            executor,
            multi_tenancy: org,
            organization_username: organization.into(),
            repo_username: repository.into(),
        })
        .await?
        .repo;
    if private && *repo.is_public() {
        return Err(errors::Error::permission_denied(
            "PM projections require a private repository",
        ));
    }
    if write {
        app.auth_app
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor,
                multi_tenancy: org,
                action: "library:UpdateRepo",
                resource_trn: &format!("trn:library:repo:{}", repo.id()),
            })
            .await?;
    }
    Ok((repo.id().to_string(), repo.organization_id().to_string()))
}
pub fn router(pool: MySqlPool) -> Router {
    if std::env::var("LIBRARY_STRATEGY_ENABLED").as_deref() != Ok("true") {
        return Router::new();
    }
    Router::new()
        .route("/v1beta/repos/:org/:repo/pm-projection/migrations/dry-run",post(migration_api::dry_run))
        .route("/v1beta/repos/:org/:repo/pm-projection/migrations/:plan",get(migration_api::view))
        .route("/v1beta/repos/:org/:repo/pm-projection/migrations/:plan/apply",post(migration_api::apply))
        .route("/v1beta/repos/:org/:repo/pm-projection/migrations/:plan/rollback",post(migration_api::rollback))
        .route(
            "/v1beta/repos/:org/:repo/pm-projection/records/:id",
            get(record),
        )
        .route("/v1beta/repos/:org/:repo/pm-projection/:id", get(record))
        .route(
            "/v1beta/repos/:org/:repo/pm-projection",
            post(connect).get(list),
        )
        .route("/v1beta/repos/:org/:repo/pm-projection/pull", post(pull))
        .route(
            "/v1beta/repos/:org/:repo/pm-projection/resync",
            post(resync),
        )
        .route(
            "/v1beta/repos/:org/:repo/pm-projection/migration-inventory",
            get(inventory),
        )
        .route(
            "/v1beta/repos/:org/:repo/pm-projection/activate",
            post(activate),
        )
        .route("/v1beta/repos/:org/:repo/pm-projection/pause", post(pause))
        .route(
            "/v1beta/repos/:org/:repo/pm-projection/rollback",
            post(rollback),
        )
        .route(
            "/v1beta/repos/:org/:repo/pm-projection/resolve",
            post(resolve),
        )
        .route(
            "/v1beta/repos/:org/:repo/pm-projection/:id/context",
            post(context),
        )
        .route(
            "/v1beta/repos/:org/:repo/data/:data_id/releases",
            post(publish).get(list_releases),
        )
        .route("/v1beta/repos/:org/:repo/releases", get(policy_options))
        .route("/v1beta/repos/:org/:repo/releases/:id", get(release))
        .route("/v1beta/repos/:org/:repo/releases/:id/revoke", post(revoke))
        .layer(Extension(StrategyState::new(pool)))
        .layer(middleware::from_fn(no_store))
}
async fn no_store(
    request: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "private, no-store".parse().unwrap(),
    );
    response.headers_mut().insert(
        header::VARY,
        "Authorization, x-operator-id".parse().unwrap(),
    );
    response
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Connect {
    subscription_id: String,
    workspace_id: String,
    generation: u64,
}
async fn connect(
    Path((org, repo)): Path<(String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
    Json(p): Json<Connect>,
) -> errors::Result<Json<Binding>> {
    let (id, tenant) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    let b = Binding {
        tenant_id: tenant.clone(),
        subscription_id: p.subscription_id.clone(),
        workspace_id: p.workspace_id.clone(),
        generation: p.generation,
        checkpoint: 0,
        cursor: None,
        writer: "paused".into(),
    };
    validate_subscription(
        &state
            .upstream(&executor, &tenant, &p.subscription_id, "", None)
            .await?,
        &b,
    )?;
    Ok(Json(
        state
            .store
            .connect(
                &id,
                &tenant,
                &p.subscription_id,
                &p.workspace_id,
                p.generation,
            )
            .await
            .map_err(map_error)?,
    ))
}
#[derive(Deserialize)]
struct Paging {
    after: Option<String>,
    limit: Option<u32>,
}
async fn list(
    Path((org, repo)): Path<(String, String)>,
    Query(q): Query<Paging>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<strategy_projection::ProjectionList>> {
    let (id, _) =
        authorize(&app, &executor, &scope, &org, &repo, false, true)
            .await?;
    let pending = state.refresh_before_read(&id, &executor).await?;
    let mut list = state
        .store
        .list(&id, q.after.as_deref(), q.limit.unwrap_or(50))
        .await
        .map_err(map_error)?;
    let mut refs = list
        .records
        .iter()
        .map(|r| r.resource_ref.clone())
        .collect::<Vec<_>>();
    refs.extend(
        list.relations
            .iter()
            .flat_map(|e| [e.source.clone(), e.target.clone()]),
    );
    refs.sort();
    refs.dedup();
    state.validate_resources(&id, &executor, refs).await?;
    list.sync_pending = pending;
    Ok(Json(list))
}
async fn activate(
    Path((org, repo)): Path<(String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<Value>> {
    let (id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    let b = state.store.binding(&id).await.map_err(map_error)?;
    validate_subscription(
        &state
            .upstream(&executor, &b.tenant_id, &b.subscription_id, "", None)
            .await?,
        &b,
    )?;
    state
        .store
        .activate_tachyon_writer(&id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({"writer":"tachyon"})))
}
async fn pause(
    Path((org, repo)): Path<(String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<Value>> {
    let (id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    state.store.pause_projection(&id).await.map_err(map_error)?;
    Ok(Json(json!({"writer":"paused"})))
}
async fn rollback(
    Path((org, repo)): Path<(String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<Value>> {
    let (id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    let binding = state.store.binding(&id).await.map_err(map_error)?;
    validate_subscription(
        &state
            .upstream(
                &executor,
                &binding.tenant_id,
                &binding.subscription_id,
                "",
                None,
            )
            .await?,
        &binding,
    )?;
    state
        .store
        .restore_legacy_writer(&id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({"writer":"legacy"})))
}
async fn pull(
    Path((org, repo)): Path<(String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<Value>> {
    let (id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    let b = state.authorized_binding(&id, &executor).await?;
    // Exactly one bounded page per invocation. No stored user credentials and
    // no fire-and-forget work. A service-principal scheduler can call repeatedly.
    let page_value = state
        .upstream(
            &executor,
            &b.tenant_id,
            &b.subscription_id,
            "changes",
            Some(json!({"generation":b.generation,"sequence":b.checkpoint,"limit":100})),
        )
        .await?;
    let wire: strategy_projection::wire::Page =
        serde_json::from_value(page_value).map_err(|_| {
            errors::Error::service_unavailable("Invalid PM change page")
        })?;
    let page = wire.into_page(&b.workspace_id).map_err(map_error)?;
    let checkpoint = state
        .store
        .apply_page(&id, &page)
        .await
        .map_err(map_error)?;
    // Authorization is checked again before returning data/acknowledging. If
    // ack fails, next invocation replays safely from the persisted checkpoint.
    state.authorized_binding(&id, &executor).await?;
    state
        .upstream(
            &executor,
            &b.tenant_id,
            &b.subscription_id,
            "ack",
            Some(json!({"generation":b.generation,"sequence":checkpoint})),
        )
        .await?;
    Ok(Json(
        json!({"checkpoint":checkpoint,"next_cursor":page.next_cursor}),
    ))
}
async fn resolve(
    Path((org, repo)): Path<(String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
    Json(reference): Json<ResourceRef>,
) -> errors::Result<Json<Value>> {
    let (id, _) =
        authorize(&app, &executor, &scope, &org, &repo, false, true)
            .await?;
    state.refresh_before_read(&id, &executor).await?;
    state
        .validate_resources(&id, &executor, vec![reference.clone()])
        .await?;
    let record = state
        .store
        .resolve(&id, &reference)
        .await
        .map_err(map_error)?
        .ok_or_else(|| errors::Error::not_found("Projection"))?;
    Ok(Json(json!({"record_id":record,"resource_ref":reference})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    note: String,
    document_ids: Vec<String>,
}
async fn context(
    Path((org, repo, id)): Path<(String, String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
    Json(p): Json<Context>,
) -> errors::Result<Json<Value>> {
    let (repo_id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    state.authorized_binding(&repo_id, &executor).await?;
    for data_id in &p.document_ids {
        app.view_data
            .execute(&ViewDataInputData {
                executor: &executor,
                multi_tenancy: &scope,
                org_username: org.clone(),
                repo_username: repo.clone(),
                data_id: data_id.clone(),
            })
            .await?;
    }
    state
        .store
        .record_local_context(&repo_id, &id, &p.note, &p.document_ids)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({"saved":true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Publish {
    expected_record_version: String,
}
async fn publish(
    Path((org, repo, data_id)): Path<(String, String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
    Json(p): Json<Publish>,
) -> errors::Result<Json<DocumentRelease>> {
    let (repo_id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, false)
            .await?;
    let (data, properties) = app
        .view_data
        .execute(&ViewDataInputData {
            executor: &executor,
            multi_tenancy: &scope,
            org_username: org,
            repo_username: repo,
            data_id: data_id.clone(),
        })
        .await?;
    let source_version = data.record_version().to_string();
    if source_version != p.expected_record_version {
        return Err(errors::Error::conflict(
            "Document changed; reload before publishing",
        ));
    }
    let release = DocumentRelease {
        id: format!("rel_{}", uuid::Uuid::new_v4().simple()),
        data_id,
        source_version,
        title: data.name().to_string(),
        body: crate::usecase::markdown_composer::compose_markdown(
            &data,
            &properties,
        ),
        published_by: executor.get_id().to_string(),
        published_at: chrono::Utc::now().to_rfc3339(),
        revoked: false,
    };
    Ok(Json(
        state
            .store
            .publish(&repo_id, release)
            .await
            .map_err(map_error)?,
    ))
}
async fn release(
    Path((org, repo, id)): Path<(String, String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Response> {
    let (repo_id, _) =
        authorize(&app, &executor, &scope, &org, &repo, false, false)
            .await?;
    let release = state
        .store
        .release(&repo_id, &id)
        .await
        .map_err(map_error)?;
    app.view_data
        .execute(&ViewDataInputData {
            executor: &executor,
            multi_tenancy: &scope,
            org_username: org,
            repo_username: repo,
            data_id: release.data_id.clone(),
        })
        .await?;
    if release.revoked {
        return Ok((
            StatusCode::GONE,
            Json(json!({"status":"revoked","id":id})),
        )
            .into_response());
    }
    Ok(Json(release).into_response())
}
async fn revoke(
    Path((org, repo, id)): Path<(String, String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<Value>> {
    let (repo_id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, false)
            .await?;
    state
        .store
        .release(&repo_id, &id)
        .await
        .map_err(map_error)?;
    state
        .store
        .revoke_release(&repo_id, &id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({"status":"revoked"})))
}

#[derive(Deserialize)]
struct Snapshot {
    generation: u64,
    sequence: u64,
    nodes: Vec<strategy_projection::Projection>,
    edges: Vec<strategy_projection::wire::WireRelation>,
    complete: bool,
}
async fn resync(
    Path((org, repo)): Path<(String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<Value>> {
    let (id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    let b = state.store.binding(&id).await.map_err(map_error)?;
    let (generation, sequence) =
        state.restore_snapshot(&id, &executor, &b).await?;
    Ok(Json(json!({"generation":generation,"sequence":sequence})))
}

async fn inventory(
    Path((org, repo)): Path<(String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<Value>> {
    let (id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    Ok(Json(
        json!({"items":state.store.legacy_inventory(&id).await.map_err(map_error)?,"automatic_merge":false}),
    ))
}

async fn list_releases(
    Path((org, repo, data_id)): Path<(String, String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<Vec<DocumentRelease>>> {
    let (repo_id, _) =
        authorize(&app, &executor, &scope, &org, &repo, false, false)
            .await?;
    app.view_data
        .execute(&ViewDataInputData {
            executor: &executor,
            multi_tenancy: &scope,
            org_username: org,
            repo_username: repo,
            data_id: data_id.clone(),
        })
        .await?;
    let mut releases = state
        .store
        .releases_for_document(&repo_id, &data_id)
        .await
        .map_err(map_error)?;
    for release in &mut releases {
        if release.revoked {
            release.body.clear();
            release.title.clear();
        }
    }
    Ok(Json(releases))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding() -> Binding {
        Binding {
            tenant_id: "t".into(),
            subscription_id: "s".into(),
            workspace_id: "w".into(),
            generation: 1,
            checkpoint: 0,
            cursor: None,
            writer: "tachyon".into(),
        }
    }
    #[test]
    fn authorization_fails_closed_for_scope_generation_and_state() {
        let b = binding();
        assert!(validate_subscription(
            &json!({"state":"active","workspace_id":"w","generation":1}),
            &b
        )
        .is_ok());
        for value in [
            json!({}),
            json!({"state":"revoked","workspace_id":"w","generation":1}),
            json!({"state":"active","workspace_id":"other","generation":1}),
            json!({"state":"active","workspace_id":"w","generation":2}),
        ] {
            assert!(validate_subscription(&value, &b).is_err());
        }
    }
    #[tokio::test]
    #[ignore = "requires isolated LIBRARY_STRATEGY_TEST_DATABASE_URL and local TCP"]
    async fn authorized_read_refreshes_and_revocation_purges() {
        use super::super::library_executor_extractor::LibraryExecutorKind;
        use axum::http::HeaderMap;
        use std::sync::atomic::{AtomicBool, Ordering};
        let database =
            std::env::var("LIBRARY_STRATEGY_TEST_DATABASE_URL").unwrap();
        let target = url::Url::parse(&database).unwrap();
        assert!(
            matches!(target.host_str(), Some("127.0.0.1" | "localhost"))
                && target.path() == "/strategy_test"
        );
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect(&database)
            .await
            .unwrap();
        let store = Store::new(pool);
        let fixture:Value=serde_json::from_str(include_str!("../../../../packages/strategy_projection/tests/fixtures/tachyon-change-page.json")).unwrap();
        let denied = Arc::new(AtomicBool::new(false));
        let deny = denied.clone();
        let resource_denied = Arc::new(AtomicBool::new(false));
        let resource_deny = resource_denied.clone();
        let node = fixture["changes"][0]["projection"].clone();
        let mock=Router::new()
            .route("/v1beta/t/pm-sync/subscriptions/pms_fixture",get(move |headers:HeaderMap|{let deny=deny.clone();async move {
                assert_eq!(headers[header::AUTHORIZATION],"Bearer viewer");
                if deny.load(Ordering::SeqCst){return StatusCode::FORBIDDEN.into_response();}
                Json(json!({"state":"active","workspace_id":"workspace","generation":1,"sequence":1})).into_response()
            }}))
            .route("/v1beta/t/pm-sync/subscriptions/pms_fixture/changes",get(move |Query(q):Query<std::collections::BTreeMap<String,String>>|{let fixture=fixture.clone();async move {
                assert_eq!(q.get("generation").map(String::as_str),Some("1"));
                let changes=if q.get("sequence").map(String::as_str)==Some("0"){vec![fixture["changes"][0].clone()]}else{vec![]};
                Json(json!({"changes":changes,"next_cursor":{"generation":1,"sequence":1},"has_more":false}))
            }}));
        let mock = mock
            .route("/v1beta/t/pm-sync/subscriptions/pms_fixture/validate-resources",post(move |Json(body):Json<Value>|{let deny=resource_deny.clone();async move {
                assert!(!body["resource_refs"].as_array().unwrap().is_empty());
                if deny.load(Ordering::SeqCst){return StatusCode::FORBIDDEN.into_response();}
                Json(json!({"authorized":true,"generation":1,"scope_revision":"scope-v1"})).into_response()
            }}))
            .route("/v1beta/t/pm-sync/subscriptions/pms_fixture/resources",get(move |Query(q):Query<std::collections::BTreeMap<String,String>>|{let mut node=node.clone();async move {
                assert_eq!(q.get("limit").map(String::as_str),Some("100"));
                let second=q.get("offset").map(String::as_str)==Some("1");
                if second {assert_eq!(q.get("generation").map(String::as_str),Some("1"));assert_eq!(q.get("sequence").map(String::as_str),Some("1"));node["resource_ref"]["external_id"]=json!("second");}
                Json(json!({"generation":1,"sequence":1,"complete":true,"nodes":[node],"next_offset":if second{None}else{Some(1)}}))
            }}))
            .route("/v1beta/t/pm-sync/subscriptions/pms_fixture/relations",get(|Query(q):Query<std::collections::BTreeMap<String,String>>|async move {
                assert_eq!(q.get("generation").map(String::as_str),Some("1"));
                assert_eq!(q.get("sequence").map(String::as_str),Some("1"));
                Json(json!({"generation":1,"sequence":1,"complete":true,"edges":[],"next_offset":null}))
            }));
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = url::Url::parse(&format!(
            "http://{}",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, mock).await.unwrap()
        });
        let repo = format!("test_{}", uuid::Uuid::new_v4().simple());
        // A distinct subscription key permits independent repeated test runs.
        sqlx::query("DELETE FROM library_pm_bindings WHERE tenant_id='t' AND subscription_id='pms_fixture'").execute(&store_test_pool(&database).await).await.unwrap();
        store
            .connect(&repo, "t", "pms_fixture", "workspace", 1)
            .await
            .unwrap();
        store.activate_tachyon_writer(&repo).await.unwrap();
        let state = StrategyState {
            store: store.clone(),
            origin: Some(origin),
            client: reqwest::Client::new(),
        };
        let executor = LibraryExecutor {
            inner: LibraryExecutorKind::None,
            original_token: Some("viewer".into()),
        };
        assert!(!state
            .refresh_before_read(&repo, &executor)
            .await
            .unwrap());
        assert_eq!(
            store.list(&repo, None, 50).await.unwrap().records.len(),
            1
        );
        let refs = store
            .list(&repo, None, 50)
            .await
            .unwrap()
            .records
            .into_iter()
            .map(|r| r.resource_ref)
            .collect::<Vec<_>>();
        state
            .validate_resources(&repo, &executor, refs.clone())
            .await
            .unwrap();
        let binding = store.binding(&repo).await.unwrap();
        state
            .restore_snapshot(&repo, &executor, &binding)
            .await
            .unwrap();
        assert_eq!(
            store.list(&repo, None, 50).await.unwrap().records.len(),
            2
        );
        // An empty feed does not authorize a cached object after its ACL changes.
        assert!(!state
            .refresh_before_read(&repo, &executor)
            .await
            .unwrap());
        resource_denied.store(true, Ordering::SeqCst);
        assert!(state
            .validate_resources(&repo, &executor, refs)
            .await
            .is_err());
        assert!(store
            .list(&repo, None, 50)
            .await
            .unwrap()
            .records
            .is_empty());
        denied.store(true, Ordering::SeqCst);
        assert!(state.refresh_before_read(&repo, &executor).await.is_err());
        assert!(store
            .list(&repo, None, 50)
            .await
            .unwrap()
            .records
            .is_empty());
        assert_eq!(store.binding(&repo).await.unwrap().writer, "paused");
        server.abort();
    }
    async fn store_test_pool(url: &str) -> sqlx::MySqlPool {
        sqlx::mysql::MySqlPoolOptions::new()
            .connect(url)
            .await
            .unwrap()
    }
    #[tokio::test]
    async fn strategy_responses_are_never_cacheable() {
        use tower::ServiceExt;
        let router = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(middleware::from_fn(no_store));
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        assert_eq!(
            response.headers()[header::VARY],
            "Authorization, x-operator-id"
        );
    }
}

async fn record(
    Path((org, repo, id)): Path<(String, String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<strategy_projection::ProjectionRecord>> {
    let (repo_id, _) =
        authorize(&app, &executor, &scope, &org, &repo, false, true)
            .await?;
    state.refresh_before_read(&repo_id, &executor).await?;
    let record =
        state.store.record(&repo_id, &id).await.map_err(map_error)?;
    state
        .validate_resources(
            &repo_id,
            &executor,
            vec![record.resource_ref.clone()],
        )
        .await?;
    Ok(Json(record))
}

async fn policy_options(
    Path((org, repo)): Path<(String, String)>,
    Query(q): Query<Paging>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<Value>> {
    let (repo_id, _) =
        authorize(&app, &executor, &scope, &org, &repo, false, false)
            .await?;
    let limit = q.limit.unwrap_or(50);
    let candidates = state
        .store
        .release_candidates(&repo_id, q.after.as_deref(), limit)
        .await
        .map_err(map_error)?;
    let has_more = candidates.len() > limit as usize;
    let mut next_after = None;
    let mut releases = Vec::new();
    for candidate in candidates.into_iter().take(limit as usize) {
        next_after = Some(candidate.id.clone());
        match app
            .view_data
            .execute(&ViewDataInputData {
                executor: &executor,
                multi_tenancy: &scope,
                org_username: org.clone(),
                repo_username: repo.clone(),
                data_id: candidate.data_id.clone(),
            })
            .await
        {
            Ok(_) => releases.push(candidate),
            Err(
                errors::Error::NotFound { .. }
                | errors::Error::Forbidden { .. }
                | errors::Error::Unauthorized { .. },
            ) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(Json(
        json!({"releases":releases,"next_after":if has_more{next_after}else{None}}),
    ))
}
