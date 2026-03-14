//! SQLx implementation of StoredOAuthTokenRepository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use std::sync::Arc;

use integration_domain::{
    OAuthProvider, StoredOAuthToken, StoredOAuthTokenRepository,
};
use value_object::TenantId;

/// SQLx implementation of StoredOAuthTokenRepository.
#[derive(Debug, Clone)]
pub struct SqlxOAuthTokenRepository {
    pool: Arc<MySqlPool>,
}

impl SqlxOAuthTokenRepository {
    /// Create a new SqlxOAuthTokenRepository.
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OAuthTokenRow {
    id: String,
    tenant_id: String,
    provider: String,
    access_token: String,
    refresh_token: Option<String>,
    token_type: String,
    scope: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    external_account_id: Option<String>,
    external_account_name: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<OAuthTokenRow> for StoredOAuthToken {
    type Error = errors::Error;

    fn try_from(row: OAuthTokenRow) -> Result<Self, Self::Error> {
        let provider: OAuthProvider = row
            .provider
            .parse()
            .map_err(|_| errors::Error::invalid("Invalid provider"))?;

        let tenant_id: TenantId =
            row.tenant_id.parse().map_err(|e: errors::ParseIdError| {
                errors::Error::invalid(e.to_string())
            })?;

        let scopes: Vec<String> = row
            .scope
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default();

        Ok(StoredOAuthToken {
            id: row.id,
            tenant_id,
            provider,
            access_token: row.access_token,
            refresh_token: row.refresh_token,
            token_type: row.token_type,
            expires_at: row.expires_at,
            scopes,
            external_account_id: row.external_account_id,
            external_account_name: row.external_account_name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[async_trait]
impl StoredOAuthTokenRepository for SqlxOAuthTokenRepository {
    async fn save(&self, token: &StoredOAuthToken) -> errors::Result<()> {
        let tenant_id = token.tenant_id.to_string();
        let provider = token.provider.to_string();
        let scope = if token.scopes.is_empty() {
            None
        } else {
            Some(token.scopes.join(" "))
        };

        sqlx::query(
            r#"
            INSERT INTO oauth_tokens (
                id, tenant_id, provider, access_token, refresh_token,
                token_type, scope, expires_at, external_account_id,
                external_account_name, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                access_token = VALUES(access_token),
                refresh_token = VALUES(refresh_token),
                token_type = VALUES(token_type),
                scope = VALUES(scope),
                expires_at = VALUES(expires_at),
                external_account_id = VALUES(external_account_id),
                external_account_name = VALUES(external_account_name),
                updated_at = VALUES(updated_at)
            "#,
        )
        .bind(&token.id)
        .bind(&tenant_id)
        .bind(&provider)
        .bind(&token.access_token)
        .bind(&token.refresh_token)
        .bind(&token.token_type)
        .bind(&scope)
        .bind(token.expires_at)
        .bind(&token.external_account_id)
        .bind(&token.external_account_name)
        .bind(token.created_at)
        .bind(token.updated_at)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        Ok(())
    }

    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<Option<StoredOAuthToken>> {
        let row: Option<OAuthTokenRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, provider, access_token, refresh_token,
                   token_type, scope, expires_at, external_account_id,
                   external_account_name, created_at, updated_at
            FROM oauth_tokens
            WHERE tenant_id = ? AND provider = ?
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(provider.to_string())
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        row.map(StoredOAuthToken::try_from).transpose()
    }

    async fn delete(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<()> {
        sqlx::query(
            "DELETE FROM oauth_tokens WHERE tenant_id = ? AND provider = ?",
        )
        .bind(tenant_id.to_string())
        .bind(provider.to_string())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_conversion() {
        let row = OAuthTokenRow {
            id: "token_123".to_string(),
            tenant_id: "tn_01hjryxysgey07h5jz5wagqj0m".to_string(),
            provider: "github".to_string(),
            access_token: "gho_xxxx".to_string(),
            refresh_token: Some("ghr_xxxx".to_string()),
            token_type: "Bearer".to_string(),
            scope: Some("repo user".to_string()),
            expires_at: None,
            external_account_id: Some("user123".to_string()),
            external_account_name: Some("Test User".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let token = StoredOAuthToken::try_from(row).unwrap();
        assert_eq!(token.provider, OAuthProvider::Github);
        assert_eq!(token.access_token, "gho_xxxx");
        assert_eq!(token.scopes, vec!["repo", "user"]);
    }
}
