//! OAuth service implementation for integration authentication.
//!
//! This module provides HTTP-based OAuth token exchange for various providers.

use std::collections::HashMap;

use inbound_sync_domain::{
    ExchangeOAuthCodeInput, InitOAuthInput, InitOAuthOutput,
    OAuthClientCredentials, OAuthProvider, OAuthService,
    OAuthTokenRepository, OAuthTokenResponse, StoredOAuthToken,
};
use reqwest::Url;
use ulid::Ulid;
use value_object::TenantId;

/// HTTP-based OAuth service implementation.
#[derive(Debug)]
pub struct HttpOAuthService {
    /// OAuth client credentials by provider
    credentials: HashMap<OAuthProvider, OAuthClientCredentials>,
    /// Token repository for storing tokens
    token_repository: std::sync::Arc<dyn OAuthTokenRepository>,
    /// HTTP client for making token requests
    http_client: reqwest::Client,
}

impl HttpOAuthService {
    /// Create a new HTTP OAuth service.
    pub fn new(
        token_repository: std::sync::Arc<dyn OAuthTokenRepository>,
    ) -> Self {
        Self {
            credentials: HashMap::new(),
            token_repository,
            http_client: reqwest::Client::new(),
        }
    }

    /// Add OAuth credentials for a provider.
    pub fn with_credentials(
        mut self,
        provider: OAuthProvider,
        credentials: OAuthClientCredentials,
    ) -> Self {
        self.credentials.insert(provider, credentials);
        self
    }

    /// Get authorization URL for a provider.
    fn get_auth_url(&self, provider: OAuthProvider) -> Option<&str> {
        match provider {
            OAuthProvider::Github => {
                Some("https://github.com/login/oauth/authorize")
            }
            OAuthProvider::Linear => {
                Some("https://linear.app/oauth/authorize")
            }
            OAuthProvider::Hubspot => {
                Some("https://app.hubspot.com/oauth/authorize")
            }
            OAuthProvider::Stripe => None, // Stripe uses API keys
            OAuthProvider::Square => {
                Some("https://connect.squareup.com/oauth2/authorize")
            }
            OAuthProvider::Notion => {
                Some("https://api.notion.com/v1/oauth/authorize")
            }
            OAuthProvider::Airtable => {
                Some("https://airtable.com/oauth2/v1/authorize")
            }
            OAuthProvider::Slack => {
                Some("https://slack.com/oauth/v2/authorize")
            }
            OAuthProvider::Discord => None,
            OAuthProvider::Generic => None,
            OAuthProvider::Custom => None,
        }
    }

    /// Get token URL for a provider.
    fn get_token_url(&self, provider: OAuthProvider) -> Option<&str> {
        match provider {
            OAuthProvider::Github => {
                Some("https://github.com/login/oauth/access_token")
            }
            OAuthProvider::Linear => {
                Some("https://api.linear.app/oauth/token")
            }
            OAuthProvider::Hubspot => {
                Some("https://api.hubapi.com/oauth/v1/token")
            }
            OAuthProvider::Stripe => None, // Stripe uses API keys
            OAuthProvider::Square => {
                Some("https://connect.squareup.com/oauth2/token")
            }
            OAuthProvider::Notion => {
                Some("https://api.notion.com/v1/oauth/token")
            }
            OAuthProvider::Airtable => {
                Some("https://airtable.com/oauth2/v1/token")
            }
            OAuthProvider::Slack => {
                Some("https://slack.com/api/oauth.v2.access")
            }
            OAuthProvider::Discord => None,
            OAuthProvider::Generic => None,
            OAuthProvider::Custom => None,
        }
    }

    /// Get default scopes for a provider.
    fn get_default_scopes(&self, provider: OAuthProvider) -> Vec<&str> {
        match provider {
            OAuthProvider::Github => {
                vec!["repo", "read:org", "read:user"]
            }
            OAuthProvider::Linear => vec!["read", "write"],
            OAuthProvider::Hubspot => vec![
                "crm.objects.contacts.read",
                "crm.objects.companies.read",
                "crm.objects.deals.read",
            ],
            OAuthProvider::Square => vec![
                "ITEMS_READ",
                "CUSTOMERS_READ",
                "ORDERS_READ",
                "INVENTORY_READ",
                "PAYMENTS_READ",
            ],
            OAuthProvider::Notion => vec!["read_content", "update_content"],
            OAuthProvider::Airtable => vec![
                "data.records:read",
                "data.records:write",
                "schema.bases:read",
            ],
            _ => vec![],
        }
    }
}

#[async_trait::async_trait]
impl OAuthService for HttpOAuthService {
    async fn init_authorization(
        &self,
        input: InitOAuthInput,
    ) -> errors::Result<InitOAuthOutput> {
        let credentials =
            self.credentials.get(&input.provider).ok_or_else(|| {
                errors::Error::bad_request(format!(
                    "No OAuth credentials configured for {:?}",
                    input.provider
                ))
            })?;

        let auth_url =
            self.get_auth_url(input.provider).ok_or_else(|| {
                errors::Error::bad_request(format!(
                    "Provider {:?} does not support OAuth",
                    input.provider
                ))
            })?;

        // Generate state for CSRF protection
        let state = input
            .state
            .unwrap_or_else(|| Ulid::new().to_string().to_lowercase());

        // Build authorization URL.
        // NOTE: redirect_uri must be registered in the OAuth provider's
        // app settings (e.g. GitHub App "Callback URL"). A mismatch
        // causes "redirect_uri is not associated with this application".
        let scopes = self.get_default_scopes(input.provider).join(" ");
        // Prefer redirect_uri from credentials (configured in IaC
        // manifest) so the OAuth callback goes to the correct
        // endpoint (e.g. tachyon-api proxy for GitHub).
        let redirect_uri = if !credentials.redirect_uri.is_empty() {
            credentials.redirect_uri.clone()
        } else {
            input.redirect_uri.unwrap_or_else(|| {
                format!(
                    "{}/v1beta/{}/integrations/callback",
                    input.base_url, input.tenant_id
                )
            })
        };

        tracing::debug!(
            ?input.provider,
            %redirect_uri,
            "Building OAuth authorization URL"
        );

        let mut url = Url::parse(auth_url).map_err(|e| {
            errors::Error::internal_server_error(format!(
                "Invalid auth URL: {e}"
            ))
        })?;

        url.query_pairs_mut()
            .append_pair("client_id", &credentials.client_id)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("scope", &scopes)
            .append_pair("response_type", "code")
            .append_pair("state", &state);

        Ok(InitOAuthOutput {
            authorization_url: url.to_string(),
            state,
        })
    }

    async fn exchange_code(
        &self,
        input: ExchangeOAuthCodeInput,
    ) -> errors::Result<StoredOAuthToken> {
        let credentials =
            self.credentials.get(&input.provider).ok_or_else(|| {
                errors::Error::bad_request(format!(
                    "No OAuth credentials configured for {:?}",
                    input.provider
                ))
            })?;

        let token_url =
            self.get_token_url(input.provider).ok_or_else(|| {
                errors::Error::bad_request(format!(
                    "Provider {:?} does not support OAuth",
                    input.provider
                ))
            })?;

        // Use credentials' redirect_uri so it matches the one used
        // during authorization (required by GitHub OAuth).
        let redirect_uri = if !credentials.redirect_uri.is_empty() {
            credentials.redirect_uri.clone()
        } else {
            input.redirect_uri
        };

        // Build token request
        let mut params = HashMap::new();
        params.insert("client_id", credentials.client_id.clone());
        params.insert("client_secret", credentials.client_secret.clone());
        params.insert("code", input.code);
        params.insert("redirect_uri", redirect_uri);
        params.insert("grant_type", "authorization_code".to_string());

        // Make token request
        let response = self
            .http_client
            .post(token_url)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                errors::Error::service_unavailable(format!(
                    "Failed to exchange OAuth code: {e}"
                ))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(errors::Error::service_unavailable(format!(
                "OAuth token exchange failed: {error_text}"
            )));
        }

        let token_response: OAuthTokenResponse =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse OAuth response: {e}"
                ))
            })?;

        let stored_token = StoredOAuthToken::from_response(
            Ulid::new().to_string(),
            input.tenant_id.clone(),
            input.provider,
            token_response,
        );

        // Save token
        self.token_repository.save(&stored_token).await?;

        Ok(stored_token)
    }

    async fn refresh_token(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<StoredOAuthToken> {
        let credentials =
            self.credentials.get(&provider).ok_or_else(|| {
                errors::Error::bad_request(format!(
                    "No OAuth credentials configured for {provider:?}"
                ))
            })?;

        let existing_token = self
            .token_repository
            .find_by_tenant_and_provider(tenant_id, provider)
            .await?
            .ok_or_else(|| {
                errors::Error::not_found(format!(
                    "No token found for tenant {tenant_id} and provider {provider:?}"
                ))
            })?;

        let refresh_token =
            existing_token.refresh_token.ok_or_else(|| {
                errors::Error::bad_request(
                    "No refresh token available".to_string(),
                )
            })?;

        let token_url = self.get_token_url(provider).ok_or_else(|| {
            errors::Error::bad_request(format!(
                "Provider {provider:?} does not support OAuth"
            ))
        })?;

        // Build refresh request
        let mut params = HashMap::new();
        params.insert("client_id", credentials.client_id.clone());
        params.insert("client_secret", credentials.client_secret.clone());
        params.insert("refresh_token", refresh_token);
        params.insert("grant_type", "refresh_token".to_string());

        // Make refresh request
        let response = self
            .http_client
            .post(token_url)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                errors::Error::service_unavailable(format!(
                    "Failed to refresh OAuth token: {e}"
                ))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(errors::Error::service_unavailable(format!(
                "OAuth token refresh failed: {error_text}"
            )));
        }

        let token_response: OAuthTokenResponse =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse OAuth response: {e}"
                ))
            })?;

        let stored_token = StoredOAuthToken::from_response(
            Ulid::new().to_string(),
            tenant_id.clone(),
            provider,
            token_response,
        );

        // Save updated token
        self.token_repository.save(&stored_token).await?;

        Ok(stored_token)
    }

    async fn revoke_token(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<()> {
        // Delete the stored token
        self.token_repository.delete(tenant_id, provider).await?;
        Ok(())
    }

    fn get_credentials(
        &self,
        provider: OAuthProvider,
    ) -> Option<&OAuthClientCredentials> {
        self.credentials.get(&provider)
    }
}

/// In-memory OAuth token repository for testing.
#[derive(Debug, Default)]
pub struct InMemoryOAuthTokenRepository {
    tokens: std::sync::RwLock<
        HashMap<(TenantId, OAuthProvider), StoredOAuthToken>,
    >,
}

#[async_trait::async_trait]
impl OAuthTokenRepository for InMemoryOAuthTokenRepository {
    async fn save(&self, token: &StoredOAuthToken) -> errors::Result<()> {
        let mut tokens = self.tokens.write().unwrap();
        tokens.insert(
            (token.tenant_id.clone(), token.provider),
            token.clone(),
        );
        Ok(())
    }

    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<Option<StoredOAuthToken>> {
        let tokens = self.tokens.read().unwrap();
        Ok(tokens.get(&(tenant_id.clone(), provider)).cloned())
    }

    async fn delete(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<()> {
        let mut tokens = self.tokens.write().unwrap();
        tokens.remove(&(tenant_id.clone(), provider));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tenant_id() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap()
    }

    #[tokio::test]
    async fn test_in_memory_token_repository() {
        let repo = InMemoryOAuthTokenRepository::default();
        let tenant_id = test_tenant_id();

        let response = OAuthTokenResponse {
            access_token: "access_123".to_string(),
            refresh_token: Some("refresh_456".to_string()),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            scope: Some("read write".to_string()),
        };

        let token = StoredOAuthToken::from_response(
            ulid::Ulid::new().to_string(),
            tenant_id.clone(),
            OAuthProvider::Github,
            response,
        );

        // Save
        repo.save(&token).await.unwrap();

        // Find
        let found = repo
            .find_by_tenant_and_provider(&tenant_id, OAuthProvider::Github)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().access_token, "access_123");

        // Delete
        repo.delete(&tenant_id, OAuthProvider::Github)
            .await
            .unwrap();
        let found = repo
            .find_by_tenant_and_provider(&tenant_id, OAuthProvider::Github)
            .await
            .unwrap();
        assert!(found.is_none());
    }
}
