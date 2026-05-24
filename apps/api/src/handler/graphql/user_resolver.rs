use super::model::{Operator, User};
use crate::sdk_auth::SdkAuthApp;
use async_graphql::{Context, Result};
use std::collections::HashSet;
use std::sync::Arc;
use tachyon_sdk::auth::MultiTenancyAction;

#[async_graphql::ComplexObject]
impl User {
    /// Resolve operator organizations accessible to this user
    #[tracing::instrument(name = "organizations_by_user", skip_all)]
    async fn organizations(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<Operator>> {
        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;
        let _executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let user_id = &self.id;

        let mut operators = Vec::new();
        let mut seen = HashSet::new();

        // If platform_id is available, fetch
        // operators via REST.
        if let Some(platform_id) = multi_tenancy.platform_id() {
            match sdk.find_operators_by_user(&platform_id, user_id).await {
                Ok(resp_operators) => {
                    push_operator_responses(
                        resp_operators,
                        &mut seen,
                        &mut operators,
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        platform_id = %platform_id,
                        error = ?err,
                        "Failed to load platform operators; falling back to user tenants"
                    );
                    load_operators_from_tenants(
                        sdk,
                        &self.tenant_id_list,
                        &mut seen,
                        &mut operators,
                        Some(&platform_id),
                    )
                    .await;
                }
            }
        } else {
            // Fallback: iterate user's tenants and
            // resolve each operator by ID.
            load_operators_from_tenants(
                sdk,
                &self.tenant_id_list,
                &mut seen,
                &mut operators,
                None,
            )
            .await;
        }

        Ok(operators.into_iter().map(Into::into).collect())
    }
}

fn push_operator_responses(
    resp_operators: Vec<crate::sdk_auth::OperatorResp>,
    seen: &mut HashSet<String>,
    operators: &mut Vec<tachyon_sdk::auth::Operator>,
) {
    for op in resp_operators {
        if seen.insert(op.id.clone()) {
            match crate::sdk_auth::operator_from_resp(&op) {
                Ok(operator) => operators.push(operator),
                Err(err) => {
                    tracing::warn!(
                        operator_id = %op.id,
                        error = ?err,
                        "Failed to parse operator"
                    );
                }
            }
        }
    }
}

async fn load_operators_from_tenants(
    sdk: &SdkAuthApp,
    tenant_id_list: &[String],
    seen: &mut HashSet<String>,
    operators: &mut Vec<tachyon_sdk::auth::Operator>,
    platform_id: Option<&tachyon_sdk::auth::TenantId>,
) {
    for tenant_id in tenant_id_list {
        if seen.insert(tenant_id.clone()) {
            match sdk.get_operator(tenant_id).await {
                Ok(Some(op)) => {
                    if let Some(platform_id) = platform_id {
                        if op.platform_id != platform_id.as_str() {
                            tracing::warn!(
                                tenant_id = %tenant_id,
                                operator_platform_id = %op.platform_id,
                                platform_id = %platform_id,
                                "Skipping operator outside platform"
                            );
                            continue;
                        }
                    }

                    match crate::sdk_auth::operator_from_resp(&op) {
                        Ok(operator) => operators.push(operator),
                        Err(err) => {
                            tracing::warn!(
                                tenant_id = %tenant_id,
                                error = ?err,
                                "Failed to parse operator"
                            );
                        }
                    }
                }
                Ok(None) => {
                    tracing::warn!(
                        tenant_id = %tenant_id,
                        "Operator not found"
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        tenant_id = %tenant_id,
                        error = ?err,
                        "Failed to load operator"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::User;
    use crate::sdk_auth::SdkAuthApp;
    use async_graphql::{
        EmptyMutation, EmptySubscription, Object, Request, Schema,
    };
    use axum::{
        extract::{Path, Query, State},
        http::StatusCode,
        response::IntoResponse,
        routing::get,
        Json, Router,
    };
    use chrono::Utc;
    use serde_json::json;
    use std::{collections::HashMap, sync::Arc};
    use tokio::net::TcpListener;
    use value_object::TenantId;

    const PLATFORM_ID: &str = "tn_01j702qf86pc2j35s0kv0gv3gy";
    const USER_ID: &str = "us_01hs2yepy5hw4rz8pdq2wywnwt";

    #[derive(Clone, Copy)]
    enum ByUserMode {
        BareArray,
        Fail,
    }

    #[derive(Clone)]
    struct FakeAuthState {
        mode: ByUserMode,
    }

    #[derive(Default)]
    struct TestQuery;

    #[Object]
    impl TestQuery {
        async fn me(&self) -> User {
            User {
                id: USER_ID.to_string(),
                email: Some("user@example.com".to_string()),
                name: Some("Test User".to_string()),
                username: Some("test-user".to_string()),
                email_verified: None,
                image: None,
                role: tachyon_sdk::auth::DefaultRole::Owner,
                tenant_id_list: vec![PLATFORM_ID.to_string()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }
    }

    #[tokio::test]
    async fn organizations_accepts_platform_operator_array_contract() {
        let response =
            execute_me_organizations(ByUserMode::BareArray).await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let organizations = data["me"]["organizations"].as_array().unwrap();
        assert_eq!(organizations.len(), 1);
        assert_eq!(organizations[0]["id"], PLATFORM_ID);
        assert_eq!(organizations[0]["platformTenantId"], PLATFORM_ID);
    }

    #[tokio::test]
    async fn organizations_falls_back_when_platform_lookup_fails() {
        let response = execute_me_organizations(ByUserMode::Fail).await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let organizations = data["me"]["organizations"].as_array().unwrap();
        assert_eq!(organizations.len(), 1);
        assert_eq!(organizations[0]["id"], PLATFORM_ID);
        assert_eq!(organizations[0]["platformTenantId"], PLATFORM_ID);
    }

    async fn execute_me_organizations(
        mode: ByUserMode,
    ) -> async_graphql::Response {
        let auth_url = spawn_fake_auth_server(mode).await;
        let platform_id = PLATFORM_ID.parse::<TenantId>().unwrap();
        let sdk =
            Arc::new(SdkAuthApp::new(auth_url, &platform_id, "token"));

        let schema =
            Schema::build(TestQuery, EmptyMutation, EmptySubscription)
                .data(sdk)
                .data(tachyon_sdk::auth::Executor::None)
                .data(tachyon_sdk::auth::MultiTenancy::new(
                    Some(platform_id.clone()),
                    Some(platform_id),
                ))
                .finish();

        schema
            .execute(Request::new(
                "query {
                    me {
                        organizations {
                            id
                            operatorName
                            platformTenantId
                        }
                    }
                }",
            ))
            .await
    }

    async fn spawn_fake_auth_server(mode: ByUserMode) -> String {
        let app = Router::new()
            .route(
                "/v1/auth/operators/by-user",
                get(fake_find_operators_by_user),
            )
            .route("/v1/auth/operators/:id", get(fake_get_operator))
            .with_state(FakeAuthState { mode });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    async fn fake_find_operators_by_user(
        State(state): State<FakeAuthState>,
        Query(query): Query<HashMap<String, String>>,
    ) -> impl IntoResponse {
        if matches!(state.mode, ByUserMode::Fail) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    json!({ "message": "platform operator lookup failed" }),
                ),
            )
                .into_response();
        }

        let operator_id = query
            .get("platform_id")
            .cloned()
            .unwrap_or_else(|| PLATFORM_ID.to_string());

        Json(json!([operator_json(&operator_id)])).into_response()
    }

    async fn fake_get_operator(
        Path(id): Path<String>,
    ) -> impl IntoResponse {
        Json(operator_json(&id))
    }

    fn operator_json(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "name": "Test Operator",
            "operatorName": "test-operator",
            "platformId": PLATFORM_ID
        })
    }
}
