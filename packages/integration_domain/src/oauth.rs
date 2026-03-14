//! OAuth domain models for authentication and integration.
//!
//! This module provides OAuth configuration, token management, and
//! credential management for external provider integrations.

use chrono::{DateTime, Duration, Utc};
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use value_object::{OperatorId, TenantId};

// =============================================================================
// OAuth Provider
// =============================================================================

/// OAuth provider type.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OAuthProvider {
    Github,
    Linear,
    Hubspot,
    Stripe,
    Notion,
    Square,
    Airtable,
    Slack,
    Discord,
    Generic,
    Custom,
}

// =============================================================================
// Legacy OAuthToken (for backward compatibility with existing usecases)
// =============================================================================

/// OAuth token for a tenant (legacy format).
///
/// This struct is maintained for backward compatibility with existing usecases.
/// For new code, consider using `StoredOAuthToken` which provides additional
/// metadata and functionality.
#[derive(Debug, Clone, Getters, new)]
pub struct OAuthToken {
    operator_id: OperatorId,
    provider: String,
    provider_user_id: String,
    access_token: String,
    token_type: String,
    expires_at: DateTime<Utc>,
    refresh_token: Option<String>,
    scope: Option<String>,
}

impl OAuthToken {
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }

    /// Refresh the token with new credentials.
    ///
    /// Note: Method name has typo for backward compatibility.
    pub fn reflesh_token(
        &self,
        access_token: String,
        refresh_token: Option<String>,
        expires_in: i64,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_at: Utc::now() + Duration::seconds(expires_in),
            ..self.clone()
        }
    }

    /// Refresh the token with new credentials (correctly spelled).
    pub fn refresh_token_data(
        &self,
        access_token: String,
        refresh_token: Option<String>,
        expires_in: i64,
    ) -> Self {
        self.reflesh_token(access_token, refresh_token, expires_in)
    }
}

/// Repository for legacy OAuthToken.
#[async_trait::async_trait]
pub trait OAuthTokenRepository: Send + Sync + Debug {
    async fn save(&self, token: &OAuthToken) -> errors::Result<()>;
    async fn find_by_tenant_id_and_provider(
        &self,
        operator_id: &OperatorId,
        provider: &str,
    ) -> errors::Result<Option<OAuthToken>>;
    async fn delete_by_tenant_id_and_provider(
        &self,
        operator_id: &OperatorId,
        provider: &str,
    ) -> errors::Result<()>;
    async fn find_all_by_operator_id(
        &self,
        operator_id: &OperatorId,
    ) -> errors::Result<Vec<OAuthToken>>;
}

// =============================================================================
// OAuth Credentials and Responses
// =============================================================================

/// OAuth client credentials for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClientCredentials {
    /// Client ID
    pub client_id: String,
    /// Client secret
    pub client_secret: String,
    /// Redirect URI for OAuth callback
    pub redirect_uri: String,
}

/// OAuth token response from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    /// Access token
    pub access_token: String,
    /// Refresh token (if supported)
    pub refresh_token: Option<String>,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Expiration time in seconds
    pub expires_in: Option<i64>,
    /// Scopes granted
    pub scope: Option<String>,
}

// =============================================================================
// StoredOAuthToken (enhanced token with metadata)
// =============================================================================

/// Stored OAuth token with metadata.
///
/// This is an enhanced version of OAuthToken that includes additional
/// metadata such as external account information and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredOAuthToken {
    /// Token ID (ULID)
    pub id: String,
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Provider
    pub provider: OAuthProvider,
    /// Access token
    pub access_token: String,
    /// Refresh token
    pub refresh_token: Option<String>,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Token expiration time
    pub expires_at: Option<DateTime<Utc>>,
    /// Scopes
    pub scopes: Vec<String>,
    /// External account ID (e.g., GitHub username)
    pub external_account_id: Option<String>,
    /// External account name
    pub external_account_name: Option<String>,
    /// When the token was created
    pub created_at: DateTime<Utc>,
    /// When the token was last updated
    pub updated_at: DateTime<Utc>,
}

/// Repository for storing StoredOAuthToken.
///
/// This is used by integration services to persist OAuth tokens
/// associated with external service connections.
#[async_trait::async_trait]
pub trait StoredOAuthTokenRepository:
    Send + Sync + std::fmt::Debug
{
    /// Save an OAuth token.
    async fn save(&self, token: &StoredOAuthToken) -> errors::Result<()>;

    /// Get a token by tenant and provider.
    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<Option<StoredOAuthToken>>;

    /// Delete a token.
    async fn delete(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<()>;
}

impl StoredOAuthToken {
    /// Create a new stored token from a token response.
    pub fn from_response(
        id: String,
        tenant_id: TenantId,
        provider: OAuthProvider,
        response: OAuthTokenResponse,
    ) -> Self {
        let now = Utc::now();
        let expires_at = response
            .expires_in
            .map(|secs| now + Duration::seconds(secs));
        let scopes = response
            .scope
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default();

        Self {
            id,
            tenant_id,
            provider,
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            token_type: response.token_type,
            expires_at,
            scopes,
            external_account_id: None,
            external_account_name: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set external account information.
    pub fn with_external_account(
        mut self,
        account_id: impl Into<String>,
        account_name: Option<String>,
    ) -> Self {
        self.external_account_id = Some(account_id.into());
        self.external_account_name = account_name;
        self
    }

    /// Check if the token is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp <= Utc::now())
            .unwrap_or(false)
    }

    /// Check if the token needs refresh (expired or about to expire).
    pub fn needs_refresh(&self) -> bool {
        self.expires_at
            .map(|exp| exp <= Utc::now() + Duration::minutes(5))
            .unwrap_or(false)
    }
}

// =============================================================================
// OAuth Service and Input/Output types
// =============================================================================

/// Input for OAuth authorization initialization.
#[derive(Debug, Clone)]
pub struct InitOAuthInput {
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Provider to connect
    pub provider: OAuthProvider,
    /// Base URL for redirect URI construction (used if redirect_uri is not
    /// provided)
    pub base_url: String,
    /// Optional explicit redirect URI (overrides base_url if provided)
    pub redirect_uri: Option<String>,
    /// Optional state for CSRF protection (will be generated if not provided)
    pub state: Option<String>,
}

/// Output for OAuth authorization initialization.
#[derive(Debug, Clone)]
pub struct InitOAuthOutput {
    /// Authorization URL to redirect user to
    pub authorization_url: String,
    /// State parameter for CSRF protection
    pub state: String,
}

/// Input for OAuth token exchange.
#[derive(Debug, Clone)]
pub struct ExchangeOAuthCodeInput {
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Provider
    pub provider: OAuthProvider,
    /// Authorization code from callback
    pub code: String,
    /// State parameter for verification
    pub state: Option<String>,
    /// Redirect URI used in the authorization request
    pub redirect_uri: String,
}

/// Service for OAuth operations.
#[async_trait::async_trait]
pub trait OAuthService: Send + Sync + Debug {
    /// Initialize OAuth authorization flow.
    ///
    /// Returns the authorization URL to redirect the user to.
    async fn init_authorization(
        &self,
        input: InitOAuthInput,
    ) -> errors::Result<InitOAuthOutput>;

    /// Exchange authorization code for tokens.
    ///
    /// Exchanges the code received from the OAuth callback for access/refresh
    /// tokens.
    async fn exchange_code(
        &self,
        input: ExchangeOAuthCodeInput,
    ) -> errors::Result<StoredOAuthToken>;

    /// Refresh an expired token.
    async fn refresh_token(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<StoredOAuthToken>;

    /// Revoke a token.
    async fn revoke_token(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<()>;

    /// Get OAuth client credentials for a provider.
    fn get_credentials(
        &self,
        provider: OAuthProvider,
    ) -> Option<&OAuthClientCredentials>;
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tenant_id() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap()
    }

    fn test_operator_id() -> OperatorId {
        "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap()
    }

    #[test]
    fn test_legacy_oauth_token_is_expired() {
        let token = OAuthToken::new(
            test_operator_id(),
            "github".to_string(),
            "user123".to_string(),
            "access_token".to_string(),
            "Bearer".to_string(),
            Utc::now() - Duration::hours(1),
            None,
            None,
        );
        assert!(token.is_expired());

        let token = OAuthToken::new(
            test_operator_id(),
            "github".to_string(),
            "user123".to_string(),
            "access_token".to_string(),
            "Bearer".to_string(),
            Utc::now() + Duration::hours(1),
            None,
            None,
        );
        assert!(!token.is_expired());
    }

    #[test]
    fn test_stored_token_from_response() {
        let response = OAuthTokenResponse {
            access_token: "access_123".to_string(),
            refresh_token: Some("refresh_456".to_string()),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            scope: Some("read write".to_string()),
        };

        let token = StoredOAuthToken::from_response(
            "token_123".to_string(),
            test_tenant_id(),
            OAuthProvider::Github,
            response,
        );

        assert_eq!(token.id, "token_123");
        assert_eq!(token.access_token, "access_123");
        assert_eq!(token.refresh_token, Some("refresh_456".to_string()));
        assert_eq!(token.token_type, "Bearer");
        assert!(!token.is_expired());
        assert!(token.expires_at.is_some());
        assert_eq!(token.scopes, vec!["read", "write"]);
    }

    #[test]
    fn test_token_expiration_check() {
        let response = OAuthTokenResponse {
            access_token: "access_123".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_in: Some(-10), // Already expired
            scope: None,
        };

        let token = StoredOAuthToken::from_response(
            "token_123".to_string(),
            test_tenant_id(),
            OAuthProvider::Github,
            response,
        );

        assert!(token.is_expired());
        assert!(token.needs_refresh());
    }

    #[test]
    fn test_oauth_provider_serialization() {
        assert_eq!(OAuthProvider::Github.to_string(), "github");
        assert_eq!(OAuthProvider::Stripe.to_string(), "stripe");

        let parsed: OAuthProvider = "github".parse().unwrap();
        assert_eq!(parsed, OAuthProvider::Github);
    }
}
