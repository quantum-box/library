use std::sync::Arc;

use async_graphql::{EmptySubscription, Schema};
use axum::body::Body;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use inbound_sync::adapter::{LibrarySyncMutation, LibrarySyncQuery};
use tachyon_sdk::auth::AuthApp;

use super::library_executor_extractor::{
    LibraryExecutor, LibraryMultiTenancy,
};
use crate::sdk_auth::SdkAuthApp;

pub mod input;
pub mod model;
mod mutation;
mod resolver;
mod user_resolver;

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
    let original_token = executor.original_token.clone();
    let auth_executor: tachyon_sdk::auth::Executor = executor.into();

    let mut req = gql_req.data(auth_executor).data(multi_tenancy.0);

    // If the caller provided a Bearer token, create a
    // request-scoped SdkAuthApp that forwards it to
    // tachyon-api. This shadows the schema-level instances
    // so resolvers using either Arc<SdkAuthApp> or
    // Arc<dyn AuthApp> get the request-scoped version.
    if let Some(token) = original_token {
        let scoped_sdk = Arc::new(base_sdk.with_caller_token(&token));
        let scoped_auth: Arc<dyn AuthApp> = scoped_sdk.clone();
        req = req.data(scoped_sdk).data(scoped_auth);
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
