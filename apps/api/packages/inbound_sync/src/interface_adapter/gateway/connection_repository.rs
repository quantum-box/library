//! SQLx implementation of ConnectionRepository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;

use integration_domain::{
    Connection, ConnectionId, ConnectionRepository, ConnectionStatus,
    IntegrationId, OAuthProvider,
};
use value_object::TenantId;

/// SQLx implementation of ConnectionRepository.
#[derive(Debug, Clone)]
pub struct SqlxConnectionRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxConnectionRepository {
    /// Create a new SqlxConnectionRepository.
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ConnectionRow {
    id: String,
    tenant_id: String,
    integration_id: String,
    provider: String,
    status: String,
    external_account_id: Option<String>,
    external_account_name: Option<String>,
    token_expires_at: Option<DateTime<Utc>>,
    last_synced_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    metadata: Option<serde_json::Value>,
}

impl TryFrom<ConnectionRow> for Connection {
    type Error = errors::Error;

    fn try_from(row: ConnectionRow) -> Result<Self, Self::Error> {
        let tenant_id: TenantId =
            row.tenant_id.parse().map_err(|e: errors::ParseIdError| {
                errors::Error::invalid(e.to_string())
            })?;

        let provider: OAuthProvider = row
            .provider
            .parse()
            .map_err(|_| errors::Error::invalid("Invalid provider"))?;

        let status: ConnectionStatus = row
            .status
            .parse()
            .map_err(|_| errors::Error::invalid("Invalid status"))?;

        let metadata: HashMap<String, serde_json::Value> = row
            .metadata
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        Ok(Connection::new(
            ConnectionId::new(row.id),
            tenant_id,
            IntegrationId::new(row.integration_id),
            provider,
            status,
            row.external_account_id,
            row.external_account_name,
            row.token_expires_at,
            row.last_synced_at,
            row.error_message,
            row.created_at,
            row.updated_at,
            metadata,
        ))
    }
}

#[async_trait]
impl ConnectionRepository for SqlxConnectionRepository {
    async fn save(&self, connection: &Connection) -> errors::Result<()> {
        let metadata_json = serde_json::to_value(connection.metadata())
            .map_err(|e| {
                errors::Error::internal_server_error(e.to_string())
            })?;

        sqlx::query(
            r#"
            INSERT INTO integration_connections (
                id, tenant_id, integration_id, provider, status,
                external_account_id, external_account_name,
                token_expires_at, last_synced_at, error_message,
                metadata, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                status = VALUES(status),
                external_account_id = VALUES(external_account_id),
                external_account_name = VALUES(external_account_name),
                token_expires_at = VALUES(token_expires_at),
                last_synced_at = VALUES(last_synced_at),
                error_message = VALUES(error_message),
                metadata = VALUES(metadata),
                updated_at = VALUES(updated_at)
            "#,
        )
        .bind(connection.id().as_str())
        .bind(connection.tenant_id().to_string())
        .bind(connection.integration_id().as_str())
        .bind(connection.provider().to_string())
        .bind(connection.status().to_string())
        .bind(connection.external_account_id())
        .bind(connection.external_account_name())
        .bind(connection.token_expires_at())
        .bind(connection.last_synced_at())
        .bind(connection.error_message())
        .bind(&metadata_json)
        .bind(connection.connected_at())
        .bind(connection.updated_at())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &ConnectionId,
    ) -> errors::Result<Option<Connection>> {
        let row: Option<ConnectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, integration_id, provider,
                   status, external_account_id,
                   external_account_name, token_expires_at,
                   last_synced_at, error_message,
                   created_at, updated_at, metadata
            FROM integration_connections
            WHERE id = ?
            "#,
        )
        .bind(id.as_str())
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(Connection::try_from).transpose()
    }

    async fn find_by_tenant_and_integration(
        &self,
        tenant_id: &TenantId,
        integration_id: &IntegrationId,
    ) -> errors::Result<Option<Connection>> {
        let row: Option<ConnectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, integration_id, provider,
                   status, external_account_id,
                   external_account_name, token_expires_at,
                   last_synced_at, error_message,
                   created_at, updated_at, metadata
            FROM integration_connections
            WHERE tenant_id = ? AND integration_id = ?
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(integration_id.as_str())
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(Connection::try_from).transpose()
    }

    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Connection>> {
        let rows: Vec<ConnectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, integration_id, provider,
                   status, external_account_id,
                   external_account_name, token_expires_at,
                   last_synced_at, error_message,
                   created_at, updated_at, metadata
            FROM integration_connections
            WHERE tenant_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id.to_string())
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter()
            .map(Connection::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn find_active_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Connection>> {
        let rows: Vec<ConnectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, integration_id, provider,
                   status, external_account_id,
                   external_account_name, token_expires_at,
                   last_synced_at, error_message,
                   created_at, updated_at, metadata
            FROM integration_connections
            WHERE tenant_id = ? AND status = 'active'
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id.to_string())
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        rows.into_iter()
            .map(Connection::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<Option<Connection>> {
        let row: Option<ConnectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, integration_id, provider,
                   status, external_account_id,
                   external_account_name, token_expires_at,
                   last_synced_at, error_message,
                   created_at, updated_at, metadata
            FROM integration_connections
            WHERE tenant_id = ? AND provider = ?
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(provider.to_string())
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(Connection::try_from).transpose()
    }

    async fn find_by_provider_and_external_account_id(
        &self,
        provider: OAuthProvider,
        external_account_id: &str,
    ) -> errors::Result<Option<Connection>> {
        let row: Option<ConnectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, integration_id, provider,
                   status, external_account_id,
                   external_account_name, token_expires_at,
                   last_synced_at, error_message,
                   created_at, updated_at, metadata
            FROM integration_connections
            WHERE provider = ? AND external_account_id = ?
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(provider.to_string())
        .bind(external_account_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(Connection::try_from).transpose()
    }

    async fn delete(&self, id: &ConnectionId) -> errors::Result<()> {
        sqlx::query("DELETE FROM integration_connections WHERE id = ?")
            .bind(id.as_str())
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| {
                errors::Error::internal_server_error(e.to_string())
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_conversion() {
        let row = ConnectionRow {
            id: "con_123".to_string(),
            tenant_id: "tn_01hjryxysgey07h5jz5wagqj0m".to_string(),
            integration_id: "int_github".to_string(),
            provider: "github".to_string(),
            status: "active".to_string(),
            external_account_id: Some("user123".to_string()),
            external_account_name: Some("Test User".to_string()),
            token_expires_at: None,
            last_synced_at: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: Some(
                serde_json::json!({"webhook_url": "https://example.com"}),
            ),
        };

        let connection = Connection::try_from(row).unwrap();
        assert_eq!(connection.provider(), OAuthProvider::Github);
        assert_eq!(connection.status(), ConnectionStatus::Active);
        assert_eq!(
            connection.get_metadata_str("webhook_url"),
            Some("https://example.com")
        );
    }
}
