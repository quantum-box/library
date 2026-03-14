use std::fmt::Debug;
use std::sync::Arc;

use database_manager::{
    domain::{Data, Property},
    usecase::FindAllPropertiesInputData,
};
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, ExecutorAction, MultiTenancyAction,
};
use value_object::OffsetPaginator;

#[async_trait::async_trait]
pub trait ViewDataListInputPort: Debug + Send + Sync {
    async fn execute(
        &self,
        input: &ViewDataListInputData,
    ) -> errors::Result<(Vec<Data>, Vec<Property>, OffsetPaginator)>;
}

#[derive(Debug, Clone)]
pub struct ViewDataListInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,
    pub page_size: Option<u32>,
    pub page: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ViewDataList {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    database: Arc<database_manager::App>,
    auth_app: Arc<dyn AuthApp>,
}

impl ViewDataList {
    pub fn new(
        database: Arc<database_manager::App>,
        get_org_by_username: Arc<
            dyn crate::usecase::GetOrganizationByUsernameQuery,
        >,
        get_repo_by_username: Arc<
            dyn crate::usecase::GetRepoByUsernameQuery,
        >,
        auth_app: Arc<dyn AuthApp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database,
            get_org_by_username,
            get_repo_by_username,
            auth_app,
        })
    }
}

#[async_trait::async_trait]
impl ViewDataListInputPort for ViewDataList {
    #[tracing::instrument(name = "ViewDataList::execute", skip(self))]
    async fn execute(
        &self,
        input: &ViewDataListInputData,
    ) -> errors::Result<(Vec<Data>, Vec<Property>, OffsetPaginator)> {
        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?
            .ok_or(errors::not_found!(
                "organization not found in view data list"
            ))?;

        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?
            .ok_or(errors::not_found!(
                "repo not found in view data list"
            ))?;

        let properties = self
            .database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: org.id().clone(),
                database_id: repo.databases().first().unwrap().clone(),
            })
            .await?;
        let (data_list, paginator) = self
            .database
            .search_data()
            .execute(&database_manager::SearchDataInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: Some(
                    repo.databases().first().unwrap().clone(),
                ),
                page: input.page,
                page_size: input.page_size,
                query: "",
            })
            .await?;

        if *repo.is_public() {
            return Ok((data_list, properties, paginator));
        }

        if input.executor.is_none() && repo.is_private() {
            return Err(errors::Error::permission_denied("Access denied"));
        }

        if !input.executor.is_none() && repo.is_private() {
            self.auth_app
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    action: "library:ViewPrivateRepo",
                })
                .await?;
        }

        Ok((data_list, properties, paginator))
    }
}
