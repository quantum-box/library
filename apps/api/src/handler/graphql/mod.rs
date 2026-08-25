use std::sync::Arc;

use super::library_executor_extractor::{
    LibraryExecutor, LibraryMultiTenancy,
};
use crate::sdk_auth::SdkAuthApp;
use async_graphql::{EmptySubscription, Schema};
use axum::body::Body;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use inbound_sync::adapter::{LibrarySyncMutation, LibrarySyncQuery};

pub mod input;
pub mod model;
mod mutation;
mod resolver;
mod user_resolver;

#[derive(Clone)]
pub(crate) struct IntegrationQueryState {
    pub list_integrations:
        Arc<dyn inbound_sync::usecase::ListIntegrationsInputPort>,
    pub list_connections:
        Arc<dyn inbound_sync::usecase::ListConnectionsInputPort>,
}

pub(crate) fn log_graphql_operation_error(
    operation: &'static str,
    _error: &errors::Error,
) {
    tracing::warn!(operation, "graphql operation context");
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("graphql".to_string()),
        message: Some("graphql operation context".to_string()),
        level: sentry::Level::Warning,
        data: [("operation".to_string(), operation.into())]
            .into_iter()
            .collect(),
        ..Default::default()
    });
}

/// The tachyon action a Library tenant seed actually performs.
///
/// Seeding attaches Library policies to every member of the tenant, so
/// this is exactly the permission the caller needs there.
pub(crate) const TENANT_SEED_ACTION: &str = "auth:AttachUserPolicy";

/// Whether the caller may import `tenant_id` into Library.
///
/// Tachyon authorizes by policy, not by the `role` field on a user
/// record: that field is a label the API renders in upper case
/// (`OWNER`), and even when it parses it says nothing about what the
/// user is allowed to do in the tenant. Asking the auth service to
/// evaluate the action the seed performs keeps this answer in step
/// with whatever policies the tenant actually grants.
pub(crate) async fn caller_can_seed_tenant(
    auth_app: &dyn tachyon_sdk::auth::AuthApp,
    executor: &dyn tachyon_sdk::auth::ExecutorAction,
    tenant_id: &value_object::TenantId,
) -> errors::Result<bool> {
    let tenant_scope = tachyon_sdk::auth::MultiTenancy::new(
        Some(crate::domain::LIBRARY_TENANT.clone()),
        Some(tenant_id.clone()),
    );
    let outcomes = auth_app
        .evaluate_policies_batch(
            &tachyon_sdk::auth::EvaluatePoliciesBatchInput {
                executor,
                multi_tenancy: &tenant_scope,
                actions: &[TENANT_SEED_ACTION],
            },
        )
        .await?;

    Ok(outcomes.iter().any(|outcome| {
        outcome.action == TENANT_SEED_ACTION && outcome.allowed
    }))
}

/// How many users the tenant has, or `None` when the caller could not
/// count them.
///
/// Listing a tenant's users is itself a permissioned call inside that
/// tenant, and a caller who merely belongs to it does not always hold
/// that permission. Reporting the failure as `0` made "this tenant has
/// no members" indistinguishable from "we were not allowed to look",
/// which is why the failure is reported as an absent count instead.
pub(crate) async fn count_tenant_staff(
    auth_app: &dyn tachyon_sdk::auth::AuthApp,
    executor: &dyn tachyon_sdk::auth::ExecutorAction,
    multi_tenancy: &dyn tachyon_sdk::auth::MultiTenancyAction,
    tenant_id: &value_object::TenantId,
) -> Option<i32> {
    match tachyon_sdk::auth::AuthApp::find_users_by_tenant(
        auth_app,
        &tachyon_sdk::auth::FindUsersByTenantInput {
            executor,
            multi_tenancy,
            tenant_id,
        },
    )
    .await
    {
        Ok(users) => Some(users.len() as i32),
        Err(error) => {
            tracing::warn!(
                tenant = %tenant_id,
                error = ?error,
                "could not count the tenant's staff"
            );
            None
        }
    }
}

#[derive(async_graphql::MergedObject, Default)]
pub struct Query(resolver::LibraryQuery, LibrarySyncQuery);

#[derive(async_graphql::MergedObject, Default)]
pub struct Mutation(mutation::LibraryMutation, LibrarySyncMutation);

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

#[allow(dead_code)]
pub async fn graphiql() -> impl axum::response::IntoResponse {
    axum::response::Html(
        async_graphql::http::GraphiQLSource::build()
            .endpoint("/v1/graphql")
            .finish(),
    )
}

/// GraphQL handler that uses library-api's own extractors
/// (LibraryExecutor / LibraryMultiTenancy) instead of the
/// auth crate's FromRequestParts implementations which
/// require Extension<Arc<tachyon_sdk::auth::AuthApp>>.
///
/// When the caller provides a Bearer token (JWT or pk_*),
/// a request-scoped SdkAuthApp is created that forwards the
/// caller's token to tachyon-api. This ensures user-scoped
/// operations (check_policy, etc.) evaluate the correct
/// user's policies.
pub async fn graphql_handler(
    executor: LibraryExecutor,
    multi_tenancy: LibraryMultiTenancy,
    Extension(base_sdk): Extension<Arc<SdkAuthApp>>,
    schema: axum::Extension<AppSchema>,
    axum::Json(gql_req): axum::Json<async_graphql::Request>,
) -> impl IntoResponse {
    let caller_auth = executor.caller_auth_app(&base_sdk).ok();
    let auth_executor: tachyon_sdk::auth::Executor = executor.into();

    let mut req = gql_req.data(auth_executor).data(multi_tenancy.0);

    // If the caller provided a Bearer token, create a
    // request-scoped SdkAuthApp that forwards it to
    // tachyon-api. This shadows the schema-level instances
    // so resolvers using either Arc<SdkAuthApp> or
    // Arc<dyn AuthApp> get the request-scoped version.
    if let Some(caller_auth) = caller_auth {
        let scoped_sdk = caller_auth.sdk_app();
        let scoped_auth = caller_auth.auth_app();
        req = req.data(caller_auth).data(scoped_sdk).data(scoped_auth);
    }

    let resp = schema.execute(req).await;
    let body = serde_json::to_string(&resp).unwrap_or_else(|_| {
        r#"{"errors":[{"message":"serialization failed"}]}"#.to_string()
    });

    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

pub async fn graphql_introspection(
    Extension(schema): Extension<AppSchema>,
) -> String {
    schema.clone().sdl().as_str().to_string()
}

#[cfg(test)]
mod tests {
    use super::count_tenant_staff;
    use chrono::Utc;
    use tachyon_sdk::auth::{DefaultRole, MockAuthApp, MultiTenancy, User};
    use value_object::{TenantId, UserId};

    #[tokio::test]
    async fn staff_count_reports_the_members_it_can_list() {
        let mut auth_app = MockAuthApp::new();
        auth_app.expect_find_users_by_tenant().returning(|_| {
            let users = vec![member("alice"), member("bob")];
            Box::pin(async move { Ok(users) })
        });

        assert_eq!(
            count_tenant_staff(
                &auth_app,
                &executor(),
                &multi_tenancy(),
                &tenant(),
            )
            .await,
            Some(2)
        );
    }

    /// A tenant nobody has joined is a real answer, and must not be
    /// confused with the unreadable case below.
    #[tokio::test]
    async fn staff_count_reports_an_empty_tenant_as_zero() {
        let mut auth_app = MockAuthApp::new();
        auth_app
            .expect_find_users_by_tenant()
            .returning(|_| Box::pin(async move { Ok(Vec::new()) }));

        assert_eq!(
            count_tenant_staff(
                &auth_app,
                &executor(),
                &multi_tenancy(),
                &tenant(),
            )
            .await,
            Some(0)
        );
    }

    /// Regression guard for PLT-3886: the import wizard showed
    /// `0 members` for tenants whose user list the caller may not read.
    #[tokio::test]
    async fn staff_count_is_absent_when_the_members_cannot_be_listed() {
        let mut auth_app = MockAuthApp::new();
        auth_app.expect_find_users_by_tenant().returning(|_| {
            Box::pin(async move {
                Err(errors::permission_denied!(
                    "not allowed to list users in this tenant"
                ))
            })
        });

        assert_eq!(
            count_tenant_staff(
                &auth_app,
                &executor(),
                &multi_tenancy(),
                &tenant(),
            )
            .await,
            None
        );
    }

    fn tenant() -> TenantId {
        TenantId::new("tn_01j702qf86pc2j35s0kv0gv3gy").unwrap()
    }

    fn multi_tenancy() -> MultiTenancy {
        MultiTenancy::new(
            Some(crate::domain::LIBRARY_TENANT.clone()),
            Some(tenant()),
        )
    }

    fn executor() -> tachyon_sdk::auth::Executor {
        tachyon_sdk::auth::Executor::User(Box::new(member("caller")))
    }

    fn member(username: &str) -> User {
        let now = Utc::now();
        User {
            id: UserId::new("us_01hs2yepy5hw4rz8pdq2wywnwt").unwrap(),
            username: username.to_string(),
            tenants: vec![tenant()],
            email: Some(format!("{username}@example.com")),
            name: Some(username.to_string()),
            email_verified: None,
            image: None,
            role: DefaultRole::General,
            metadata: None,
            created_at: now,
            updated_at: now,
        }
    }
}
