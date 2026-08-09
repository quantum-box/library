use std::sync::Arc;

use crate::{
    domain::repo::{RepoId, Source, SourceId, SourceRepository},
    interface_adapter::gateway::row_parse::{
        is_missing_table, parse_stored,
    },
};
use persistence;

pub struct SourceRow {
    id: String,
    repo_id: String,
    name: String,
    url: Option<String>,
}

#[derive(Debug)]
pub struct SourceRepositoryImpl {
    db: Arc<persistence::Db>,
}

impl SourceRepositoryImpl {
    pub fn new(db: Arc<persistence::Db>) -> Self {
        Self { db }
    }
}

fn source_from_stored(row: SourceRow) -> errors::Result<Source> {
    let id = parse_stored("source", "id", &row.id)?;
    let repo_id = parse_stored("source", "repo_id", &row.repo_id)?;
    let name = parse_stored("source", "name", &row.name)?;
    let url = row
        .url
        .map(|url| parse_stored("source", "url", &url))
        .transpose()?;

    Ok(Source::new(&id, &repo_id, &name, url))
}

#[async_trait::async_trait]
impl SourceRepository for SourceRepositoryImpl {
    async fn save(&self, entity: &Source) -> errors::Result<()> {
        let mut tx = self
            .db
            .pool()
            .begin()
            .await
            .map_err(errors::Error::internal_server_error)?;

        let result = sqlx::query!(
            "INSERT INTO sources (id, repo_id, name, url)
             VALUES (?, ?, ?, ?) 
             ON DUPLICATE KEY UPDATE 
                name = VALUES(name), 
                url = VALUES(url)",
            entity.id().to_string(),
            entity.repo_id().to_string(),
            entity.name().to_string(),
            entity.url().as_ref().map(|u| u.to_string()),
        )
        .execute(&mut *tx)
        .await;

        match result {
            Ok(_) => {
                tx.commit()
                    .await
                    .map_err(errors::Error::internal_server_error)?;
                Ok(())
            }
            Err(e) => {
                tx.rollback()
                    .await
                    .map_err(errors::Error::internal_server_error)?;
                if is_missing_table(&e) {
                    // TODO: add English comment
                    sqlx::query!(
                        "CREATE TABLE IF NOT EXISTS sources (
                            id VARCHAR(255) NOT NULL PRIMARY KEY,
                            repo_id VARCHAR(255) NOT NULL,
                            name VARCHAR(255) NOT NULL,
                            url VARCHAR(255),
                            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                            INDEX (repo_id)
                        )"
                    )
                    .execute(self.db.pool().as_ref())
                    .await
                    .map_err(errors::Error::internal_server_error)?;

                    // TODO: add English comment
                    let mut tx =
                        self.db.pool().begin().await.map_err(
                            errors::Error::internal_server_error,
                        )?;

                    sqlx::query!(
                        "INSERT INTO sources (id, repo_id, name, url)
                         VALUES (?, ?, ?, ?)",
                        entity.id().to_string(),
                        entity.repo_id().to_string(),
                        entity.name().to_string(),
                        entity.url().as_ref().map(|u| u.to_string()),
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(errors::Error::internal_server_error)?;

                    tx.commit()
                        .await
                        .map_err(errors::Error::internal_server_error)?;
                    Ok(())
                } else {
                    Err(errors::Error::internal_server_error(e))
                }
            }
        }
    }

    async fn get_by_id(
        &self,
        id: &SourceId,
    ) -> errors::Result<Option<Source>> {
        let source = sqlx::query_as!(
            SourceRow,
            "SELECT id, repo_id, name, url FROM sources WHERE id = ?",
            id.to_string()
        )
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        source.map(source_from_stored).transpose()
    }

    async fn find_by_repo_id(
        &self,
        repo_id: &RepoId,
    ) -> errors::Result<Vec<Source>> {
        let sources = sqlx::query_as!(
            SourceRow,
            "SELECT id, repo_id, name, url FROM sources WHERE repo_id = ?",
            repo_id.to_string()
        )
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        sources.into_iter().map(source_from_stored).collect()
    }

    async fn delete(&self, id: &SourceId) -> errors::Result<()> {
        let result = sqlx::query!(
            "DELETE FROM sources WHERE id = ?",
            id.to_string()
        )
        .execute(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        if result.rows_affected() == 0 {
            return Err(errors::Error::not_found("Source not found"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_with_name(name: &str) -> SourceRow {
        SourceRow {
            id: "src_01hkz3700yt46snfewzpakeyj4".to_string(),
            repo_id: "rp_01hkz3700yt46snfewzpakeyj4".to_string(),
            name: name.to_string(),
            url: Some("https://example.com/repo".to_string()),
        }
    }

    #[test]
    fn source_row_empty_name_returns_error_instead_of_panicking() {
        let result = source_from_stored(row_with_name(""));

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("invalid stored source.name"));
        assert!(message.contains("en.err.empty_type"));
    }
}
