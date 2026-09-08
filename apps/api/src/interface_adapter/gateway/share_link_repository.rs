//! SQLx implementation of [`ShareLinkRepository`].
//!
//! Lookups on the read path are by `token_hash` alone, with no tenant or
//! repo in the predicate: the caller presenting a token has not told us
//! which repo it belongs to, and the hash's uniqueness is what answers
//! that. Everything the token then authorizes is decided from the row
//! it resolves to, never from anything the caller supplied.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{RepoId, ShareLink, ShareLinkId, ShareLinkRepository};

#[derive(Debug, FromRow)]
struct ShareLinkRow {
    id: String,
    token_hash: String,
    repo_id: String,
    data_id: String,
    name: Option<String>,
    created_by: Option<String>,
    created_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl TryFrom<ShareLinkRow> for ShareLink {
    type Error = errors::Error;

    fn try_from(row: ShareLinkRow) -> Result<Self, Self::Error> {
        let id: ShareLinkId =
            row.id.parse().map_err(|e: errors::ParseIdError| {
                errors::Error::invalid(e.to_string())
            })?;
        let repo_id: RepoId =
            row.repo_id.parse().map_err(|e: errors::ParseIdError| {
                errors::Error::invalid(e.to_string())
            })?;
        let name = row
            .name
            .map(|name| {
                name.parse().map_err(|e: anyhow::Error| {
                    errors::Error::invalid(e.to_string())
                })
            })
            .transpose()?;

        Ok(ShareLink::new(
            id,
            row.token_hash,
            repo_id,
            row.data_id,
            name,
            row.created_by,
            row.created_at,
            row.revoked_at,
        ))
    }
}

const SELECT_COLUMNS: &str = r#"`id`, `token_hash`, `repo_id`, `data_id`,
       `name`, `created_by`, `created_at`, `revoked_at`"#;

#[derive(Debug)]
pub struct ShareLinkRepositoryImpl {
    db: Arc<persistence::Db>,
}

impl ShareLinkRepositoryImpl {
    pub fn new(db: Arc<persistence::Db>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl ShareLinkRepository for ShareLinkRepositoryImpl {
    async fn insert(&self, entity: &ShareLink) -> errors::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO `data_share_links`
                (`id`, `token_hash`, `repo_id`, `data_id`, `name`,
                 `created_by`, `created_at`, `revoked_at`)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(entity.id().to_string())
        .bind(entity.token_hash())
        .bind(entity.repo_id().to_string())
        .bind(entity.data_id())
        .bind(entity.name().as_ref().map(|name| name.to_string()))
        .bind(entity.created_by())
        .bind(entity.created_at())
        .bind(entity.revoked_at())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;
        Ok(())
    }

    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> errors::Result<Option<ShareLink>> {
        let row: Option<ShareLinkRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLUMNS} FROM `data_share_links` \
             WHERE `token_hash` = ?"
        ))
        .bind(token_hash)
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        row.map(ShareLink::try_from).transpose()
    }

    async fn find_for_data(
        &self,
        repo_id: &RepoId,
        data_id: &str,
    ) -> errors::Result<Vec<ShareLink>> {
        let rows: Vec<ShareLinkRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLUMNS} FROM `data_share_links` \
             WHERE `repo_id` = ? AND `data_id` = ? \
             ORDER BY `created_at` DESC"
        ))
        .bind(repo_id.to_string())
        .bind(data_id)
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        rows.into_iter().map(ShareLink::try_from).collect()
    }

    async fn get_by_id(
        &self,
        id: &ShareLinkId,
    ) -> errors::Result<Option<ShareLink>> {
        let row: Option<ShareLinkRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLUMNS} FROM `data_share_links` \
             WHERE `id` = ?"
        ))
        .bind(id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        row.map(ShareLink::try_from).transpose()
    }

    async fn revoke(
        &self,
        id: &ShareLinkId,
        revoked_at: DateTime<Utc>,
    ) -> errors::Result<()> {
        // `revoked_at IS NULL` keeps the first revocation's timestamp:
        // re-revoking is a no-op rather than a rewrite of when access
        // actually stopped.
        sqlx::query(
            r#"
            UPDATE `data_share_links`
            SET `revoked_at` = ?
            WHERE `id` = ? AND `revoked_at` IS NULL
            "#,
        )
        .bind(revoked_at)
        .bind(id.to_string())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;
        Ok(())
    }
}
