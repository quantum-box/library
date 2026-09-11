//! Persistence for external ChangeSets, deliveries, and dispatch capabilities.

use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use integration_domain::{
    ExternalChangeType, ExternalSyncBindingId, InboundChangeSet,
    InboundChangeSetId, InboundChangeSetRepository, InboundChangeSetStatus,
    LibraryDataId, OutboundDelivery, OutboundDeliveryId,
    OutboundDeliveryRepository, OutboundDeliveryStatus,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use value_object::TenantId;

use super::external_sync_repository::persistence_error;
use crate::usecase::WebhookDispatcher;

fn sha256(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[derive(Debug, Clone)]
pub struct SqlxExternalSyncLifecycleRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxExternalSyncLifecycleRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct InboundChangeSetRow {
    id: String,
    tenant_id: String,
    binding_id: String,
    data_id: Option<String>,
    external_object_id: String,
    external_revision: String,
    base_external_revision: Option<String>,
    change_type: String,
    payload: serde_json::Value,
    idempotency_key: String,
    status: String,
    decision_note: Option<String>,
    created_at: DateTime<Utc>,
    decided_at: Option<DateTime<Utc>>,
}

impl TryFrom<InboundChangeSetRow> for InboundChangeSet {
    type Error = errors::Error;

    fn try_from(row: InboundChangeSetRow) -> Result<Self, Self::Error> {
        InboundChangeSet::new(
            InboundChangeSetId::parse(row.id)?,
            row.tenant_id.parse().map_err(
                |error: errors::ParseIdError| {
                    errors::Error::invalid(error.to_string())
                },
            )?,
            ExternalSyncBindingId::parse(row.binding_id)?,
            row.data_id.map(LibraryDataId::parse).transpose()?,
            row.external_object_id,
            row.external_revision,
            row.base_external_revision,
            ExternalChangeType::from_str(&row.change_type)?,
            row.payload,
            row.idempotency_key,
            InboundChangeSetStatus::from_str(&row.status)?,
            row.decision_note,
            row.created_at,
            row.decided_at,
        )
    }
}

const CHANGE_SET_COLUMNS: &str = r#"
    changeset.id, changeset.tenant_id, changeset.binding_id,
    changeset.data_id, changeset.external_object_id,
    changeset.external_revision, changeset.base_external_revision,
    changeset.change_type, changeset.payload, changeset.idempotency_key,
    changeset.status, changeset.decision_note, changeset.created_at,
    changeset.decided_at
"#;

#[async_trait]
impl InboundChangeSetRepository for SqlxExternalSyncLifecycleRepository {
    async fn save(
        &self,
        change_set: &InboundChangeSet,
    ) -> errors::Result<()> {
        let mut transaction =
            self.pool.begin().await.map_err(persistence_error)?;
        let existing_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM inbound_change_sets WHERE binding_id = ? AND idempotency_key = ? FOR UPDATE",
        )
        .bind(change_set.binding_id().as_str())
        .bind(change_set.idempotency_key())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(persistence_error)?;

        if existing_id
            .as_deref()
            .is_some_and(|id| id != change_set.id().as_str())
        {
            transaction.commit().await.map_err(persistence_error)?;
            return Ok(());
        }

        sqlx::query(
            r#"
            INSERT INTO inbound_change_sets (
                id, tenant_id, binding_id, data_id, external_object_id,
                external_object_id_hash, external_revision,
                base_external_revision, change_type, payload,
                idempotency_key, status, decision_note, created_at,
                decided_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                status = VALUES(status),
                decision_note = VALUES(decision_note),
                decided_at = VALUES(decided_at)
            "#,
        )
        .bind(change_set.id().as_str())
        .bind(change_set.tenant_id().to_string())
        .bind(change_set.binding_id().as_str())
        .bind(change_set.data_id().map(LibraryDataId::as_str))
        .bind(change_set.external_object_id())
        .bind(sha256(change_set.external_object_id()))
        .bind(change_set.external_revision())
        .bind(change_set.base_external_revision())
        .bind(change_set.change_type().as_str())
        .bind(change_set.payload())
        .bind(change_set.idempotency_key())
        .bind(change_set.status().as_str())
        .bind(change_set.decision_note())
        .bind(change_set.created_at())
        .bind(change_set.decided_at())
        .execute(&mut *transaction)
        .await
        .map_err(persistence_error)?;
        transaction.commit().await.map_err(persistence_error)?;
        Ok(())
    }

    async fn find_by_id(
        &self,
        tenant_id: &TenantId,
        id: &InboundChangeSetId,
    ) -> errors::Result<Option<InboundChangeSet>> {
        let query = format!(
            "SELECT {CHANGE_SET_COLUMNS} FROM inbound_change_sets AS changeset WHERE changeset.tenant_id = ? AND changeset.id = ?"
        );
        let row: Option<InboundChangeSetRow> = sqlx::query_as(&query)
            .bind(tenant_id.to_string())
            .bind(id.as_str())
            .fetch_optional(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_binding(
        &self,
        tenant_id: &TenantId,
        binding_id: &ExternalSyncBindingId,
        status: Option<InboundChangeSetStatus>,
        limit: u32,
    ) -> errors::Result<Vec<InboundChangeSet>> {
        let (query, status_value) = if let Some(status) = status {
            (
                format!("SELECT {CHANGE_SET_COLUMNS} FROM inbound_change_sets AS changeset WHERE changeset.tenant_id = ? AND changeset.binding_id = ? AND changeset.status = ? ORDER BY changeset.created_at DESC, changeset.id DESC LIMIT ?"),
                Some(status.as_str()),
            )
        } else {
            (
                format!("SELECT {CHANGE_SET_COLUMNS} FROM inbound_change_sets AS changeset WHERE changeset.tenant_id = ? AND changeset.binding_id = ? ORDER BY changeset.created_at DESC, changeset.id DESC LIMIT ?"),
                None,
            )
        };
        let mut statement =
            sqlx::query_as::<_, InboundChangeSetRow>(&query)
                .bind(tenant_id.to_string())
                .bind(binding_id.as_str());
        if let Some(status) = status_value {
            statement = statement.bind(status);
        }
        let rows = statement
            .bind(limit.min(200))
            .fetch_all(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OutboundDeliveryRow {
    id: String,
    tenant_id: String,
    binding_id: String,
    data_id: String,
    external_object_id: String,
    library_revision: String,
    base_external_revision: Option<String>,
    payload: serde_json::Value,
    idempotency_key: String,
    status: String,
    attempt_count: u32,
    next_attempt_at: DateTime<Utc>,
    last_error_category: Option<String>,
    remote_revision: Option<String>,
    delivery_url: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<OutboundDeliveryRow> for OutboundDelivery {
    type Error = errors::Error;

    fn try_from(row: OutboundDeliveryRow) -> Result<Self, Self::Error> {
        OutboundDelivery::new(
            OutboundDeliveryId::parse(row.id)?,
            row.tenant_id.parse().map_err(
                |error: errors::ParseIdError| {
                    errors::Error::invalid(error.to_string())
                },
            )?,
            ExternalSyncBindingId::parse(row.binding_id)?,
            LibraryDataId::parse(row.data_id)?,
            row.external_object_id,
            row.library_revision,
            row.base_external_revision,
            row.payload,
            row.idempotency_key,
            OutboundDeliveryStatus::from_str(&row.status)?,
            row.attempt_count,
            row.next_attempt_at,
            row.last_error_category,
            row.remote_revision,
            row.delivery_url,
            row.created_at,
            row.updated_at,
        )
    }
}

const DELIVERY_COLUMNS: &str = r#"
    delivery.id, delivery.tenant_id, delivery.binding_id,
    delivery.data_id, delivery.external_object_id,
    delivery.library_revision, delivery.base_external_revision,
    delivery.payload, delivery.idempotency_key, delivery.status,
    delivery.attempt_count, delivery.next_attempt_at,
    delivery.last_error_category, delivery.remote_revision,
    delivery.delivery_url, delivery.created_at, delivery.updated_at
"#;

#[async_trait]
impl OutboundDeliveryRepository for SqlxExternalSyncLifecycleRepository {
    async fn save(
        &self,
        delivery: &OutboundDelivery,
    ) -> errors::Result<()> {
        let mut transaction =
            self.pool.begin().await.map_err(persistence_error)?;
        let existing_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM outbound_deliveries WHERE binding_id = ? AND idempotency_key = ? FOR UPDATE",
        )
        .bind(delivery.binding_id().as_str())
        .bind(delivery.idempotency_key())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(persistence_error)?;
        if existing_id
            .as_deref()
            .is_some_and(|id| id != delivery.id().as_str())
        {
            transaction.commit().await.map_err(persistence_error)?;
            return Ok(());
        }

        sqlx::query(
            r#"
            INSERT INTO outbound_deliveries (
                id, tenant_id, binding_id, data_id, external_object_id,
                external_object_id_hash, library_revision,
                base_external_revision, payload, idempotency_key, status,
                attempt_count, next_attempt_at, last_error_category,
                remote_revision, delivery_url, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                status = VALUES(status),
                attempt_count = VALUES(attempt_count),
                next_attempt_at = VALUES(next_attempt_at),
                last_error_category = VALUES(last_error_category),
                remote_revision = VALUES(remote_revision),
                delivery_url = VALUES(delivery_url),
                updated_at = VALUES(updated_at)
            "#,
        )
        .bind(delivery.id().as_str())
        .bind(delivery.tenant_id().to_string())
        .bind(delivery.binding_id().as_str())
        .bind(delivery.data_id().as_str())
        .bind(delivery.external_object_id())
        .bind(sha256(delivery.external_object_id()))
        .bind(delivery.library_revision())
        .bind(delivery.base_external_revision())
        .bind(delivery.payload())
        .bind(delivery.idempotency_key())
        .bind(delivery.status().as_str())
        .bind(delivery.attempt_count())
        .bind(delivery.next_attempt_at())
        .bind(delivery.last_error_category())
        .bind(delivery.remote_revision())
        .bind(delivery.delivery_url())
        .bind(delivery.created_at())
        .bind(delivery.updated_at())
        .execute(&mut *transaction)
        .await
        .map_err(persistence_error)?;
        transaction.commit().await.map_err(persistence_error)?;
        Ok(())
    }

    async fn find_by_id(
        &self,
        tenant_id: &TenantId,
        id: &OutboundDeliveryId,
    ) -> errors::Result<Option<OutboundDelivery>> {
        let query = format!(
            "SELECT {DELIVERY_COLUMNS} FROM outbound_deliveries AS delivery WHERE delivery.tenant_id = ? AND delivery.id = ?"
        );
        let row: Option<OutboundDeliveryRow> = sqlx::query_as(&query)
            .bind(tenant_id.to_string())
            .bind(id.as_str())
            .fetch_optional(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_binding(
        &self,
        tenant_id: &TenantId,
        binding_id: &ExternalSyncBindingId,
        status: Option<OutboundDeliveryStatus>,
        limit: u32,
    ) -> errors::Result<Vec<OutboundDelivery>> {
        let (query, status_value) = if let Some(status) = status {
            (
                format!("SELECT {DELIVERY_COLUMNS} FROM outbound_deliveries AS delivery WHERE delivery.tenant_id = ? AND delivery.binding_id = ? AND delivery.status = ? ORDER BY delivery.created_at DESC, delivery.id DESC LIMIT ?"),
                Some(status.as_str()),
            )
        } else {
            (
                format!("SELECT {DELIVERY_COLUMNS} FROM outbound_deliveries AS delivery WHERE delivery.tenant_id = ? AND delivery.binding_id = ? ORDER BY delivery.created_at DESC, delivery.id DESC LIMIT ?"),
                None,
            )
        };
        let mut statement =
            sqlx::query_as::<_, OutboundDeliveryRow>(&query)
                .bind(tenant_id.to_string())
                .bind(binding_id.as_str());
        if let Some(status) = status_value {
            statement = statement.bind(status);
        }
        let rows = statement
            .bind(limit.min(200))
            .fetch_all(self.pool.as_ref())
            .await
            .map_err(persistence_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[derive(Debug, Clone)]
pub struct ExternalSyncDispatchJob {
    pub event_id: inbound_sync_domain::WebhookEventId,
    pub capability: String,
}

#[derive(Debug, Clone)]
pub struct SqlxExternalSyncDispatchRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxExternalSyncDispatchRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        event_id: &inbound_sync_domain::WebhookEventId,
    ) -> errors::Result<ExternalSyncDispatchJob> {
        let mut bytes = [0_u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let capability = hex::encode(bytes);
        sqlx::query(
            r#"
            INSERT INTO external_sync_dispatch_jobs (
                event_id, capability_hash, status
            ) VALUES (?, ?, 'pending')
            ON DUPLICATE KEY UPDATE
                capability_hash = VALUES(capability_hash),
                status = IF(status = 'completed', status, 'pending'),
                next_attempt_at = IF(
                    status = 'completed', next_attempt_at, NOW(6)
                ),
                lease_expires_at = IF(
                    status = 'completed', lease_expires_at, NULL
                )
            "#,
        )
        .bind(event_id.to_string())
        .bind(sha256(&capability))
        .execute(self.pool.as_ref())
        .await
        .map_err(persistence_error)?;
        Ok(ExternalSyncDispatchJob {
            event_id: event_id.clone(),
            capability,
        })
    }

    pub async fn claim(
        &self,
        event_id: &inbound_sync_domain::WebhookEventId,
        capability: &str,
    ) -> errors::Result<bool> {
        if capability.len() != 64
            || !capability.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Ok(false);
        }
        let result = sqlx::query(
            r#"
            UPDATE external_sync_dispatch_jobs
            SET status = 'processing', attempt_count = attempt_count + 1,
                lease_expires_at = DATE_ADD(NOW(6), INTERVAL 2 MINUTE),
                last_error_category = NULL
            WHERE event_id = ? AND capability_hash = ?
              AND next_attempt_at <= NOW(6)
              AND (
                status = 'pending'
                OR (status = 'processing' AND lease_expires_at <= NOW(6))
              )
            "#,
        )
        .bind(event_id.to_string())
        .bind(sha256(capability))
        .execute(self.pool.as_ref())
        .await
        .map_err(persistence_error)?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn validate(
        &self,
        event_id: &inbound_sync_domain::WebhookEventId,
        capability: &str,
    ) -> errors::Result<bool> {
        if capability.len() != 64
            || !capability.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Ok(false);
        }
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM external_sync_dispatch_jobs WHERE event_id = ? AND capability_hash = ?)",
        )
        .bind(event_id.to_string())
        .bind(sha256(capability))
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(persistence_error)?;
        Ok(exists)
    }

    pub async fn completed(
        &self,
        event_id: &inbound_sync_domain::WebhookEventId,
        capability: &str,
    ) -> errors::Result<bool> {
        if !self.validate(event_id, capability).await? {
            return Ok(false);
        }
        let completed: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM external_sync_dispatch_jobs WHERE event_id = ? AND capability_hash = ? AND status = 'completed')",
        )
        .bind(event_id.to_string())
        .bind(sha256(capability))
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(persistence_error)?;
        Ok(completed)
    }

    pub async fn complete(
        &self,
        event_id: &inbound_sync_domain::WebhookEventId,
    ) -> errors::Result<()> {
        sqlx::query(
            "UPDATE external_sync_dispatch_jobs SET status = 'completed', lease_expires_at = NULL WHERE event_id = ? AND status = 'processing'",
        )
        .bind(event_id.to_string())
        .execute(self.pool.as_ref())
        .await
        .map_err(persistence_error)?;
        Ok(())
    }

    pub async fn retry(
        &self,
        event_id: &inbound_sync_domain::WebhookEventId,
        error_category: &str,
    ) -> errors::Result<()> {
        sqlx::query(
            r#"
            UPDATE external_sync_dispatch_jobs
            SET status = 'pending', lease_expires_at = NULL,
                next_attempt_at = DATE_ADD(
                    NOW(6),
                    INTERVAL LEAST(300, POW(2, LEAST(attempt_count, 8))) SECOND
                ),
                last_error_category = ?
            WHERE event_id = ? AND status = 'processing'
            "#,
        )
        .bind(error_category)
        .bind(event_id.to_string())
        .execute(self.pool.as_ref())
        .await
        .map_err(persistence_error)?;
        Ok(())
    }
}

/// Persists a one-time capability before asking the edge Durable Object to
/// store and schedule it. A failed edge enqueue is returned to the webhook
/// sender so the provider redelivers; the database row remains safe to retry.
#[derive(Debug, Clone)]
pub struct HttpDurableWebhookDispatcher {
    jobs: Arc<SqlxExternalSyncDispatchRepository>,
    client: reqwest::Client,
    dispatcher_url: String,
    callback_url: String,
}

impl HttpDurableWebhookDispatcher {
    pub fn new(
        jobs: Arc<SqlxExternalSyncDispatchRepository>,
        dispatcher_url: impl Into<String>,
        callback_url: impl Into<String>,
    ) -> errors::Result<Self> {
        let dispatcher_url = dispatcher_url.into();
        let callback_url = callback_url.into();
        for (name, url) in [
            ("dispatcher_url", &dispatcher_url),
            ("callback_url", &callback_url),
        ] {
            let parsed = reqwest::Url::parse(url).map_err(|error| {
                errors::Error::invalid(format!("invalid {name}: {error}"))
            })?;
            if parsed.scheme() != "https"
                && parsed.host_str() != Some("127.0.0.1")
            {
                return Err(errors::Error::invalid(format!(
                    "{name} must use HTTPS"
                )));
            }
        }
        Ok(Self {
            jobs,
            client: reqwest::Client::new(),
            dispatcher_url,
            callback_url: callback_url.trim_end_matches('/').into(),
        })
    }
}

#[async_trait]
impl WebhookDispatcher for HttpDurableWebhookDispatcher {
    async fn dispatch(
        &self,
        event_id: &inbound_sync_domain::WebhookEventId,
    ) -> errors::Result<()> {
        let job = self.jobs.create(event_id).await?;
        let response = self
            .client
            .post(&self.dispatcher_url)
            .timeout(std::time::Duration::from_secs(10))
            .json(&serde_json::json!({
                "event_id": job.event_id.to_string(),
                "capability": job.capability,
                "callback_url": self.callback_url,
            }))
            .send()
            .await
            .map_err(|error| {
                errors::Error::service_unavailable(format!(
                    "external sync dispatcher unavailable: {error}"
                ))
            })?;
        if !response.status().is_success() {
            return Err(errors::Error::service_unavailable(format!(
                "external sync dispatcher rejected enqueue with HTTP {}",
                response.status()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_hash_is_not_the_capability() {
        let capability = "a".repeat(64);
        assert_ne!(sha256(&capability), capability);
        assert_eq!(sha256(&capability).len(), 64);
    }
}
