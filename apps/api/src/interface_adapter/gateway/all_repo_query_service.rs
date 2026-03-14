use crate::{
    domain::Repo,
    usecase::{
        AllRepoQuery, AllRepoQuerySearchDto,
        AllRepoQuerySearchInOrgQueryData,
    },
};
use derive_new::new;
use errors::{Error, Result};
use sqlx::prelude::FromRow;
use std::sync::Arc;

#[derive(Debug, Clone, FromRow)]
pub struct RepoRowOnQuery {
    id: String,
    name: String,
    username: String,
    org_id: String,
    org_username: String,
    description: Option<String>,
    is_public: i8,
}

#[derive(Debug, Clone, FromRow)]
pub struct DatabaseRow {
    database_id: String,
    repo_id: String,
}

#[derive(Debug, Clone, new)]
pub struct AllRepoQueryServiceImpl {
    db: Arc<persistence::Db>,
}

impl AllRepoQueryServiceImpl {
    async fn to_entity(
        &self,
        r: RepoRowOnQuery,
        databases: Vec<DatabaseRow>,
    ) -> Result<Repo> {
        Ok(Repo::new(
            &r.id.parse().unwrap(),
            &r.org_id.parse().unwrap(),
            &r.org_username.parse().unwrap(),
            &r.name.parse().unwrap(),
            &r.username.parse().unwrap(),
            r.is_public == 1,
            r.description.map(|d| d.parse()).transpose().unwrap(),
            databases
                .into_iter()
                .map(|d| d.database_id.parse().unwrap())
                .collect(),
            vec![], // tags
        ))
    }
}

#[async_trait::async_trait]
impl AllRepoQuery for AllRepoQueryServiceImpl {
    async fn search(
        &self,
        query: &AllRepoQuerySearchDto,
    ) -> errors::Result<Vec<Repo>> {
        let repo_rows = if let Some(name) = &query.name {
            sqlx::query_as!(
                RepoRowOnQuery,
                "SELECT id, name, username, org_id, org_username, description, is_public FROM library.repos
                WHERE platform_id = ? AND name LIKE ? LIMIT ?",
                crate::domain::LIBRARY_TENANT.to_string(),
                format!("%{}%", name),
                query.limit.unwrap_or(10),
            )
            .fetch_all(self.db.pool().as_ref())
            .await
        } else {
            sqlx::query_as!(RepoRowOnQuery, "SELECT id, org_id, org_username, name, username, description, is_public FROM library.repos WHERE platform_id = ?", crate::domain::LIBRARY_TENANT.to_string()) 
                .fetch_all(self.db.pool().as_ref())
                .await
        }
        .map_err(|e| Error::application_logic_error(e.to_string()))?;

        let databases = sqlx::query_as!(
            DatabaseRow,
            "SELECT database_id, repo_id FROM library.databases
            WHERE platform_id = ? AND repo_id IN (?)",
            crate::domain::LIBRARY_TENANT.to_string(),
            repo_rows
                .iter()
                .map(|p| p.id.clone())
                .collect::<Vec<String>>()
                .join(","),
        )
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(|e| Error::application_logic_error(e.to_string()))?;

        let mut repos = Vec::new();
        for r in repo_rows {
            let databases = databases
                .iter()
                .filter(|d| d.repo_id == r.id)
                .cloned()
                .collect::<Vec<DatabaseRow>>();
            repos.push(self.to_entity(r, databases).await?);
        }
        Ok(repos)
    }

    async fn search_in_org(
        &self,
        _query: &AllRepoQuerySearchInOrgQueryData,
    ) -> Result<Vec<Repo>> {
        unimplemented!();
    }
}
