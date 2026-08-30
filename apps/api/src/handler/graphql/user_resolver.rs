use super::model::{Operator, User};
use crate::domain::{OrganizationRepository, LIBRARY_TENANT};
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
        let organization_repo =
            ctx.data::<Arc<dyn OrganizationRepository>>()?;
        let mut operators = Vec::new();
        let mut seen = HashSet::new();
        load_operators_from_tenants(
            sdk,
            organization_repo.as_ref(),
            &self.tenant_id_list,
            &mut seen,
            &mut operators,
        )
        .await;

        Ok(operators.into_iter().map(Into::into).collect())
    }
}

/// Whether a tenant the caller belongs to is one of Library's own
/// organizations.
///
/// An organization created in Library gets an operator on the Library
/// platform, but one adopted from tachyon by `seedLibraryTenant` keeps
/// the platform it already had -- so the platform alone cannot tell a
/// Library organization from any other tenant the caller happens to be
/// a member of. What both have is a row in Library's own organizations
/// table, which is what this asks for.
async fn is_library_organization(
    organization_repo: &dyn OrganizationRepository,
    operator: &tachyon_sdk::auth::Operator,
) -> bool {
    if operator.platform_id == *LIBRARY_TENANT {
        return true;
    }

    match organization_repo.get_by_id(&operator.id).await {
        Ok(organization) => organization.is_some(),
        Err(err) => {
            tracing::warn!(
                tenant_id = %operator.id,
                error = ?err,
                "Failed to look up the Library organization for a tenant"
            );
            false
        }
    }
}

async fn load_operators_from_tenants(
    sdk: &SdkAuthApp,
    organization_repo: &dyn OrganizationRepository,
    tenant_id_list: &[String],
    seen: &mut HashSet<String>,
    operators: &mut Vec<tachyon_sdk::auth::Operator>,
) {
    // One upstream call per tenant, so issuing them one after another
    // made this field as slow as the caller has tenants. They are
    // independent lookups; awaiting them together keeps the result
    // order that the sequential version produced. The organization
    // lookup rides along inside the same future for the same reason.
    let pending: Vec<_> = tenant_id_list
        .iter()
        .filter(|tenant_id| seen.insert((*tenant_id).clone()))
        .map(|tenant_id| async move {
            let operator = match sdk.get_operator(tenant_id).await {
                Ok(Some(op)) => {
                    match crate::sdk_auth::operator_from_resp(&op) {
                        Ok(operator) => operator,
                        Err(err) => {
                            tracing::warn!(
                                tenant_id = %tenant_id,
                                error = ?err,
                                "Failed to parse operator"
                            );
                            return None;
                        }
                    }
                }
                Ok(None) => {
                    tracing::warn!(
                        tenant_id = %tenant_id,
                        "Operator not found"
                    );
                    return None;
                }
                Err(err) => {
                    tracing::warn!(
                        tenant_id = %tenant_id,
                        error = ?err,
                        "Failed to load operator"
                    );
                    return None;
                }
            };

            is_library_organization(organization_repo, &operator)
                .await
                .then_some(operator)
        })
        .collect();

    operators.extend(join_all(pending).await.into_iter().flatten());
}

#[cfg(test)]
mod tests {
    use super::User;
    use crate::domain::{Organization, OrganizationRepository};
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
    use value_object::{FromStr, Identifier, TenantId, Text};

    const PLATFORM_ID: &str = "tn_01j702qf86pc2j35s0kv0gv3gy";
    const LIBRARY_PLATFORM_ID: &str = "tn_01j702qf86pc2j35s0kv0gv3gz";
    const USER_ID: &str = "us_01hs2yepy5hw4rz8pdq2wywnwt";
    /// A tenant that was created in tachyon and later adopted by
    /// Library, so its operator still lives on the tachyon platform.
    const IMPORTED_TENANT_ID: &str = "tn_01kxz0ytmhnab5vh53011cwctj";
    const TACHYON_PLATFORM_ID: &str = "tn_01hjjn348rn3t49zz6hvmfq67p";

    struct TestQuery {
        tenant_id_list: Vec<String>,
    }

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
                tenant_id_list: self.tenant_id_list.clone(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }
    }

    /// Stands in for Library's organizations table, holding only the
    /// tenants that have been created or imported as organizations.
    #[derive(Debug, Default)]
    struct SeededOrganizations {
        tenant_ids: Vec<TenantId>,
    }

    impl SeededOrganizations {
        fn with(tenant_ids: &[&str]) -> Self {
            Self {
                tenant_ids: tenant_ids
                    .iter()
                    .map(|id| id.parse::<TenantId>().unwrap())
                    .collect(),
            }
        }

        fn organization(tenant_id: &TenantId) -> Organization {
            Organization::new(
                tenant_id,
                &Text::from_str("Seeded Organization").unwrap(),
                &Identifier::from_str("seeded-organization").unwrap(),
                None,
                None,
            )
        }
    }

    #[async_trait::async_trait]
    impl OrganizationRepository for SeededOrganizations {
        async fn insert(
            &self,
            _organization: &Organization,
        ) -> errors::Result<()> {
            unimplemented!("organizations are not written by this field")
        }

        async fn update(
            &self,
            _organization: &Organization,
        ) -> errors::Result<()> {
            unimplemented!("organizations are not written by this field")
        }

        async fn get_by_id(
            &self,
            org_id: &TenantId,
        ) -> errors::Result<Option<Organization>> {
            Ok(self
                .tenant_ids
                .contains(org_id)
                .then(|| Self::organization(org_id)))
        }

        async fn get_by_username(
            &self,
            _username: &Identifier,
        ) -> errors::Result<Option<Organization>> {
            unimplemented!("this field looks organizations up by id")
        }

        async fn find_all(&self) -> errors::Result<Vec<Organization>> {
            unimplemented!("this field looks organizations up by id")
        }

        async fn delete(&self, _org_id: &TenantId) -> errors::Result<()> {
            unimplemented!("organizations are not written by this field")
        }
    }

    #[tokio::test]
    async fn organizations_loads_user_tenants_without_platform_lookup() {
        let response = execute_me_organizations(
            Some(LIBRARY_PLATFORM_ID),
            &[PLATFORM_ID],
            SeededOrganizations::default(),
        )
        .await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let organizations = data["me"]["organizations"].as_array().unwrap();
        assert_eq!(organizations.len(), 1);
        assert_eq!(organizations[0]["id"], PLATFORM_ID);
        assert_eq!(organizations[0]["platformTenantId"], PLATFORM_ID);
    }

    #[tokio::test]
    async fn organizations_loads_user_tenants_without_platform_id() {
        let response = execute_me_organizations(
            None,
            &[PLATFORM_ID],
            SeededOrganizations::default(),
        )
        .await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let organizations = data["me"]["organizations"].as_array().unwrap();
        assert_eq!(organizations.len(), 1);
        assert_eq!(organizations[0]["id"], PLATFORM_ID);
        assert_eq!(organizations[0]["platformTenantId"], PLATFORM_ID);
    }

    /// An imported tenant keeps the platform it was created under, so
    /// the organizations table is the only thing that marks it as one
    /// of Library's own. Matching on the platform alone hid it.
    #[tokio::test]
    async fn organizations_include_an_imported_tenant_from_another_platform(
    ) {
        let response = execute_me_organizations(
            Some(LIBRARY_PLATFORM_ID),
            &[IMPORTED_TENANT_ID],
            SeededOrganizations::with(&[IMPORTED_TENANT_ID]),
        )
        .await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let organizations = data["me"]["organizations"].as_array().unwrap();
        assert_eq!(organizations.len(), 1);
        assert_eq!(organizations[0]["id"], IMPORTED_TENANT_ID);
        assert_eq!(
            organizations[0]["platformTenantId"],
            TACHYON_PLATFORM_ID
        );
    }

    /// The caller belongs to plenty of tachyon tenants that were never
    /// adopted by Library. Those are not organizations here.
    #[tokio::test]
    async fn organizations_omit_a_tenant_that_was_never_imported() {
        let response = execute_me_organizations(
            Some(LIBRARY_PLATFORM_ID),
            &[IMPORTED_TENANT_ID],
            SeededOrganizations::default(),
        )
        .await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let organizations = data["me"]["organizations"].as_array().unwrap();
        assert!(organizations.is_empty(), "{organizations:?}");
    }

    async fn execute_me_organizations(
        request_platform_id: Option<&str>,
        tenant_id_list: &[&str],
        organizations: SeededOrganizations,
    ) -> async_graphql::Response {
        let auth_url = spawn_fake_auth_server().await;
        let sdk_platform_id =
            LIBRARY_PLATFORM_ID.parse::<TenantId>().unwrap();
        let sdk =
            Arc::new(SdkAuthApp::new(auth_url, &sdk_platform_id, "token"));
        let request_platform_id =
            request_platform_id.map(|id| id.parse::<TenantId>().unwrap());
        let organization_repo: Arc<dyn OrganizationRepository> =
            Arc::new(organizations);

        let schema = Schema::build(
            TestQuery {
                tenant_id_list: tenant_id_list
                    .iter()
                    .map(|id| (*id).to_string())
                    .collect(),
            },
            EmptyMutation,
            EmptySubscription,
        )
        .data(sdk)
        .data(organization_repo)
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
        let platform_id = if id == IMPORTED_TENANT_ID {
            TACHYON_PLATFORM_ID
        } else {
            PLATFORM_ID
        };

        json!({
            "id": id,
            "name": "Test Operator",
            "operatorName": "test-operator",
            "platformId": platform_id
        })
    }
}
