use super::model::{Operator, User};
use crate::sdk_auth::SdkAuthApp;
use async_graphql::{Context, Result};
use futures_util::future::join_all;
use std::collections::HashSet;
use std::sync::Arc;

#[async_graphql::ComplexObject]
impl User {
    /// Resolve operator organizations accessible to this user
    #[tracing::instrument(name = "organizations_by_user", skip_all)]
    async fn organizations(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<Operator>> {
        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;
        let mut operators = Vec::new();
        let mut seen = HashSet::new();
        load_operators_from_tenants(
            sdk,
            &self.tenant_id_list,
            &mut seen,
            &mut operators,
        )
        .await;

        Ok(operators.into_iter().map(Into::into).collect())
    }
}

async fn load_operators_from_tenants(
    sdk: &SdkAuthApp,
    tenant_id_list: &[String],
    seen: &mut HashSet<String>,
    operators: &mut Vec<tachyon_sdk::auth::Operator>,
) {
    // One upstream call per tenant, so issuing them one after another
    // made this field as slow as the caller has tenants. They are
    // independent lookups; awaiting them together keeps the result
    // order that the sequential version produced.
    let pending: Vec<_> = tenant_id_list
        .iter()
        .filter(|tenant_id| seen.insert((*tenant_id).clone()))
        .map(|tenant_id| async move {
            (tenant_id, sdk.get_operator(tenant_id).await)
        })
        .collect();

    for (tenant_id, result) in join_all(pending).await {
        match result {
            Ok(Some(op)) => {
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

#[cfg(test)]
mod tests {
    use super::User;
    use crate::sdk_auth::SdkAuthApp;
    use async_graphql::{
        EmptyMutation, EmptySubscription, Object, Request, Schema,
    };
    use axum::{
        extract::Path, response::IntoResponse, routing::get, Json, Router,
    };
    use chrono::Utc;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use value_object::TenantId;

    const PLATFORM_ID: &str = "tn_01j702qf86pc2j35s0kv0gv3gy";
    const LIBRARY_PLATFORM_ID: &str = "tn_01j702qf86pc2j35s0kv0gv3gz";
    const USER_ID: &str = "us_01hs2yepy5hw4rz8pdq2wywnwt";

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
    async fn organizations_loads_user_tenants_without_platform_lookup() {
        let response =
            execute_me_organizations(Some(LIBRARY_PLATFORM_ID)).await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let organizations = data["me"]["organizations"].as_array().unwrap();
        assert_eq!(organizations.len(), 1);
        assert_eq!(organizations[0]["id"], PLATFORM_ID);
        assert_eq!(organizations[0]["platformTenantId"], PLATFORM_ID);
    }

    #[tokio::test]
    async fn organizations_loads_user_tenants_without_platform_id() {
        let response = execute_me_organizations(None).await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let organizations = data["me"]["organizations"].as_array().unwrap();
        assert_eq!(organizations.len(), 1);
        assert_eq!(organizations[0]["id"], PLATFORM_ID);
        assert_eq!(organizations[0]["platformTenantId"], PLATFORM_ID);
    }

    async fn execute_me_organizations(
        request_platform_id: Option<&str>,
    ) -> async_graphql::Response {
        let auth_url = spawn_fake_auth_server().await;
        let sdk_platform_id =
            LIBRARY_PLATFORM_ID.parse::<TenantId>().unwrap();
        let sdk =
            Arc::new(SdkAuthApp::new(auth_url, &sdk_platform_id, "token"));
        let request_platform_id =
            request_platform_id.map(|id| id.parse::<TenantId>().unwrap());

        let schema =
            Schema::build(TestQuery, EmptyMutation, EmptySubscription)
                .data(sdk)
                .data(tachyon_sdk::auth::Executor::None)
                .data(tachyon_sdk::auth::MultiTenancy::new(
                    request_platform_id.clone(),
                    request_platform_id,
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

    async fn spawn_fake_auth_server() -> String {
        let app = Router::new()
            .route("/v1/auth/operators/:id", get(fake_get_operator));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
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
