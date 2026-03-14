//! Inbound Sync SDK - Application interface for webhook synchronization.
//!
//! This module provides the main application interface for the inbound sync system,
//! including OAuth token management integration with AuthApp.
//!
//! # Architecture
//!
//! ```text
//! WebhookEvent ─▶ EventProcessor ─▶ OAuthTokenProvider ─▶ AuthApp
//!                      │                    │
//!                      │                    ▼
//!                      │              OAuth Token
//!                      │                    │
//!                      ▼                    ▼
//!               Provider API ◀───────── Access Token
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use inbound_sync::sdk::{InboundSyncApp, OAuthTokenProvider};
//!
//! // Get OAuth token for a provider
//! let token = oauth_provider
//!     .get_access_token(&tenant_id, "square")
//!     .await?;
//!
//! // Use token for API calls
//! let client = SquareApiClient::new(token.access_token);
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use value_object::TenantId;

/// OAuth token information for provider API calls.
#[derive(Debug, Clone)]
pub struct ProviderToken {
    /// The provider name (e.g., "square", "github", "hubspot")
    pub provider: String,
    /// Provider-specific user/merchant ID
    pub provider_user_id: String,
    /// Access token for API calls
    pub access_token: String,
    /// Refresh token (if available)
    pub refresh_token: Option<String>,
    /// Token expiration time
    pub expires_at: DateTime<Utc>,
}

impl ProviderToken {
    /// Check if the token is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }

    /// Check if the token will expire within the given duration.
    pub fn expires_within(&self, duration: chrono::Duration) -> bool {
        self.expires_at <= Utc::now() + duration
    }
}

/// Trait for providing OAuth tokens for external providers.
///
/// This trait abstracts the OAuth token retrieval mechanism,
/// allowing EventProcessors to get valid access tokens for API calls
/// without direct dependency on AuthApp.
///
/// # Implementation
///
/// The primary implementation wraps AuthApp and handles:
/// - Token retrieval by tenant and provider
/// - Automatic token refresh when expired
/// - Error handling for missing or invalid tokens
#[async_trait]
pub trait OAuthTokenProvider: Send + Sync + std::fmt::Debug {
    /// Get an OAuth token for the specified tenant and provider.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant (operator) ID
    /// * `provider` - The provider name (e.g., "square", "github")
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(token))` if a valid token exists,
    /// `Ok(None)` if no token is configured,
    /// or an error if token retrieval/refresh fails.
    async fn get_token(
        &self,
        tenant_id: &TenantId,
        provider: &str,
    ) -> errors::Result<Option<ProviderToken>>;

    /// Check if a provider is connected (has valid OAuth credentials).
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant (operator) ID
    /// * `provider` - The provider name
    ///
    /// # Returns
    ///
    /// Returns `true` if the provider has valid OAuth credentials.
    async fn is_connected(
        &self,
        tenant_id: &TenantId,
        provider: &str,
    ) -> errors::Result<bool> {
        Ok(self.get_token(tenant_id, provider).await?.is_some())
    }
}

/// AuthApp-backed OAuth token provider.
///
/// This implementation wraps AuthApp to provide OAuth tokens
/// for the inbound sync system. It creates a system executor
/// and operator-based multi-tenancy context for token retrieval.
#[derive(Debug)]
pub struct AuthAppTokenProvider {
    auth_app: Arc<dyn tachyon_sdk::auth::AuthApp>,
}

impl AuthAppTokenProvider {
    /// Create a new AuthApp-backed token provider.
    pub fn new(auth_app: Arc<dyn tachyon_sdk::auth::AuthApp>) -> Self {
        Self { auth_app }
    }
}

#[async_trait]
impl OAuthTokenProvider for AuthAppTokenProvider {
    async fn get_token(
        &self,
        tenant_id: &TenantId,
        provider: &str,
    ) -> errors::Result<Option<ProviderToken>> {
        // Create system executor for internal operations
        let executor = SystemExecutor;
        // Create multi-tenancy context from tenant_id
        let multi_tenancy = OperatorMultiTenancy::new(tenant_id.clone());

        let mut token = self
            .auth_app
            .get_oauth_token_by_provider(
                &tachyon_sdk::auth::GetOAuthTokenByProviderInput {
                    executor: &executor,
                    multi_tenancy: &multi_tenancy,
                    provider,
                },
            )
            .await?;

        if let Some(ref current) = token {
            if current.expires_at <= Utc::now() {
                // Refresh expired tokens via the auth usecase.
                let _ = self
                    .auth_app
                    .oauth_tokens(&tachyon_sdk::auth::OAuthTokenInput {
                        executor: &executor,
                        multi_tenancy: &multi_tenancy,
                    })
                    .await?;
                token = self
                    .auth_app
                    .get_oauth_token_by_provider(
                        &tachyon_sdk::auth::GetOAuthTokenByProviderInput {
                            executor: &executor,
                            multi_tenancy: &multi_tenancy,
                            provider,
                        },
                    )
                    .await?;
            }
        }

        Ok(token.map(|t| ProviderToken {
            provider: t.provider,
            provider_user_id: t.provider_user_id,
            access_token: t.access_token,
            refresh_token: t.refresh_token,
            expires_at: t.expires_at,
        }))
    }
}

/// System executor for internal operations.
///
/// Used when processing webhooks (system-triggered operations)
/// where there is no user context.
#[derive(Debug, Clone)]
pub struct SystemExecutor;

impl tachyon_sdk::auth::ExecutorAction for SystemExecutor {
    fn get_id(&self) -> &str {
        "system"
    }

    fn has_tenant_id(&self, _tenant_id: &TenantId) -> bool {
        // System user has access to all tenants
        true
    }

    fn is_system_user(&self) -> bool {
        true
    }

    fn is_user(&self) -> bool {
        false
    }

    fn is_service_account(&self) -> bool {
        false
    }

    fn is_none(&self) -> bool {
        false
    }
}

/// Operator-based multi-tenancy context.
///
/// Used to provide multi-tenancy context for OAuth token retrieval
/// when processing webhooks. The operator_id is derived from the
/// webhook endpoint's tenant_id.
#[derive(Debug, Clone)]
pub struct OperatorMultiTenancy {
    operator_id: value_object::OperatorId,
}

impl OperatorMultiTenancy {
    /// Create a new operator multi-tenancy context.
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            operator_id: value_object::OperatorId::from(
                tenant_id.to_string(),
            ),
        }
    }
}

impl tachyon_sdk::auth::MultiTenancyAction for OperatorMultiTenancy {
    fn platform_id(&self) -> Option<value_object::PlatformId> {
        None
    }

    fn operator_id(&self) -> Option<value_object::OperatorId> {
        Some(self.operator_id.clone())
    }

    fn get_operator_id(&self) -> errors::Result<value_object::OperatorId> {
        Ok(self.operator_id.clone())
    }
}

/// NoOp implementation of OAuthTokenProvider for testing.
///
/// Returns None for all token requests.
#[derive(Debug, Clone, Default)]
pub struct NoOpTokenProvider;

#[async_trait]
impl OAuthTokenProvider for NoOpTokenProvider {
    async fn get_token(
        &self,
        _tenant_id: &TenantId,
        _provider: &str,
    ) -> errors::Result<Option<ProviderToken>> {
        Ok(None)
    }
}

/// Static token provider for testing and development.
///
/// Returns a pre-configured token for all requests.
#[derive(Debug, Clone)]
pub struct StaticTokenProvider {
    token: ProviderToken,
}

impl StaticTokenProvider {
    /// Create a new static token provider.
    pub fn new(
        provider: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Self {
        Self {
            token: ProviderToken {
                provider: provider.into(),
                provider_user_id: "static_user".to_string(),
                access_token: access_token.into(),
                refresh_token: None,
                expires_at: Utc::now() + chrono::Duration::days(365),
            },
        }
    }

    /// Create with full token details.
    pub fn with_token(token: ProviderToken) -> Self {
        Self { token }
    }
}

#[async_trait]
impl OAuthTokenProvider for StaticTokenProvider {
    async fn get_token(
        &self,
        _tenant_id: &TenantId,
        provider: &str,
    ) -> errors::Result<Option<ProviderToken>> {
        if self.token.provider == provider {
            Ok(Some(self.token.clone()))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tenant_id() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap()
    }

    #[test]
    fn test_provider_token_is_expired() {
        let expired_token = ProviderToken {
            provider: "square".to_string(),
            provider_user_id: "merchant_123".to_string(),
            access_token: "expired_token".to_string(),
            refresh_token: None,
            expires_at: Utc::now() - chrono::Duration::hours(1),
        };
        assert!(expired_token.is_expired());

        let valid_token = ProviderToken {
            provider: "square".to_string(),
            provider_user_id: "merchant_123".to_string(),
            access_token: "valid_token".to_string(),
            refresh_token: None,
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };
        assert!(!valid_token.is_expired());
    }

    #[test]
    fn test_provider_token_expires_within() {
        let token = ProviderToken {
            provider: "square".to_string(),
            provider_user_id: "merchant_123".to_string(),
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: Utc::now() + chrono::Duration::minutes(30),
        };

        // Token expires within 1 hour
        assert!(token.expires_within(chrono::Duration::hours(1)));
        // Token doesn't expire within 15 minutes
        assert!(!token.expires_within(chrono::Duration::minutes(15)));
    }

    #[tokio::test]
    async fn test_noop_token_provider() {
        let provider = NoOpTokenProvider;
        let result = provider
            .get_token(&test_tenant_id(), "square")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_static_token_provider() {
        let provider = StaticTokenProvider::new("square", "test_token");

        // Should return token for matching provider
        let result = provider
            .get_token(&test_tenant_id(), "square")
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().access_token, "test_token");

        // Should return None for non-matching provider
        let result = provider
            .get_token(&test_tenant_id(), "github")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_is_connected() {
        let provider = StaticTokenProvider::new("square", "test_token");

        assert!(provider
            .is_connected(&test_tenant_id(), "square")
            .await
            .unwrap());
        assert!(!provider
            .is_connected(&test_tenant_id(), "github")
            .await
            .unwrap());
    }

    #[test]
    fn test_system_executor() {
        use tachyon_sdk::auth::ExecutorAction;

        let executor = SystemExecutor;
        assert!(executor.is_system_user());
        assert!(!executor.is_user());
        assert!(!executor.is_service_account());
        assert!(!executor.is_none());
        assert_eq!(executor.get_id(), "system");
    }

    #[test]
    fn test_operator_multi_tenancy() {
        use tachyon_sdk::auth::MultiTenancyAction;

        let mt = OperatorMultiTenancy::new(test_tenant_id());
        assert!(mt.operator_id().is_some());
        assert!(mt.platform_id().is_none());
        assert!(mt.get_operator_id().is_ok());
    }
}
