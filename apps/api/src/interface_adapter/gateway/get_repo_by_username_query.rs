use std::sync::Arc;

use errors::Result;
use sqlx::prelude::FromRow;
use value_object::Identifier;

use crate::{
    domain::Repo,
    interface_adapter::gateway::row_parse::{
        is_missing_table, parse_stored,
    },
    usecase::GetRepoByUsernameQuery,
};

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

fn repo_from_stored(
    row: RepoRow,
    database_ids: Vec<String>,
    tags: Vec<String>,
) -> Result<Repo> {
    let id = parse_stored("repo", "id", &row.id)?;
    let org_id = parse_stored("repo", "org_id", &row.org_id)?;
    let org_username =
        parse_stored("repo", "org_username", &row.org_username)?;
    let name = parse_stored("repo", "name", &row.name)?;
    let username = parse_stored("repo", "username", &row.username)?;
    let description = row
        .description
        .map(|d| parse_stored("repo", "description", &d))
        .transpose()?;
    let databases = database_ids
        .into_iter()
        .map(|database_id| {
            parse_stored("repo_database", "database_id", &database_id)
        })
        .collect::<Result<_>>()?;
    let tags = tags
        .into_iter()
        .map(|tag| parse_stored("repo_tag", "tag", &tag))
        .collect::<Result<_>>()?;

    Ok(Repo::new(
        &id,
        &org_id,
        &org_username,
        &name,
        &username,
        row.is_public == 1,
        description,
        databases,
        tags,
    ))
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
            "SELECT id, org_id, org_username, name, username, description, is_public FROM repos WHERE platform_id = ? AND org_username = ? AND username = ?",
            crate::domain::LIBRARY_TENANT.to_string(),
            operator_alias.to_string(),
            repo_alias.to_string()
        )
        .fetch_optional(self.db.pool().as_ref())
        .await?;

        if let Some(row) = row_opt {
            let databases = sqlx::query!(
                "SELECT id, database_id FROM databases WHERE platform_id = ? AND repo_id = ?",
                crate::domain::LIBRARY_TENANT.to_string(),
                row.id
            )
            .fetch_all(self.db.pool().as_ref())
            .await?;
            let tags_result = sqlx::query!(
                "SELECT tag FROM tags WHERE platform_id = ? AND repo_id = ?",
                crate::domain::LIBRARY_TENANT.to_string(),
                row.id
            )
            .fetch_all(self.db.pool().as_ref())
            .await;
            let tags = match tags_result {
                Ok(tags) => tags,
                Err(e) => {
                    if is_missing_table(&e) {
                        vec![]
                    } else {
                        return Err(errors::Error::internal_server_error(
                            e,
                        ));
                    }
                }
            };
            Ok(Some(repo_from_stored(
                row,
                databases.into_iter().map(|d| d.database_id).collect(),
                tags.into_iter().map(|t| t.tag).collect(),
            )?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_with_name(name: &str) -> RepoRow {
        RepoRow {
            id: "rp_01hkz3700yt46snfewzpakeyj4".to_string(),
            org_id: "tn_01hkz3700yt46snfewzpakeyj4".to_string(),
            org_username: "test-org".to_string(),
            name: name.to_string(),
            username: "test-repo".to_string(),
            description: None,
            is_public: 1,
        }
    }

    #[test]
    fn repo_row_empty_name_returns_error_instead_of_panicking() {
        let result = repo_from_stored(row_with_name(""), vec![], vec![]);

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("invalid stored repo.name"));
        assert!(message.contains("en.err.empty_type"));
    }
}
