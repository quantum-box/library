use std::{fmt::Debug, sync::Arc};

use crate::domain::{Repo, RepoId, RepoRepository};

use super::{
    CreateRepoInputData, CreateRepoInputPort,
    GetOrganizationByUsernameQuery,
};
use database_manager::domain::PropertyType;
use database_manager::usecase::CreateDatabaseInputData;
use database_manager::{domain::DatabaseId, AddPropertyInputData};
use database_manager::{AddDataInputData, PropertyDataInputData};
use derive_new::new;
use tachyon_sdk::auth::AuthApp;
use value_object::Ulid;

#[derive(Debug, Clone, new)]
pub struct CreateRepo {
    repo_repository: Arc<dyn RepoRepository>,
    get_organization_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    database_client: Arc<database_manager::App>,
    auth: Arc<dyn AuthApp>,
}

#[async_trait::async_trait]
impl CreateRepoInputPort for CreateRepo {
    #[tracing::instrument(name = "CreateRepo::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: CreateRepoInputData<'a>,
    ) -> errors::Result<Repo> {
        // TODO: add English comment
        self.auth
            .check_policy(&tachyon_sdk::auth::CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:CreateRepo",
            })
            .await?;

        // TODO: add English comment
        let org = self
            .get_organization_by_username
            .execute(&input.org_username.parse()?)
            .await?
            .ok_or(errors::not_found!(
                "organization not found in create repo"
            ))?;

        let mut repo = Repo::create(
            &RepoId::default(),
            org.id(),
            org.username(),
            &input.repo_name.parse()?,
            &input.repo_username.parse()?,
            &input.user_id.parse()?,
            input.is_public,
            input.description.map(|d| d.parse().unwrap()),
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
        let id_property = self
            .database_client
            .add_property()
            .execute(AddPropertyInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: database.id(),
                name: "id",
                property_type: PropertyType::String,
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
                name: "content",
                property_type: PropertyType::Markdown,
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
                    property_data: vec![
                        PropertyDataInputData {
                            property_id: id_property.id().to_string(),
                            value: Ulid::new().to_string(),
                        },
                        PropertyDataInputData {
                            property_id: content_property.id().to_string(),
                            value: "# data1\n\nwrite here\n".to_string(),
                        },
                    ],
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
                    property_data: vec![
                        PropertyDataInputData {
                            property_id: id_property.id().to_string(),
                            value: Ulid::new().to_string(),
                        },
                        PropertyDataInputData {
                            property_id: content_property.id().to_string(),
                            value: "# data2\n\nwrite here\n".to_string(),
                        },
                    ],
                })
                .await?;
        }

        let repo = repo.add_database(database.id());

        // Attach Owner policy to IAM (with repository scope)
        let resource_trn = format!("trn:library:repo:{}", repo.id());
        self.auth
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
