//! SQLx implementation of SyncOperationRepository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use std::sync::Arc;

use inbound_sync_domain::{
    ProcessingStats, SyncOperation, SyncOperationId,
    SyncOperationRepository, SyncOperationStatus, SyncOperationType,
    WebhookEndpointId,
};

/// SQLx implementation of SyncOperationRepository.
#[derive(Debug, Clone)]
pub struct SqlxSyncOperationRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxSyncOperationRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SyncOperationRow {
    id: String,
    endpoint_id: String,
    operation_type: String,
    status: String,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    stats: Option<serde_json::Value>,
    error_message: Option<String>,
    progress: Option<String>,
}

impl TryFrom<SyncOperationRow> for SyncOperation {
    type Error = errors::Error;

    fn try_from(row: SyncOperationRow) -> Result<Self, Self::Error> {
        let operation_type: SyncOperationType =
            row.operation_type.parse()?;
        let status: SyncOperationStatus = row.status.parse()?;

        let stats: Option<ProcessingStats> = row
            .stats
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| errors::Error::invalid(e.to_string()))?;

        Ok(SyncOperation::new(
            SyncOperationId::from(row.id),
            WebhookEndpointId::from(row.endpoint_id),
            operation_type,
            status,
            row.started_at,
            row.completed_at,
            stats,
            row.error_message,
            row.progress,
        ))
    }
}

#[async_trait]
impl SyncOperationRepository for SqlxSyncOperationRepository {
    async fn save(&self, operation: &SyncOperation) -> errors::Result<()> {
        let stats_json = operation
            .stats()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| errors::Error::invalid(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO sync_operations
                (id, endpoint_id, operation_type, status, started_at,
                 completed_at, stats, error_message, progress)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                status = VALUES(status),
                completed_at = VALUES(completed_at),
                stats = VALUES(stats),
                error_message = VALUES(error_message),
                progress = VALUES(progress)
            "#,
        )
        .bind(operation.id().to_string())
        .bind(operation.endpoint_id().to_string())
        .bind(operation.operation_type().as_str())
        .bind(operation.status().as_str())
        .bind(operation.started_at())
        .bind(operation.completed_at())
        .bind(&stats_json)
        .bind(operation.error_message())
        .bind(operation.progress())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &SyncOperationId,
    ) -> errors::Result<Option<SyncOperation>> {
        let row: Option<SyncOperationRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, operation_type, status, started_at,
                   completed_at, stats, error_message, progress
            FROM sync_operations
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_endpoint(
        &self,
        endpoint_id: &WebhookEndpointId,
        limit: u32,
        offset: u32,
    ) -> errors::Result<Vec<SyncOperation>> {
        let rows: Vec<SyncOperationRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, operation_type, status, started_at,
                   completed_at, stats, error_message, progress
            FROM sync_operations
            WHERE endpoint_id = ?
            ORDER BY started_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(endpoint_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn find_pending(
        &self,
        limit: u32,
    ) -> errors::Result<Vec<SyncOperation>> {
        let rows: Vec<SyncOperationRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, operation_type, status, started_at,
                   completed_at, stats, error_message, progress
            FROM sync_operations
            WHERE status = 'queued'
            ORDER BY started_at ASC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }
}
