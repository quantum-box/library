use std::sync::Arc;

use errors::Result;
use sqlx::prelude::FromRow;
use value_object::Identifier;

use crate::{domain::Repo, usecase::GetRepoByUsernameQuery};

#[derive(Debug, Clone, FromRow)]
pub struct RepoRow {
    id: String,
    org_id: String,
    org_username: String,
    name: String,
    username: String,
    description: Option<String>,
    is_public: i8,
}

#[derive(Debug, Clone)]
pub struct GetRepoByUsernameQueryImpl {
    db: Arc<persistence::Db>,
}

impl GetRepoByUsernameQueryImpl {
    pub fn new(db: Arc<persistence::Db>) -> Arc<Self> {
        Arc::new(Self { db })
    }
}

#[async_trait::async_trait]
impl GetRepoByUsernameQuery for GetRepoByUsernameQueryImpl {
    async fn execute(
        &self,
        operator_alias: &Identifier,
        repo_alias: &Identifier,
    ) -> Result<Option<Repo>> {
        let row_opt = sqlx::query_as!(
            RepoRow,
            "SELECT id, org_id, org_username, name, username, description, is_public FROM library.repos WHERE platform_id = ? AND org_username = ? AND username = ?",
            crate::domain::LIBRARY_TENANT.to_string(),
            operator_alias.to_string(),
            repo_alias.to_string()
        )
        .fetch_optional(self.db.pool().as_ref())
        .await?;

        if let Some(row) = row_opt {
            let databases = sqlx::query!(
                "SELECT id, database_id FROM library.databases WHERE platform_id = ? AND repo_id = ?",
                crate::domain::LIBRARY_TENANT.to_string(),
                row.id
            )
            .fetch_all(self.db.pool().as_ref())
            .await?;
            let tags_result = sqlx::query!(
                "SELECT tag FROM library.tags WHERE platform_id = ? AND repo_id = ?",
                crate::domain::LIBRARY_TENANT.to_string(),
                row.id
            )
            .fetch_all(self.db.pool().as_ref())
            .await;
            let tags = match tags_result {
                Ok(tags) => tags,
                Err(e) => {
                    if e.to_string()
                        .contains("Table 'library.tags' doesn't exist")
                    {
                        vec![]
                    } else {
                        return Err(errors::Error::internal_server_error(
                            e,
                        ));
                    }
                }
            };
            Ok(Some(Repo::new(
                &row.id.parse().unwrap(),
                &row.org_id.parse().unwrap(),
                &row.org_username.parse().unwrap(),
                &row.name.parse().unwrap(),
                &row.username.parse().unwrap(),
                row.is_public == 1,
                row.description.map(|d| d.parse()).transpose()?,
                databases
                    .into_iter()
                    .map(|d| d.database_id.parse().unwrap())
                    .collect(),
                tags.into_iter().map(|t| t.tag.parse().unwrap()).collect(),
            )))
        } else {
            Ok(None)
        }
    }
}
