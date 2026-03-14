use super::{ViewDataInputData, ViewDataInputPort};
use database_manager::{
    domain::{Data, Property},
    usecase::FindAllPropertiesInputData,
};
use std::sync::Arc;
use tachyon_sdk::auth::{AuthApp, CheckPolicyInput};

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

        if *repo.is_public() {
            return Ok((data, properties));
        }

        if input.executor.is_none() && repo.is_private() {
            return Err(errors::permission_denied!("Access denied"));
        }

        if !input.executor.is_none() && repo.is_private() {
            self.auth
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    action: "library:ViewData",
                })
                .await?;
            // repo.can_view(&input.executor.get_id().parse()?)?;
        }

        // TODO: add English comment
        Ok((data, properties))
    }
}
