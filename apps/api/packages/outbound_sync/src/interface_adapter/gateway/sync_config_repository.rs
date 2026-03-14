//! SQLx implementation of SyncConfigRepository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, MySqlPool};
use std::sync::Arc;

use outbound_sync_domain::{
    DataId, SyncConfig, SyncConfigId, SyncConfigRepository, SyncStatus,
    SyncTarget,
};
use value_object::TenantId;

/// SQLx-based sync config repository
#[derive(Debug)]
pub struct SqlxSyncConfigRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxSyncConfigRepository {
    /// Create a new repository instance
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct SyncConfigRow {
    id: String,
    tenant_id: String,
    data_id: String,
    provider: String,
    target_container: String,
    target_resource: Option<String>,
    target_version: Option<String>,
    status: String,
    status_error: Option<String>,
    last_synced_at: Option<DateTime<Utc>>,
    last_result_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<SyncConfigRow> for SyncConfig {
    type Error = errors::Error;

    fn try_from(row: SyncConfigRow) -> Result<Self, Self::Error> {
        let status = match row.status.as_str() {
            "never_synced" => SyncStatus::NeverSynced,
            "synced" => SyncStatus::Synced,
            "pending" => SyncStatus::Pending,
            "failed" => {
                SyncStatus::Failed(row.status_error.unwrap_or_default())
            }
            _ => SyncStatus::NeverSynced,
        };

        let target = SyncTarget {
            container: row.target_container,
            resource: row.target_resource,
            version: row.target_version,
        };

        Ok(SyncConfig::new(
            SyncConfigId::new(row.id),
            TenantId::new(&row.tenant_id)?,
            DataId::new(row.data_id),
            row.provider,
            target,
            status,
            row.last_synced_at,
            row.last_result_id,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[async_trait]
impl SyncConfigRepository for SqlxSyncConfigRepository {
    #[tracing::instrument(skip(self, config))]
    async fn save(&self, config: &SyncConfig) -> errors::Result<()> {
        let (status_str, status_error) = match config.status() {
            SyncStatus::NeverSynced => ("never_synced", None),
            SyncStatus::Synced => ("synced", None),
            SyncStatus::Pending => ("pending", None),
            SyncStatus::Failed(msg) => ("failed", Some(msg.clone())),
        };

        sqlx::query(
            r#"
            INSERT INTO database_sync_configs (
                id, tenant_id, data_id, provider,
                target_container, target_resource, target_version,
                status, status_error, last_synced_at, last_result_id,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                target_container = VALUES(target_container),
                target_resource = VALUES(target_resource),
                target_version = VALUES(target_version),
                status = VALUES(status),
                status_error = VALUES(status_error),
                last_synced_at = VALUES(last_synced_at),
                last_result_id = VALUES(last_result_id),
                updated_at = VALUES(updated_at)
            "#,
        )
        .bind(config.id().as_str())
        .bind(config.tenant_id().as_str())
        .bind(config.data_id().as_str())
        .bind(config.provider())
        .bind(&config.target().container)
        .bind(&config.target().resource)
        .bind(&config.target().version)
        .bind(status_str)
        .bind(&status_error)
        .bind(config.last_synced_at())
        .bind(config.last_result_id())
        .bind(config.created_at())
        .bind(config.updated_at())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn find_by_id(
        &self,
        id: &SyncConfigId,
    ) -> errors::Result<Option<SyncConfig>> {
        let row: Option<SyncConfigRow> = sqlx::query_as(
            r#"
            SELECT
                id, tenant_id, data_id, provider,
                target_container, target_resource, target_version,
                status, status_error, last_synced_at, last_result_id,
                created_at, updated_at
            FROM database_sync_configs
            WHERE id = ?
            "#,
        )
        .bind(id.as_str())
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(TryInto::try_into).transpose()
    }

    #[tracing::instrument(skip(self))]
    async fn find_by_data_id(
        &self,
        data_id: &DataId,
    ) -> errors::Result<Option<SyncConfig>> {
        let row: Option<SyncConfigRow> = sqlx::query_as(
            r#"
            SELECT
                id, tenant_id, data_id, provider,
                target_container, target_resource, target_version,
                status, status_error, last_synced_at, last_result_id,
                created_at, updated_at
            FROM database_sync_configs
            WHERE data_id = ?
            "#,
        )
        .bind(data_id.as_str())
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(TryInto::try_into).transpose()
    }

    #[tracing::instrument(skip(self))]
    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &TenantId,
        provider: &str,
    ) -> errors::Result<Vec<SyncConfig>> {
        let rows: Vec<SyncConfigRow> = sqlx::query_as(
            r#"
            SELECT
                id, tenant_id, data_id, provider,
                target_container, target_resource, target_version,
                status, status_error, last_synced_at, last_result_id,
                created_at, updated_at
            FROM database_sync_configs
            WHERE tenant_id = ? AND provider = ?
            ORDER BY updated_at DESC
            "#,
        )
        .bind(tenant_id.as_str())
        .bind(provider)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
    }

    #[tracing::instrument(skip(self))]
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<SyncConfig>> {
        let rows: Vec<SyncConfigRow> = sqlx::query_as(
            r#"
            SELECT
                id, tenant_id, data_id, provider,
                target_container, target_resource, target_version,
                status, status_error, last_synced_at, last_result_id,
                created_at, updated_at
            FROM database_sync_configs
            WHERE tenant_id = ?
            ORDER BY updated_at DESC
            "#,
        )
        .bind(tenant_id.as_str())
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
    }

    #[tracing::instrument(skip(self))]
    async fn delete(&self, id: &SyncConfigId) -> errors::Result<()> {
        sqlx::query("DELETE FROM database_sync_configs WHERE id = ?")
            .bind(id.as_str())
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| {
                errors::Error::internal_server_error(e.to_string())
            })?;

        Ok(())
    }
}
