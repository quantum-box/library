#![allow(dead_code)]

use super::{SearchDataInputData, SearchDataInputPort};
use database_manager::{
    domain::{self, DataCollection},
    usecase::FindAllPropertiesInputData,
};
use std::{fmt::Debug, sync::Arc};
use value_object::OffsetPaginator;

#[async_trait::async_trait]
pub trait SearchDataQuery: Debug + Send + Sync {
    async fn search_all(
        &self,
        name: &str,
    ) -> errors::Result<DataCollection>;
    async fn search_in_repo(
        &self,
        name: &str,
        repo_id: &str,
    ) -> errors::Result<DataCollection>;
}

#[derive(Debug, Clone)]
pub struct SearchData {
    database: Arc<database_manager::App>,
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
}

impl SearchData {
    pub fn new(
        database: Arc<database_manager::App>,
        get_org_by_username: Arc<
            dyn crate::usecase::GetOrganizationByUsernameQuery,
        >,
        get_repo_by_username: Arc<
            dyn crate::usecase::GetRepoByUsernameQuery,
        >,
    ) -> Arc<Self> {
        Arc::new(Self {
            database,
            get_org_by_username,
            get_repo_by_username,
        })
    }
}

#[async_trait::async_trait]
impl SearchDataInputPort for SearchData {
    /// TODO: add English documentation
    #[tracing::instrument(name = "SearchData::execute", skip(self))]
    async fn execute(
        &self,
        input: &SearchDataInputData,
    ) -> errors::Result<(
        Vec<domain::Data>,
        Vec<domain::Property>,
        OffsetPaginator,
    )> {
        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await?
            .ok_or(errors::not_found!(
                "organization not found in search data"
            ))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await?
            .ok_or(errors::not_found!("repo not found in search data"))?;

        let properties = self
            .database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: org.id().clone(),
                database_id: repo.databases().first().unwrap().clone(),
            })
            .await?;
        let (data, paginator) = self
            .database
            .search_data()
            .execute(&database_manager::SearchDataInputData {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: org.id(),
                database_id: repo.databases().first().cloned(),
                query: input.name,
                page: input.page,
                page_size: input.page_size,
            })
            .await?;

        // TODO: add English comment
        Ok((data, properties, paginator))
    }
}
