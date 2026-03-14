#![allow(dead_code)]

use super::{SearchRepoInputData, SearchRepoInputPort};
use crate::domain::Repo;
use std::{fmt::Debug, sync::Arc};

#[derive(Debug, Clone)]
pub struct AllRepoQuerySearchDto {
    pub name: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AllRepoQuerySearchInOrgQueryData {
    pub org_id: String,
    pub name: Option<String>,
    pub limit: Option<i64>,
}

#[async_trait::async_trait]
pub trait AllRepoQuery: Debug + Send + Sync {
    async fn search(
        &self,
        query: &AllRepoQuerySearchDto,
    ) -> errors::Result<Vec<Repo>>;
    async fn search_in_org(
        &self,
        query: &AllRepoQuerySearchInOrgQueryData,
    ) -> errors::Result<Vec<Repo>>;
}

#[derive(Debug, Clone)]
pub struct SearchRepo {
    all_repo_query: Arc<dyn AllRepoQuery>,
}

impl SearchRepo {
    pub fn new(all_repo_query: Arc<dyn AllRepoQuery>) -> Arc<Self> {
        Arc::new(Self { all_repo_query })
    }
}

#[async_trait::async_trait]
impl SearchRepoInputPort for SearchRepo {
    /// TODO: add English documentation
    #[tracing::instrument(name = "SearchRepo::execute", skip(self))]
    async fn execute(
        &self,
        input: &SearchRepoInputData,
    ) -> errors::Result<Vec<Repo>> {
        if let Some(org_username) = &input.org_username {
            let repos = self
                .all_repo_query
                .search_in_org(&AllRepoQuerySearchInOrgQueryData {
                    org_id: org_username.clone(),
                    name: input.name.clone(),
                    limit: input.limit,
                })
                .await?;
            return Ok(repos);
        };
        let repos = self
            .all_repo_query
            .search(&AllRepoQuerySearchDto {
                name: input.name.clone(),
                limit: input.limit,
            })
            .await?;
        Ok(repos)
    }
}
