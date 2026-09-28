use std::sync::Arc;

use sqlx::Row;

use crate::domain::{
    RepoId, RichTextTemplate, RichTextTemplateId,
    RichTextTemplateRepository,
};

#[derive(Debug)]
pub struct RichTextTemplateRepositoryImpl {
    db: Arc<persistence::Db>,
}

impl RichTextTemplateRepositoryImpl {
    pub fn new(db: Arc<persistence::Db>) -> Self {
        Self { db }
    }
}

fn parse_template_row(
    row: sqlx::mysql::MySqlRow,
) -> errors::Result<RichTextTemplate> {
    let id: String = row
        .try_get("id")
        .map_err(errors::Error::internal_server_error)?;
    let repo_id: String = row
        .try_get("repo_id")
        .map_err(errors::Error::internal_server_error)?;
    let name: String = row
        .try_get("name")
        .map_err(errors::Error::internal_server_error)?;
    let rich_text: String = row
        .try_get("rich_text")
        .map_err(errors::Error::internal_server_error)?;
    Ok(RichTextTemplate::new(
        id.parse().map_err(errors::Error::internal_server_error)?,
        repo_id
            .parse()
            .map_err(errors::Error::internal_server_error)?,
        name,
        rich_text,
    ))
}

#[async_trait::async_trait]
impl RichTextTemplateRepository for RichTextTemplateRepositoryImpl {
    async fn find_by_repo(
        &self,
        repo_id: &RepoId,
    ) -> errors::Result<Vec<RichTextTemplate>> {
        let rows = sqlx::query(
            "SELECT id, repo_id, name, rich_text
             FROM repo_rich_text_templates
             WHERE repo_id = ?
             ORDER BY name, id",
        )
        .bind(repo_id.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        rows.into_iter().map(parse_template_row).collect()
    }

    async fn create(
        &self,
        template: &RichTextTemplate,
    ) -> errors::Result<()> {
        sqlx::query(
            "INSERT INTO repo_rich_text_templates
                 (id, repo_id, name, rich_text)
             VALUES (?, ?, ?, ?)",
        )
        .bind(template.id().to_string())
        .bind(template.repo_id().to_string())
        .bind(template.name())
        .bind(template.rich_text())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(map_write_error)?;
        Ok(())
    }

    async fn update(
        &self,
        template: &RichTextTemplate,
    ) -> errors::Result<bool> {
        let result = sqlx::query(
            "UPDATE repo_rich_text_templates
             SET name = ?, rich_text = ?, updated_at = CURRENT_TIMESTAMP
             WHERE repo_id = ? AND id = ?",
        )
        .bind(template.name())
        .bind(template.rich_text())
        .bind(template.repo_id().to_string())
        .bind(template.id().to_string())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(map_write_error)?;
        if result.rows_affected() > 0 {
            return Ok(true);
        }

        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM repo_rich_text_templates
             WHERE repo_id = ? AND id = ?",
        )
        .bind(template.repo_id().to_string())
        .bind(template.id().to_string())
        .fetch_one(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;
        Ok(exists > 0)
    }

    async fn delete(
        &self,
        repo_id: &RepoId,
        template_id: &RichTextTemplateId,
    ) -> errors::Result<bool> {
        let result = sqlx::query(
            "DELETE FROM repo_rich_text_templates
             WHERE repo_id = ? AND id = ?",
        )
        .bind(repo_id.to_string())
        .bind(template_id.to_string())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;
        Ok(result.rows_affected() > 0)
    }
}

fn map_write_error(error: sqlx::Error) -> errors::Error {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.code().as_deref() == Some("1062") {
            return errors::Error::bad_request(
                "A RichText template with this name already exists",
            );
        }
    }
    errors::Error::internal_server_error(error)
}
