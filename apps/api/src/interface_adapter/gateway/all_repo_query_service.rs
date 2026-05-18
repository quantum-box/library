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
    async fn load_databases(
        &self,
        repo_rows: &[RepoRowOnQuery],
    ) -> Result<Vec<DatabaseRow>> {
        let mut databases = Vec::new();

        for repo_id in repo_rows.iter().map(|r| r.id.as_str()) {
            let mut rows = sqlx::query_as::<_, DatabaseRow>(
                "SELECT database_id, repo_id FROM library.databases
                WHERE platform_id = ? AND repo_id = ?",
            )
            .bind(crate::domain::LIBRARY_TENANT.to_string())
            .bind(repo_id)
            .fetch_all(self.db.pool().as_ref())
            .await
            .map_err(|e| Error::application_logic_error(e.to_string()))?;

            databases.append(&mut rows);
        }

        Ok(databases)
    }

    async fn to_entity(
        &self,
        r: RepoRowOnQuery,
        databases: Vec<DatabaseRow>,
    ) -> Result<Repo> {
        let database_ids = databases
            .into_iter()
            .map(|d| d.database_id.parse().map_err(Error::from))
            .collect::<Result<Vec<_>>>()?;

        Ok(Repo::new(
            &r.id.parse()?,
            &r.org_id.parse()?,
            &r.org_username.parse()?,
            &r.name.parse()?,
            &r.username.parse()?,
            r.is_public == 1,
            r.description
                .filter(|d| !d.is_empty())
                .map(|d| d.parse())
                .transpose()?,
            database_ids,
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

        let databases = self.load_databases(&repo_rows).await?;

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
        query: &AllRepoQuerySearchInOrgQueryData,
    ) -> Result<Vec<Repo>> {
        let repo_rows = if let Some(name) = &query.name {
            sqlx::query_as::<_, RepoRowOnQuery>(
                "SELECT id, name, username, org_id, org_username, description, is_public FROM library.repos
                WHERE platform_id = ? AND org_username = ? AND name LIKE ? LIMIT ?",
            )
            .bind(crate::domain::LIBRARY_TENANT.to_string())
            .bind(&query.org_username)
            .bind(format!("%{}%", name))
            .bind(query.limit.unwrap_or(10))
            .fetch_all(self.db.pool().as_ref())
            .await
        } else {
            sqlx::query_as::<_, RepoRowOnQuery>(
                "SELECT id, org_id, org_username, name, username, description, is_public FROM library.repos
                WHERE platform_id = ? AND org_username = ? LIMIT ?",
            )
            .bind(crate::domain::LIBRARY_TENANT.to_string())
            .bind(&query.org_username)
            .bind(query.limit.unwrap_or(10))
            .fetch_all(self.db.pool().as_ref())
            .await
        }
        .map_err(|e| Error::application_logic_error(e.to_string()))?;

        let databases = self.load_databases(&repo_rows).await?;

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
}
