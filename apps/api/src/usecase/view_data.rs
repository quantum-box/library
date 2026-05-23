use super::{
    authorize_private_repo_read, ViewDataInputData, ViewDataInputPort,
};
use crate::domain::VisibilityService;
use database_manager::{
    domain::{Data, Property},
    usecase::FindAllPropertiesInputData,
};
use std::sync::Arc;
use tachyon_sdk::auth::AuthApp;

#[derive(Debug, Clone)]
pub struct ViewData {
    auth: Arc<dyn AuthApp>,
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    database: Arc<database_manager::App>,
}

impl ViewData {
    pub fn new(
        auth: Arc<dyn AuthApp>,
        get_org_by_username: Arc<
            dyn crate::usecase::GetOrganizationByUsernameQuery,
        >,
        get_repo_by_username: Arc<
            dyn crate::usecase::GetRepoByUsernameQuery,
        >,
        database: Arc<database_manager::App>,
    ) -> Arc<Self> {
        Arc::new(Self {
            auth,
            get_org_by_username,
            get_repo_by_username,
            database,
        })
    }
}

#[async_trait::async_trait]
impl ViewDataInputPort for ViewData {
    #[tracing::instrument(name = "ViewData::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: &ViewDataInputData<'a>,
    ) -> errors::Result<(Data, Vec<Property>)> {
        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await?
            .ok_or(errors::not_found!(
                "organization not found in view data"
            ))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await?
            .ok_or(errors::not_found!("repo not found in view data"))?;

        let need_policy_check =
            VisibilityService::new().check_access(&repo, input.executor)?;
        if need_policy_check {
            authorize_private_repo_read(
                self.auth.as_ref(),
                input.executor,
                input.multi_tenancy,
                repo.id().as_ref(),
            )
            .await?;
        }

        let properties = self
            .database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: org.id().clone(),
                database_id: repo.databases().first().unwrap().clone(),
            })
            .await?;
        let data = self
            .database
            .get_data_usecase()
            .execute(&database_manager::GetDataInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: &repo.databases().first().unwrap().clone(),
                data_id: &input.data_id.parse()?,
            })
            .await?;

        Ok((data, properties))
    }
}
