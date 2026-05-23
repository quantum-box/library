use crate::usecase::{
    authorize_private_repo_read, GetPropertiesInputData,
    GetPropertiesInputPort,
};
use database_manager::domain;
use std::sync::Arc;
use tachyon_sdk::auth::AuthApp;

#[derive(Debug, Clone)]
pub struct GetProperties {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    database: Arc<database_manager::App>,
    auth: Arc<dyn AuthApp>,
}

impl GetProperties {
    pub fn new(
        get_org_by_username: Arc<
            dyn crate::usecase::GetOrganizationByUsernameQuery,
        >,
        get_repo_by_username: Arc<
            dyn crate::usecase::GetRepoByUsernameQuery,
        >,
        database: Arc<database_manager::App>,
        auth: Arc<dyn AuthApp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            get_org_by_username,
            get_repo_by_username,
            database,
            auth,
        })
    }
}

#[async_trait::async_trait]
impl GetPropertiesInputPort for GetProperties {
    #[tracing::instrument(name = "GetProperties::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: GetPropertiesInputData<'a>,
    ) -> errors::Result<Vec<domain::Property>> {
        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await?
            .ok_or(errors::not_found!(
                "organization not found in get properties"
            ))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await?
            .ok_or(errors::not_found!(
                "repo not found in get properties"
            ))?;

        if input.executor.is_none() && repo.is_private() {
            return Err(errors::Error::permission_denied("Access denied"));
        }

        if !input.executor.is_none() && repo.is_private() {
            authorize_private_repo_read(
                self.auth.as_ref(),
                input.executor,
                input.multi_tenancy,
                repo.id().as_ref(),
            )
            .await?;
        }

        let (_, properties) = self
            .database
            .get_database_definition_usecase()
            .execute(
                database_manager::usecase::GetDatabaseDefinitionInputData {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,

                    tenant_id: org.id(),
                    database_id: &repo
                        .databases()
                        .first()
                        .unwrap()
                        .clone()
                        .parse()?,
                },
            )
            .await?;

        Ok(properties)
    }
}
