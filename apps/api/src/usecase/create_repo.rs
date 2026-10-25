use std::{fmt::Debug, sync::Arc};

use crate::domain::{
    Organization, OrganizationRepository, Repo, RepoId, RepoRepository,
    LIBRARY_TENANT,
};
use crate::sdk_auth::{OperatorResp, SdkAuthApp};

use super::{
    CreateRepoInputData, CreateRepoInputPort,
    GetOrganizationByUsernameQuery,
};
use database_manager::domain::{
    PropertyType, PropertyValueCommand, TypeId,
};
use database_manager::usecase::CreateDatabaseInputData;
use database_manager::{domain::DatabaseId, AddPropertyInputData};
use database_manager::{AddDataInputData, PropertyDataInputData};
use derive_new::new;
use value_object::{Identifier, LongText, TenantId};

async fn authorize_creation_in_destination(
    input: &CreateRepoInputData<'_>,
    destination: &TenantId,
) -> errors::Result<super::LibraryOrg> {
    // GraphQL supports an omitted tenant header. Resolve that case from the
    // destination, but reject a conflicting explicit header before any writes.
    if input.multi_tenancy.operator_id().is_some() {
        ensure_creation_tenant(input.multi_tenancy, destination)?;
    }
    let scope = super::LibraryOrg::with_org_and_operator(
        input.org_username.clone(),
        destination.clone(),
    );
    input
        .auth
        .check_policy(&tachyon_sdk::auth::CheckPolicyInput {
            executor: input.executor,
            multi_tenancy: &scope,
            action: "library:CreateRepo",
        })
        .await?;
    Ok(scope)
}

fn ensure_creation_tenant(
    scope: &dyn tachyon_sdk::auth::MultiTenancyAction,
    destination: &TenantId,
) -> errors::Result<()> {
    if scope.get_operator_id()? != *destination {
        return Err(errors::Error::forbidden(
            "Repository destination does not match the authorization tenant",
        ));
    }
    Ok(())
}

/// An organization owner may already manage every repository in the tenant.
/// Reuse that authorization instead of requiring a redundant IAM mutation.
/// In particular, a resource-bound OAuth token can request delegated policy
/// decisions but cannot be forwarded to the upstream policy-write endpoint.
async fn ensure_creator_owner_access(
    input: &CreateRepoInputData<'_>,
    repo: &Repo,
) -> errors::Result<()> {
    ensure_creation_tenant(input.multi_tenancy, repo.organization_id())?;
    let resource_trn = format!("trn:library:repo:{}", repo.id());
    if input.executor.is_user()
        && input.executor.get_user_id()?.as_str() == input.user_id
    {
        match input
            .auth
            .check_policy_for_resource(
                &tachyon_sdk::auth::CheckPolicyForResourceInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    action: "library:ManageRepoPolicy",
                    resource_trn: &resource_trn,
                },
            )
            .await
        {
            Ok(()) => return Ok(()),
            // A user without existing owner access still needs the scoped
            // grant, authorized independently by the upstream IAM service.
            Err(errors::Error::Forbidden { .. }) => {}
            Err(error) => return Err(error),
        }
    }

    input
        .auth
        .attach_user_policy_with_scope(
            &tachyon_sdk::auth::AttachUserPolicyWithScopeInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                user_id: &input.user_id.parse()?,
                policy_id: &tachyon_sdk::auth::PolicyId::new(
                    "pol_01libraryrepoowner",
                ),
                tenant_id: repo.organization_id(),
                resource_scope: &resource_trn,
            },
        )
        .await
}

#[derive(Debug, Clone, new)]
pub struct CreateRepo {
    repo_repository: Arc<dyn RepoRepository>,
    get_organization_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    database_client: Arc<database_manager::App>,
    organization_repository: Arc<dyn OrganizationRepository>,
    sdk: Arc<SdkAuthApp>,
}

/// Older Library tenants can exist in auth without a local organization row.
/// Restore only the already-authorized request's native Library tenant; never
/// implicitly import a tenant from another platform or grant any policies.
async fn restore_library_organization(
    repository: &dyn OrganizationRepository,
    operator: &OperatorResp,
    tenant_id: &TenantId,
    username: &Identifier,
) -> errors::Result<Organization> {
    if operator.id != tenant_id.as_str()
        || operator.platform_id != LIBRARY_TENANT.as_str()
        || operator.operator_name != username.value().as_str()
    {
        return Err(errors::Error::not_found(
            "organization not found in create repo",
        ));
    }
    let organization = Organization::new(
        tenant_id,
        &operator.name.parse()?,
        username,
        None,
        None,
    );
    repository.insert(&organization).await?;
    Ok(organization)
}

fn parse_optional_description(
    description: Option<&str>,
) -> errors::Result<Option<LongText>> {
    description
        .map(str::trim)
        .filter(|description| !description.is_empty())
        .map(str::parse::<LongText>)
        .transpose()
        .map_err(|err| errors::Error::invalid(err.to_string()))
}

#[async_trait::async_trait]
impl CreateRepoInputPort for CreateRepo {
    #[tracing::instrument(name = "CreateRepo::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: CreateRepoInputData<'a>,
    ) -> errors::Result<Repo> {
        // TODO: add English comment
        let username = input.org_username.parse::<Identifier>()?;
        let org =
            self.get_organization_by_username.execute(&username).await?;
        let org = match org {
            Some(org) => org,
            None => {
                let tenant_id = input.multi_tenancy.get_operator_id()?;
                // Restoring a local organization row is a write too.
                authorize_creation_in_destination(&input, &tenant_id)
                    .await?;
                let operator = self
                    .sdk
                    .get_operator(tenant_id.as_str())
                    .await?
                    .ok_or(errors::not_found!(
                        "organization not found in create repo"
                    ))?;
                restore_library_organization(
                    self.organization_repository.as_ref(),
                    &operator,
                    &tenant_id,
                    &username,
                )
                .await?
            }
        };
        let scope =
            authorize_creation_in_destination(&input, org.id()).await?;
        let input = CreateRepoInputData {
            multi_tenancy: &scope,
            ..input
        };
        let description =
            parse_optional_description(input.description.as_deref())?;

        let mut repo = Repo::create(
            &RepoId::default(),
            org.id(),
            org.username(),
            &input.repo_name.parse()?,
            &input.repo_username.parse()?,
            &input.user_id.parse()?,
            input.is_public,
            description,
            vec![],
            vec![], // tags
        );
        let database = self
            .database_client
            .create_database()
            .execute(CreateDatabaseInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                name: &input.repo_name,
                database_id: input
                    .database_id
                    .as_ref()
                    .map(|id| id.parse::<DatabaseId>())
                    .transpose()?
                    .as_ref(),
            })
            .await?;
        self.database_client
            .add_property()
            .execute(AddPropertyInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: database.id(),
                display_name: None,
                name: "id",
                property_type: PropertyType::Id(TypeId::new(true)),
            })
            .await?;
        let content_property = self
            .database_client
            .add_property()
            .execute(AddPropertyInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: database.id(),
                display_name: None,
                name: "content",
                property_type: PropertyType::RichText,
            })
            .await?;

        // Create sample data only if not skipped
        if !input.skip_sample_data {
            self.database_client
                .add_data_usecase()
                .execute(AddDataInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    tenant_id: org.id(),
                    database_id: database.id(),
                    name: "data1",
                    property_data: vec![PropertyDataInputData {
                        property_id: content_property.id().clone(),
                        value: PropertyValueCommand::RichText(
                            database_manager::domain::rich_text::from_markdown(
                                "# data1\n\nwrite here\n",
                            ),
                        ),
                    }],
                })
                .await?;
            self.database_client
                .add_data_usecase()
                .execute(AddDataInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    tenant_id: org.id(),
                    database_id: database.id(),
                    name: "data2",
                    property_data: vec![PropertyDataInputData {
                        property_id: content_property.id().clone(),
                        value: PropertyValueCommand::RichText(
                            database_manager::domain::rich_text::from_markdown(
                                "# data2\n\nwrite here\n",
                            ),
                        ),
                    }],
                })
                .await?;
        }

        let repo = repo.add_database(database.id());

        ensure_creator_owner_access(&input, &repo).await?;

        self.repo_repository.save(&repo).await?;
        Ok(repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use tachyon_sdk::auth::{ExecutorAction, MultiTenancy};

    #[derive(Debug)]
    struct Caller;

    impl ExecutorAction for Caller {
        fn get_id(&self) -> &str {
            "us_01testcaller"
        }
        fn has_tenant_id(&self, _: &TenantId) -> bool {
            true
        }
        fn is_system_user(&self) -> bool {
            false
        }
        fn is_user(&self) -> bool {
            true
        }
        fn is_service_account(&self) -> bool {
            false
        }
        fn is_none(&self) -> bool {
            false
        }
    }

    /// Exercise the real SDK adapter, including credential selection and
    /// the HTTP boundary, without a database or production credentials.
    async fn owner_access_case(
        oauth: bool,
        owner_status: axum::http::StatusCode,
        owner_response: serde_json::Value,
        grant_status: axum::http::StatusCode,
        grant_to_caller: bool,
    ) -> (errors::Result<()>, Vec<String>) {
        owner_access_case_with_scope(
            oauth,
            owner_status,
            owner_response,
            grant_status,
            grant_to_caller,
            None,
        )
        .await
    }

    async fn owner_access_case_with_scope(
        oauth: bool,
        owner_status: axum::http::StatusCode,
        owner_response: serde_json::Value,
        grant_status: axum::http::StatusCode,
        grant_to_caller: bool,
        authorization_tenant: Option<TenantId>,
    ) -> (errors::Result<()>, Vec<String>) {
        let tenant: TenantId =
            "tn_01kshr165k3wwr8xyk0e6vy36c".parse().unwrap();
        let repo_id: RepoId =
            "rp_01m5sjxk92qd7yrzxk798b1jnc".parse().unwrap();
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = requests.clone();
        let app = axum::Router::new().fallback(axum::routing::any(
            move |uri: axum::http::Uri, headers: axum::http::HeaderMap,
                  axum::Json(body): axum::Json<serde_json::Value>| {
                let captured = captured.clone();
                let owner_response = owner_response.clone();
                async move {
                    captured.lock().unwrap().push(uri.path().to_string());
                    if uri.path().contains("check-for-resource") {
                        assert_eq!(body["action"], "library:ManageRepoPolicy");
                        assert_eq!(body["resourceTrn"], "trn:library:repo:rp_01m5sjxk92qd7yrzxk798b1jnc");
                        if oauth {
                            assert_eq!(uri.path(), "/v1/auth/policies/check-for-resource-for-user");
                            assert_eq!(headers["authorization"], "Bearer service-token");
                            assert_eq!(headers["x-operator-id"], LIBRARY_TENANT.as_str());
                            assert_eq!(body["userId"], "us_01testcaller");
                            assert_eq!(body["tenantId"], "tn_01kshr165k3wwr8xyk0e6vy36c");
                        } else {
                            assert_eq!(headers["authorization"], "Bearer caller-token");
                            assert_eq!(headers["x-operator-id"], "tn_01kshr165k3wwr8xyk0e6vy36c");
                        }
                        (owner_status, axum::Json(owner_response))
                    } else {
                        assert_eq!(uri.path(), "/v1/auth/user-policies/attach-with-scope");
                        assert_eq!(body["tenantId"], "tn_01kshr165k3wwr8xyk0e6vy36c");
                        assert_eq!(body["policyId"], "pol_01libraryrepoowner");
                        assert_eq!(body["resourceScope"], "trn:library:repo:rp_01m5sjxk92qd7yrzxk798b1jnc");
                        if oauth {
                            assert!(headers.get("authorization").is_none());
                        } else {
                            assert_eq!(headers["authorization"], "Bearer caller-token");
                        }
                        (grant_status, axum::Json(serde_json::json!({})))
                    }
                }
            },
        ));
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap()
        });
        let token = if oauth {
            format!("e30.{}.sig", URL_SAFE_NO_PAD.encode(serde_json::to_vec(&serde_json::json!({
                "client_id": "mcp-client", "aud": "https://library-api.txcloud.app/mcp"
            })).unwrap()))
        } else {
            "caller-token".to_string()
        };
        let sdk = crate::sdk_auth::SdkAuthApp::new(
            format!("http://{address}"),
            &LIBRARY_TENANT,
            "service-token",
        )
        .with_caller_token(&token);
        let scope = MultiTenancy::new(
            None,
            Some(authorization_tenant.unwrap_or_else(|| tenant.clone())),
        );
        let input = CreateRepoInputData {
            auth: Arc::new(sdk),
            executor: &Caller,
            multi_tenancy: &scope,
            org_username: "bernard-square".into(),
            repo_name: "Recipes".into(),
            repo_username: "recipes".into(),
            user_id: if grant_to_caller {
                "us_01testcaller"
            } else {
                "us_01otheruser"
            }
            .into(),
            is_public: false,
            description: None,
            database_id: None,
            skip_sample_data: true,
        };
        let repo = Repo::create(
            &repo_id,
            &tenant,
            &"bernard-square".parse().unwrap(),
            &"Recipes".parse().unwrap(),
            &"recipes".parse().unwrap(),
            &input.user_id.parse().unwrap(),
            false,
            None,
            vec![],
            vec![],
        );
        let result = crate::sdk_auth::caller_token_scope(
            Some(token),
            ensure_creator_owner_access(&input, &repo),
        )
        .await;
        server.abort();
        let seen = requests.lock().unwrap().clone();
        (result, seen)
    }

    #[tokio::test]
    async fn mismatched_authorization_tenant_is_rejected_before_any_grant()
    {
        for oauth in [false, true] {
            let (result, seen) = owner_access_case_with_scope(
                oauth,
                axum::http::StatusCode::OK,
                serde_json::json!({"allowed": true}),
                axum::http::StatusCode::OK,
                true,
                Some(LIBRARY_TENANT.clone()),
            )
            .await;
            assert!(matches!(result, Err(errors::Error::Forbidden { .. })));
            assert!(seen.is_empty());
        }
    }

    #[tokio::test]
    async fn existing_owner_does_not_require_a_redundant_grant() {
        for oauth in [false, true] {
            let (result, seen) = owner_access_case(
                oauth,
                axum::http::StatusCode::OK,
                serde_json::json!({"allowed": true}),
                axum::http::StatusCode::UNAUTHORIZED,
                true,
            )
            .await;
            result.unwrap();
            assert_eq!(seen.len(), 1);
        }
    }

    #[tokio::test]
    async fn missing_owner_access_requires_an_authorized_scoped_grant() {
        let (result, seen) = owner_access_case(
            false,
            axum::http::StatusCode::OK,
            serde_json::json!({"allowed": false}),
            axum::http::StatusCode::OK,
            true,
        )
        .await;
        result.unwrap();
        assert_eq!(seen.len(), 2);
        let (result, seen) = owner_access_case(
            false,
            axum::http::StatusCode::OK,
            serde_json::json!({"allowed": false}),
            axum::http::StatusCode::FORBIDDEN,
            true,
        )
        .await;
        assert!(matches!(result, Err(errors::Error::Forbidden { .. })));
        assert_eq!(seen.len(), 2);
    }

    #[tokio::test]
    async fn oauth_caller_without_owner_access_does_not_gain_it() {
        let (result, seen) = owner_access_case(
            true,
            axum::http::StatusCode::OK,
            serde_json::json!({"allowed": false}),
            axum::http::StatusCode::UNAUTHORIZED,
            true,
        )
        .await;
        assert!(matches!(result, Err(errors::Error::Unauthorized { .. })));
        assert_eq!(seen.len(), 2);
    }

    #[tokio::test]
    async fn owner_check_failures_do_not_trigger_a_grant() {
        for (status, body) in [
            (axum::http::StatusCode::UNAUTHORIZED, serde_json::json!({})),
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                serde_json::json!({}),
            ),
            (axum::http::StatusCode::OK, serde_json::json!({})),
        ] {
            let (result, seen) = owner_access_case(
                true,
                status,
                body,
                axum::http::StatusCode::OK,
                true,
            )
            .await;
            assert!(result.is_err());
            assert_eq!(seen.len(), 1);
        }
    }

    #[tokio::test]
    async fn another_users_grant_cannot_reuse_the_callers_owner_access() {
        let (result, seen) = owner_access_case(
            true,
            axum::http::StatusCode::OK,
            serde_json::json!({"allowed": true}),
            axum::http::StatusCode::UNAUTHORIZED,
            false,
        )
        .await;
        assert!(matches!(result, Err(errors::Error::Unauthorized { .. })));
        assert_eq!(seen, ["/v1/auth/user-policies/attach-with-scope"]);
    }

    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct Organizations(Mutex<Option<Organization>>);

    #[async_trait::async_trait]
    impl OrganizationRepository for Organizations {
        async fn insert(&self, org: &Organization) -> errors::Result<()> {
            *self.0.lock().unwrap() = Some(org.clone());
            Ok(())
        }
        async fn update(&self, _: &Organization) -> errors::Result<()> {
            unreachable!()
        }
        async fn get_by_id(
            &self,
            id: &TenantId,
        ) -> errors::Result<Option<Organization>> {
            Ok(self.0.lock().unwrap().clone().filter(|org| org.id() == id))
        }
        async fn get_by_username(
            &self,
            _: &Identifier,
        ) -> errors::Result<Option<Organization>> {
            unreachable!()
        }
        async fn find_all(&self) -> errors::Result<Vec<Organization>> {
            unreachable!()
        }
        async fn delete(&self, _: &TenantId) -> errors::Result<()> {
            unreachable!()
        }
    }

    fn native_operator() -> OperatorResp {
        OperatorResp {
            id: TenantId::default().to_string(),
            name: "Test organization".into(),
            operator_name: "test1".into(),
            platform_id: LIBRARY_TENANT.to_string(),
        }
    }

    #[tokio::test]
    async fn restores_missing_native_organization_for_authorized_tenant() {
        let repository = Organizations::default();
        let operator = native_operator();
        let tenant_id = operator.id.parse().unwrap();
        let org = restore_library_organization(
            &repository,
            &operator,
            &tenant_id,
            &"test1".parse().unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(org.id(), &tenant_id);
        assert_eq!(org.username().value().as_str(), "test1");
        assert_eq!(org.name().as_str(), "Test organization");
        assert!(repository.get_by_id(&tenant_id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn restore_rejects_foreign_platform_or_mismatched_tenant_or_alias(
    ) {
        for mismatch in ["platform", "tenant", "alias"] {
            let repository = Organizations::default();
            let mut operator = native_operator();
            let tenant_id = operator.id.parse().unwrap();
            match mismatch {
                "platform" => {
                    operator.platform_id = TenantId::default().to_string()
                }
                "tenant" => operator.id = TenantId::default().to_string(),
                "alias" => operator.operator_name = "other-org".into(),
                _ => unreachable!(),
            }
            assert!(
                restore_library_organization(
                    &repository,
                    &operator,
                    &tenant_id,
                    &"test1".parse().unwrap(),
                )
                .await
                .is_err(),
                "{mismatch}"
            );
            assert!(repository.0.lock().unwrap().is_none(), "{mismatch}");
        }
    }

    #[test]
    fn empty_description_is_treated_as_absent() {
        assert!(parse_optional_description(Some("")).unwrap().is_none());
        assert!(parse_optional_description(Some("   ")).unwrap().is_none());
        assert!(parse_optional_description(None).unwrap().is_none());
    }

    #[test]
    fn non_empty_description_is_preserved() {
        let description =
            parse_optional_description(Some("  useful context  "))
                .unwrap()
                .unwrap();

        assert_eq!(description.as_str(), "useful context");
    }
}
