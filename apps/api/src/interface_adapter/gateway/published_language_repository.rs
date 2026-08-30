use std::collections::HashSet;
use std::sync::Arc;

use crate::domain::repo::RepoId;
use crate::domain::translation::{
    LanguageTag, PublishedLanguageRepository,
};

#[derive(Debug)]
pub struct PublishedLanguageRepositoryImpl {
    db: Arc<persistence::Db>,
}

impl PublishedLanguageRepositoryImpl {
    pub fn new(db: Arc<persistence::Db>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl PublishedLanguageRepository for PublishedLanguageRepositoryImpl {
    async fn find_by_repo(
        &self,
        repo_id: &RepoId,
    ) -> errors::Result<Vec<LanguageTag>> {
        let rows = sqlx::query!(
            "SELECT lang FROM repo_published_languages
             WHERE repo_id = ?
             ORDER BY lang",
            repo_id.to_string(),
        )
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        rows.into_iter()
            .map(|row| {
                // A tag that fails to parse was written by an older or
                // broken build. Surfacing it is better than silently
                // dropping a language the owner believes is published.
                LanguageTag::new(&row.lang).map_err(|error| {
                    errors::Error::internal_server_error(format!(
                        "stored published language `{}` is not a valid \
                         language tag: {error}",
                        row.lang
                    ))
                })
            })
            .collect()
    }

    async fn replace_for_repo(
        &self,
        repo_id: &RepoId,
        languages: &[LanguageTag],
    ) -> errors::Result<()> {
        let desired: HashSet<String> =
            languages.iter().map(|tag| tag.to_string()).collect();

        let mut tx = self
            .db
            .pool()
            .begin()
            .await
            .map_err(errors::Error::internal_server_error)?;

        let existing = sqlx::query!(
            "SELECT lang FROM repo_published_languages WHERE repo_id = ?",
            repo_id.to_string(),
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(errors::Error::internal_server_error)?
        .into_iter()
        .map(|row| row.lang)
        .collect::<HashSet<_>>();

        // Diff rather than delete-and-reinsert so that `enabled_at`
        // keeps recording when a language was first published, which is
        // what the progress view reports against.
        for stale in existing.difference(&desired) {
            sqlx::query!(
                "DELETE FROM repo_published_languages
                 WHERE repo_id = ? AND lang = ?",
                repo_id.to_string(),
                stale.as_str(),
            )
            .execute(&mut *tx)
            .await
            .map_err(errors::Error::internal_server_error)?;
        }

        for added in desired.difference(&existing) {
            sqlx::query!(
                "INSERT INTO repo_published_languages (repo_id, lang)
                 VALUES (?, ?)",
                repo_id.to_string(),
                added.as_str(),
            )
            .execute(&mut *tx)
            .await
            .map_err(errors::Error::internal_server_error)?;
        }

        tx.commit()
            .await
            .map_err(errors::Error::internal_server_error)?;

        Ok(())
    }
}
