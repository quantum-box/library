use std::sync::Arc;

use crate::domain::repo::{RepoId, Source, SourceId, SourceRepository};
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
            "INSERT INTO library.sources (id, repo_id, name, url) 
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
                if e.to_string()
                    .contains("Table 'library.sources' doesn't exist")
                {
                    // TODO: add English comment
                    sqlx::query!(
                        "CREATE TABLE IF NOT EXISTS library.sources (
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
                        "INSERT INTO library.sources (id, repo_id, name, url) 
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
            "SELECT id, repo_id, name, url FROM library.sources WHERE id = ?",
            id.to_string()
        )
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        Ok(source.map(|s| {
            Source::new(
                &s.id.parse().unwrap(),
                &s.repo_id.parse().unwrap(),
                &s.name.parse().unwrap(),
                s.url.map(|u| u.parse().unwrap()),
            )
        }))
    }

    async fn find_by_repo_id(
        &self,
        repo_id: &RepoId,
    ) -> errors::Result<Vec<Source>> {
        let sources = sqlx::query_as!(
            SourceRow,
            "SELECT id, repo_id, name, url FROM library.sources WHERE repo_id = ?",
            repo_id.to_string()
        )
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        Ok(sources
            .into_iter()
            .map(|s| {
                Source::new(
                    &s.id.parse().unwrap(),
                    &s.repo_id.parse().unwrap(),
                    &s.name.parse().unwrap(),
                    s.url.map(|u| u.parse().unwrap()),
                )
            })
            .collect())
    }

    async fn delete(&self, id: &SourceId) -> errors::Result<()> {
        let result = sqlx::query!(
            "DELETE FROM library.sources WHERE id = ?",
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
