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
        input
            .auth
            .check_policy(&tachyon_sdk::auth::CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:CreateRepo",
            })
            .await?;

        // TODO: add English comment
        let username = input.org_username.parse::<Identifier>()?;
        let org =
            self.get_organization_by_username.execute(&username).await?;
        let org = match org {
            Some(org) => org,
            None => {
                let tenant_id = input.multi_tenancy.get_operator_id()?;
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

        // Attach Owner policy to IAM (with repository scope)
        let resource_trn = format!("trn:library:repo:{}", repo.id());
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
                    tenant_id: org.id(),
                    resource_scope: &resource_trn,
                },
            )
            .await?;

        self.repo_repository.save(&repo).await?;
        Ok(repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
