//! SQLx implementation of SyncStateRepository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use std::sync::Arc;

use inbound_sync_domain::{
    SyncDirection, SyncState, SyncStateId, SyncStateRepository,
    WebhookEndpointId,
};

/// SQLx implementation of SyncStateRepository.
#[derive(Debug, Clone)]
pub struct SqlxSyncStateRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxSyncStateRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SyncStateRow {
    id: String,
    endpoint_id: String,
    data_id: String,
    external_id: String,
    external_version: Option<String>,
    local_version: Option<String>,
    sync_direction: String,
    last_synced_at: DateTime<Utc>,
    metadata: Option<serde_json::Value>,
}

impl TryFrom<SyncStateRow> for SyncState {
    type Error = errors::Error;

    fn try_from(row: SyncStateRow) -> Result<Self, Self::Error> {
        let direction: SyncDirection = row.sync_direction.parse()?;

        Ok(SyncState::new(
            SyncStateId::from(row.id),
            WebhookEndpointId::from(row.endpoint_id),
            row.data_id,
            row.external_id,
            row.external_version,
            row.local_version,
            direction,
            row.last_synced_at,
            row.metadata,
        ))
    }
}

#[async_trait]
impl SyncStateRepository for SqlxSyncStateRepository {
    async fn save(&self, state: &SyncState) -> errors::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sync_states
                (id, endpoint_id, data_id, external_id, external_version,
                 local_version, sync_direction, last_synced_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                external_version = VALUES(external_version),
                local_version = VALUES(local_version),
                last_synced_at = VALUES(last_synced_at),
                metadata = VALUES(metadata)
            "#,
        )
        .bind(state.id().to_string())
        .bind(state.endpoint_id().to_string())
        .bind(state.data_id())
        .bind(state.external_id())
        .bind(state.external_version())
        .bind(state.local_version())
        .bind(state.sync_direction().as_str())
        .bind(state.last_synced_at())
        .bind(state.metadata())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &SyncStateId,
    ) -> errors::Result<Option<SyncState>> {
        let row: Option<SyncStateRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, data_id, external_id, external_version,
                   local_version, sync_direction, last_synced_at, metadata
            FROM sync_states
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_external_id(
        &self,
        endpoint_id: &WebhookEndpointId,
        external_id: &str,
    ) -> errors::Result<Option<SyncState>> {
        let row: Option<SyncStateRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, data_id, external_id, external_version,
                   local_version, sync_direction, last_synced_at, metadata
            FROM sync_states
            WHERE endpoint_id = ? AND external_id = ?
            "#,
        )
        .bind(endpoint_id.to_string())
        .bind(external_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_data_id(
        &self,
        endpoint_id: &WebhookEndpointId,
        data_id: &str,
    ) -> errors::Result<Option<SyncState>> {
        let row: Option<SyncStateRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, data_id, external_id, external_version,
                   local_version, sync_direction, last_synced_at, metadata
            FROM sync_states
            WHERE endpoint_id = ? AND data_id = ?
            "#,
        )
        .bind(endpoint_id.to_string())
        .bind(data_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_endpoint(
        &self,
        endpoint_id: &WebhookEndpointId,
    ) -> errors::Result<Vec<SyncState>> {
        let rows: Vec<SyncStateRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, data_id, external_id, external_version,
                   local_version, sync_direction, last_synced_at, metadata
            FROM sync_states
            WHERE endpoint_id = ?
            ORDER BY last_synced_at DESC
            "#,
        )
        .bind(endpoint_id.to_string())
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn delete(&self, id: &SyncStateId) -> errors::Result<()> {
        sqlx::query("DELETE FROM sync_states WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| {
                errors::Error::internal_server_error(e.to_string())
            })?;

        Ok(())
    }

    async fn delete_by_endpoint(
        &self,
        endpoint_id: &WebhookEndpointId,
    ) -> errors::Result<u64> {
        let result =
            sqlx::query("DELETE FROM sync_states WHERE endpoint_id = ?")
                .bind(endpoint_id.to_string())
                .execute(self.pool.as_ref())
                .await
                .map_err(|e| {
                    errors::Error::internal_server_error(e.to_string())
                })?;

        Ok(result.rows_affected())
    }
}
