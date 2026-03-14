//! SQLx implementation of WebhookEventRepository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use std::sync::Arc;

use inbound_sync_domain::{
    ProcessingStats, ProcessingStatus, Provider, WebhookEndpointId,
    WebhookEvent, WebhookEventId, WebhookEventRepository,
};

/// SQLx implementation of WebhookEventRepository.
#[derive(Debug, Clone)]
pub struct SqlxWebhookEventRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxWebhookEventRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct WebhookEventRow {
    id: String,
    endpoint_id: String,
    provider: String,
    event_type: String,
    payload: serde_json::Value,
    headers: Option<serde_json::Value>,
    signature_valid: bool,
    processing_status: String,
    error_message: Option<String>,
    retry_count: i32,
    next_retry_at: Option<DateTime<Utc>>,
    stats: Option<serde_json::Value>,
    received_at: DateTime<Utc>,
    processed_at: Option<DateTime<Utc>>,
}

impl TryFrom<WebhookEventRow> for WebhookEvent {
    type Error = errors::Error;

    fn try_from(row: WebhookEventRow) -> Result<Self, Self::Error> {
        let provider: Provider = row
            .provider
            .parse()
            .map_err(|_| errors::Error::invalid("Invalid provider"))?;

        let status: ProcessingStatus = row.processing_status.parse()?;

        let stats: Option<ProcessingStats> = row
            .stats
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| errors::Error::invalid(e.to_string()))?;

        Ok(WebhookEvent::new(
            WebhookEventId::from(row.id),
            WebhookEndpointId::from(row.endpoint_id),
            provider,
            row.event_type,
            row.payload,
            row.headers,
            row.signature_valid,
            status,
            row.error_message,
            row.retry_count as u32,
            row.next_retry_at,
            stats,
            row.received_at,
            row.processed_at,
        ))
    }
}

#[async_trait]
impl WebhookEventRepository for SqlxWebhookEventRepository {
    async fn save(&self, event: &WebhookEvent) -> errors::Result<()> {
        let stats_json = event
            .stats()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| errors::Error::invalid(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO webhook_events
                (id, endpoint_id, provider, event_type, payload, headers,
                 signature_valid, processing_status, error_message, retry_count,
                 next_retry_at, stats, received_at, processed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                processing_status = VALUES(processing_status),
                error_message = VALUES(error_message),
                retry_count = VALUES(retry_count),
                next_retry_at = VALUES(next_retry_at),
                stats = VALUES(stats),
                processed_at = VALUES(processed_at)
            "#,
        )
        .bind(event.id().to_string())
        .bind(event.endpoint_id().to_string())
        .bind(event.provider().to_string())
        .bind(event.event_type())
        .bind(event.payload())
        .bind(event.headers())
        .bind(event.signature_valid())
        .bind(event.status().as_str())
        .bind(event.error_message())
        .bind(*event.retry_count() as i32)
        .bind(event.next_retry_at())
        .bind(&stats_json)
        .bind(event.received_at())
        .bind(event.processed_at())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &WebhookEventId,
    ) -> errors::Result<Option<WebhookEvent>> {
        let row: Option<WebhookEventRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, provider, event_type, payload, headers,
                   signature_valid, processing_status, error_message, retry_count,
                   next_retry_at, stats, received_at, processed_at
            FROM webhook_events
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
    ) -> errors::Result<Vec<WebhookEvent>> {
        let rows: Vec<WebhookEventRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, provider, event_type, payload, headers,
                   signature_valid, processing_status, error_message, retry_count,
                   next_retry_at, stats, received_at, processed_at
            FROM webhook_events
            WHERE endpoint_id = ?
            ORDER BY received_at DESC
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

    async fn find_pending_events(
        &self,
        limit: u32,
    ) -> errors::Result<Vec<WebhookEvent>> {
        let rows: Vec<WebhookEventRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, provider, event_type, payload, headers,
                   signature_valid, processing_status, error_message, retry_count,
                   next_retry_at, stats, received_at, processed_at
            FROM webhook_events
            WHERE processing_status = 'pending'
              AND (next_retry_at IS NULL OR next_retry_at <= NOW())
            ORDER BY received_at ASC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn find_by_status(
        &self,
        endpoint_id: &WebhookEndpointId,
        status: ProcessingStatus,
        limit: u32,
    ) -> errors::Result<Vec<WebhookEvent>> {
        let rows: Vec<WebhookEventRow> = sqlx::query_as(
            r#"
            SELECT id, endpoint_id, provider, event_type, payload, headers,
                   signature_valid, processing_status, error_message, retry_count,
                   next_retry_at, stats, received_at, processed_at
            FROM webhook_events
            WHERE endpoint_id = ? AND processing_status = ?
            ORDER BY received_at DESC
            LIMIT ?
            "#,
        )
        .bind(endpoint_id.to_string())
        .bind(status.as_str())
        .bind(limit)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn update_status(
        &self,
        id: &WebhookEventId,
        status: ProcessingStatus,
        error_message: Option<String>,
        stats: Option<ProcessingStats>,
    ) -> errors::Result<()> {
        let stats_json = stats
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| errors::Error::invalid(e.to_string()))?;

        let processed_at = if status.is_terminal() {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE webhook_events
            SET processing_status = ?,
                error_message = ?,
                stats = ?,
                processed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(error_message)
        .bind(&stats_json)
        .bind(processed_at)
        .bind(id.to_string())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        Ok(())
    }

    async fn delete_older_than(
        &self,
        before: DateTime<Utc>,
    ) -> errors::Result<u64> {
        let result =
            sqlx::query("DELETE FROM webhook_events WHERE received_at < ?")
                .bind(before)
                .execute(self.pool.as_ref())
                .await
                .map_err(|e| {
                    errors::Error::internal_server_error(e.to_string())
                })?;

        Ok(result.rows_affected())
    }
}
