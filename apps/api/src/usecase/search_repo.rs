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
    pub org_username: String,
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
                    org_username: org_username.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct RecordingAllRepoQuery {
        calls: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl AllRepoQuery for RecordingAllRepoQuery {
        async fn search(
            &self,
            query: &AllRepoQuerySearchDto,
        ) -> errors::Result<Vec<Repo>> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("all:{:?}:{:?}", query.name, query.limit));
            Ok(vec![])
        }

        async fn search_in_org(
            &self,
            query: &AllRepoQuerySearchInOrgQueryData,
        ) -> errors::Result<Vec<Repo>> {
            self.calls.lock().unwrap().push(format!(
                "org:{}:{:?}:{:?}",
                query.org_username, query.name, query.limit
            ));
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn execute_routes_to_org_search_when_org_username_is_set() {
        let query = Arc::new(RecordingAllRepoQuery::default());
        let usecase = SearchRepo::new(query.clone());

        let repos = usecase
            .execute(&SearchRepoInputData {
                org_username: Some("acme".to_string()),
                name: Some("docs".to_string()),
                limit: Some(7),
            })
            .await
            .unwrap();

        assert!(repos.is_empty());
        assert_eq!(
            query.calls.lock().unwrap().as_slice(),
            &["org:acme:Some(\"docs\"):Some(7)"]
        );
    }

    #[tokio::test]
    async fn execute_routes_to_global_search_without_org_username() {
        let query = Arc::new(RecordingAllRepoQuery::default());
        let usecase = SearchRepo::new(query.clone());

        let repos = usecase
            .execute(&SearchRepoInputData {
                org_username: None,
                name: Some("docs".to_string()),
                limit: Some(7),
            })
            .await
            .unwrap();

        assert!(repos.is_empty());
        assert_eq!(
            query.calls.lock().unwrap().as_slice(),
            &["all:Some(\"docs\"):Some(7)"]
        );
    }
}
