//! SQLx implementation of WebhookEndpointRepository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use std::sync::Arc;

use inbound_sync_domain::{
    EndpointStatus, PropertyMapping, Provider, ProviderConfig,
    WebhookEndpoint, WebhookEndpointId, WebhookEndpointRepository,
};
use value_object::TenantId;

/// SQLx implementation of WebhookEndpointRepository.
#[derive(Debug, Clone)]
pub struct SqlxWebhookEndpointRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxWebhookEndpointRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct WebhookEndpointRow {
    id: String,
    tenant_id: String,
    repository_id: Option<String>,
    name: String,
    provider: String,
    config: serde_json::Value,
    events: serde_json::Value,
    mapping_config: Option<serde_json::Value>,
    secret_hash: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<WebhookEndpointRow> for WebhookEndpoint {
    type Error = errors::Error;

    fn try_from(row: WebhookEndpointRow) -> Result<Self, Self::Error> {
        let provider: Provider = row
            .provider
            .parse()
            .map_err(|_| errors::Error::invalid("Invalid provider"))?;

        let config: ProviderConfig = serde_json::from_value(row.config)
            .map_err(|e| {
                errors::Error::invalid(format!("Invalid config: {e}"))
            })?;

        let events: Vec<String> = serde_json::from_value(row.events)
            .map_err(|e| {
                errors::Error::invalid(format!("Invalid events: {e}"))
            })?;

        let mapping: Option<PropertyMapping> = row
            .mapping_config
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| {
                errors::Error::invalid(format!("Invalid mapping: {e}"))
            })?;

        let status: EndpointStatus = row.status.parse()?;

        let tenant_id: TenantId =
            row.tenant_id.parse().map_err(|e: errors::ParseIdError| {
                errors::Error::invalid(e.to_string())
            })?;

        let endpoint = WebhookEndpoint::new(
            WebhookEndpointId::from(row.id),
            tenant_id,
            row.repository_id,
            row.name,
            provider,
            config,
            events,
            mapping,
            row.secret_hash,
            status,
            row.created_at,
            row.updated_at,
        );

        Ok(endpoint)
    }
}

#[async_trait]
impl WebhookEndpointRepository for SqlxWebhookEndpointRepository {
    async fn save(&self, endpoint: &WebhookEndpoint) -> errors::Result<()> {
        let config_json = serde_json::to_value(endpoint.config())
            .map_err(|e| errors::Error::invalid(e.to_string()))?;

        let events_json = serde_json::to_value(endpoint.events())
            .map_err(|e| errors::Error::invalid(e.to_string()))?;

        let mapping_json = endpoint
            .mapping()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| errors::Error::invalid(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO webhook_endpoints 
                (id, tenant_id, repository_id, name, provider, config, events, 
                 mapping_config, secret_hash, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                repository_id = VALUES(repository_id),
                name = VALUES(name),
                config = VALUES(config),
                events = VALUES(events),
                mapping_config = VALUES(mapping_config),
                status = VALUES(status),
                updated_at = VALUES(updated_at)
            "#,
        )
        .bind(endpoint.id().to_string())
        .bind(endpoint.tenant_id().to_string())
        .bind(endpoint.repository_id())
        .bind(endpoint.name())
        .bind(endpoint.provider().to_string())
        .bind(&config_json)
        .bind(&events_json)
        .bind(&mapping_json)
        .bind(endpoint.secret_hash())
        .bind(endpoint.status().as_str())
        .bind(endpoint.created_at())
        .bind(endpoint.updated_at())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &WebhookEndpointId,
    ) -> errors::Result<Option<WebhookEndpoint>> {
        let row: Option<WebhookEndpointRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, repository_id, name, provider, config, events,
                   mapping_config, secret_hash, status, created_at, updated_at
            FROM webhook_endpoints
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<WebhookEndpoint>> {
        let rows: Vec<WebhookEndpointRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, repository_id, name, provider, config, events,
                   mapping_config, secret_hash, status, created_at, updated_at
            FROM webhook_endpoints
            WHERE tenant_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id.to_string())
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &TenantId,
        provider: Provider,
    ) -> errors::Result<Vec<WebhookEndpoint>> {
        let rows: Vec<WebhookEndpointRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, repository_id, name, provider, config, events,
                   mapping_config, secret_hash, status, created_at, updated_at
            FROM webhook_endpoints
            WHERE tenant_id = ? AND provider = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(provider.to_string())
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn delete(&self, id: &WebhookEndpointId) -> errors::Result<()> {
        sqlx::query("DELETE FROM webhook_endpoints WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| {
                errors::Error::internal_server_error(e.to_string())
            })?;

        Ok(())
    }
}
