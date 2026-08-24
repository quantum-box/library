use std::sync::Arc;

use crate::{
    domain::{Repo, RepoId, RepoRepository},
    interface_adapter::gateway::row_parse::{
        is_missing_table, parse_stored,
    },
};
use derive_new::new;
use value_object::TenantId;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RepoRow {
    id: String,
    org_id: String,
    org_username: String,
    name: String,
    username: String,
    description: Option<String>,
    is_public: i8,
}

#[derive(Debug, Clone, new)]
pub struct RepoRepositoryImpl {
    db: Arc<persistence::Db>,
}

fn repo_from_stored(
    row: RepoRow,
    database_ids: Vec<String>,
    tags: Vec<String>,
) -> errors::Result<Repo> {
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
        .collect::<errors::Result<_>>()?;
    let tags = tags
        .into_iter()
        .map(|tag| parse_stored("repo_tag", "tag", &tag))
        .collect::<errors::Result<_>>()?;

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
impl RepoRepository for RepoRepositoryImpl {
    // TODO: add English comment
    async fn save(&self, entity: &Repo) -> errors::Result<()> {
        let mut tx = self
            .db
            .pool()
            .begin()
            .await
            .map_err(errors::Error::internal_server_error)?;

        sqlx::query!(
            "INSERT INTO repos (id, org_id, org_username, username, name, description, is_public, platform_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE org_id = VALUES(org_id), org_username = VALUES(org_username), username = VALUES(username), name = VALUES(name), description = VALUES(description), is_public = VALUES(is_public), platform_id = VALUES(platform_id)",
            entity.id().to_string(),
            entity.organization_id().to_string(),
            entity.org_username().to_string(),
            entity.username().to_string(),
            entity.name().to_string(),
            entity.description().clone().map(|d| d.to_string()),
            i8::from(*entity.is_public()),
            crate::domain::LIBRARY_TENANT.to_string()
        )
        .execute(&mut *tx)
        .await
        .map_err(errors::Error::internal_server_error)?;

        sqlx::query!(
            "DELETE FROM `databases` WHERE repo_id = ?",
            entity.id().to_string()
        )
        .execute(&mut *tx)
        .await
        .map_err(errors::Error::internal_server_error)?;

        for database in entity.databases() {
            sqlx::query!(
                "INSERT INTO `databases` (database_id, repo_id, platform_id) VALUES (?, ?, ?)",
                database.to_string(),
                entity.id().to_string(),
                crate::domain::LIBRARY_TENANT.to_string()
            )
            .execute(&mut *tx)
            .await
            .map_err(errors::Error::internal_server_error)?;
        }

        // Delete existing tags if the table exists
        let delete_tags_result = sqlx::query!(
            "DELETE FROM tags WHERE repo_id = ?",
            entity.id().to_string()
        )
        .execute(&mut *tx)
        .await;

        // TODO: add English comment
        if let Err(e) = delete_tags_result {
            if !is_missing_table(&e) {
                return Err(errors::Error::internal_server_error(e));
            }
        }

        // Delete existing sources if the table exists
        let delete_sources_result = sqlx::query!(
            "DELETE FROM sources WHERE repo_id = ?",
            entity.id().to_string()
        )
        .execute(&mut *tx)
        .await;

        // TODO: add English comment
        if let Err(e) = delete_sources_result {
            if !is_missing_table(&e) {
                return Err(errors::Error::internal_server_error(e));
            }
        }

        // Insert tags if the table exists
        for tag in entity.tags() {
            let insert_tag_result = sqlx::query!(
                "INSERT INTO tags (id, repo_id, tag, platform_id) VALUES (?, ?, ?, ?)",
                value_object::Ulid::new().to_string(),
                entity.id().to_string(),
                tag.to_string(),
                crate::domain::LIBRARY_TENANT.to_string()
            )
            .execute(&mut *tx)
            .await;

            // TODO: add English comment
            if let Err(e) = insert_tag_result {
                if !is_missing_table(&e) {
                    return Err(errors::Error::internal_server_error(e));
                }
            }
        }

        // Insert sources if the table exists
        // TODO: add English comment

        tx.commit()
            .await
            .map_err(errors::Error::internal_server_error)?;
        Ok(())
    }

    async fn get_by_id(
        &self,
        _tenant_id: &TenantId,
        id: &RepoId,
    ) -> errors::Result<Option<Repo>> {
        let row = sqlx::query_as!(
            RepoRow,
            "SELECT id, org_id, org_username, name, username, description, is_public FROM repos WHERE platform_id = ? AND id = ?",
            crate::domain::LIBRARY_TENANT.to_string(),
            id.to_string()
        )
        .fetch_one(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        let databases = sqlx::query!(
            "SELECT id, database_id FROM `databases` WHERE platform_id = ? AND repo_id = ?",
            crate::domain::LIBRARY_TENANT.to_string(),
            id.to_string()
        )
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        // Fetch tags if the table exists
        let tags_result = sqlx::query!(
            "SELECT tag FROM tags WHERE platform_id = ? AND repo_id = ?",
            crate::domain::LIBRARY_TENANT.to_string(),
            id.to_string()
        )
        .fetch_all(self.db.pool().as_ref())
        .await;

        let tags = match tags_result {
            Ok(tags) => tags,
            Err(e) => {
                if is_missing_table(&e) {
                    vec![]
                } else {
                    return Err(errors::Error::internal_server_error(e));
                }
            }
        };

        // Fetch sources if the table exists
        let sources_result = sqlx::query!(
            "SELECT name, url FROM sources WHERE repo_id = ?",
            id.to_string()
        )
        .fetch_all(self.db.pool().as_ref())
        .await;

        let _sources = match sources_result {
            Ok(sources) => sources,
            Err(e) => {
                if is_missing_table(&e) {
                    vec![]
                } else {
                    return Err(errors::Error::internal_server_error(e));
                }
            }
        };

        Ok(Some(repo_from_stored(
            row,
            databases.into_iter().map(|d| d.database_id).collect(),
            tags.into_iter().map(|t| t.tag).collect(),
        )?))
    }

    #[tracing::instrument(
        name = "RepoRepositoryImpl::find_all",
        skip(self)
    )]
    async fn find_all(
        &self,
        org_id: &TenantId,
    ) -> errors::Result<Vec<Repo>> {
        let rows = sqlx::query_as!(
            RepoRow,
                "SELECT id, org_id, org_username, name, username, description, is_public FROM repos WHERE platform_id = ? AND org_id = ?",
                crate::domain::LIBRARY_TENANT.to_string(),
                org_id.to_string()
        )
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        let mut repos = vec![];
        for row in rows.iter() {
            let database_rows = sqlx::query!(
                "SELECT id, database_id, repo_id FROM `databases` WHERE platform_id = ? AND repo_id = ?",
                crate::domain::LIBRARY_TENANT.to_string(),
                row.id.to_string()
            )
            .fetch_all(self.db.pool().as_ref())
            .await
            .map_err(errors::Error::internal_server_error)?;

            repos.push(repo_from_stored(
                row.clone(),
                database_rows.into_iter().map(|d| d.database_id).collect(),
                vec![],
            )?);
        }
        Ok(repos)
    }

    async fn delete(
        &self,
        _tenant_id: &TenantId,
        id: &RepoId,
    ) -> errors::Result<()> {
        sqlx::query!(
            "DELETE FROM repos WHERE platform_id = ? AND id = ?",
            crate::domain::LIBRARY_TENANT.to_string(),
            id.to_string()
        )
        .execute(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;
        Ok(())
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
        let row = row_with_name("");

        let result = repo_from_stored(row, vec![], vec![]);

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("invalid stored repo.name"));
        assert!(message.contains("en.err.empty_type"));
    }
}
