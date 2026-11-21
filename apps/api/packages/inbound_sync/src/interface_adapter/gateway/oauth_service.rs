//! OAuth service implementation for integration authentication.
//!
//! This module provides HTTP-based OAuth token exchange for various providers.

use std::collections::HashMap;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use inbound_sync_domain::{
    ExchangeOAuthCodeInput, InitOAuthInput, InitOAuthOutput,
    OAuthClientCredentials, OAuthCredentialsSource, OAuthProvider,
    OAuthService, OAuthTokenRepository, OAuthTokenResponse,
    StoredOAuthToken,
};
use reqwest::Url;
use sha2::Sha256;
use ulid::Ulid;
use value_object::TenantId;

/// How long an authorization may take between `init_authorization` and
/// `exchange_code`.
const OAUTH_STATE_TTL_SECS: i64 = 600;

/// Version of the states `mint_oauth_state` signs.
const OAUTH_STATE_VERSION: i64 = 1;

/// Who an authorization was started by, for which tenant and provider.
#[derive(Debug, Clone, Copy)]
struct OAuthStateBinding<'a> {
    tenant_id: &'a TenantId,
    provider: OAuthProvider,
    user_id: &'a str,
}

fn oauth_state_mac(secret: &str) -> errors::Result<Hmac<Sha256>> {
    if secret.is_empty() {
        return Err(errors::Error::internal_server_error(
            "OAuth client secret is not configured",
        ));
    }
    Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))
}

/// Mint the `state` of an authorization for `binding`, signed with the
/// provider's client secret: `{base64url payload}.{hex hmac}`.
///
/// The state used to be whatever the caller sent, or a random value that
/// was never stored, and `exchange_code` never read it, so it protected
/// nothing: anyone's authorization code could be attached to a tenant.
/// It now names the user and tenant that started the authorization and
/// expires, and `exchange_code` accepts it only for them. Nothing records
/// that a state was used, as there is no store shared across instances for
/// it; the binding and the expiry are what protect the exchange, and the
/// provider accepts each code once.
fn mint_oauth_state(
    binding: OAuthStateBinding<'_>,
    client_state: Option<&str>,
    secret: &str,
    now: i64,
) -> errors::Result<String> {
    if binding.user_id.is_empty() {
        return Err(errors::Error::unauthorized(
            "Authentication is required to connect an integration",
        ));
    }
    let payload = serde_json::json!({
        "v": OAUTH_STATE_VERSION,
        "tenantId": binding.tenant_id.as_str(),
        "provider": format!("{:?}", binding.provider),
        "userId": binding.user_id,
        "expiresAt": now + OAUTH_STATE_TTL_SECS,
        "nonce": Ulid::new().to_string(),
        "client": client_state,
    });
    let encoded =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).map_err(
            |e| errors::Error::internal_server_error(e.to_string()),
        )?);
    let mut mac = oauth_state_mac(secret)?;
    mac.update(encoded.as_bytes());
    Ok(format!(
        "{encoded}.{}",
        hex::encode(mac.finalize().into_bytes())
    ))
}

/// Accept `state` only if `mint_oauth_state` signed it for `binding` and
/// it has not expired.
fn verify_oauth_state(
    state: &str,
    binding: OAuthStateBinding<'_>,
    secret: &str,
    now: i64,
) -> errors::Result<()> {
    let invalid =
        || errors::Error::bad_request("Invalid or expired OAuth state");
    let (encoded, signature) =
        state.rsplit_once('.').ok_or_else(invalid)?;
    let signature = hex::decode(signature).map_err(|_| invalid())?;
    let mut mac = oauth_state_mac(secret)?;
    mac.update(encoded.as_bytes());
    mac.verify_slice(&signature).map_err(|_| invalid())?;
    let payload: serde_json::Value = URL_SAFE_NO_PAD
        .decode(encoded)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .ok_or_else(invalid)?;
    let provider = format!("{:?}", binding.provider);
    if binding.user_id.is_empty()
        || payload["v"].as_i64() != Some(OAUTH_STATE_VERSION)
        || payload["tenantId"].as_str() != Some(binding.tenant_id.as_str())
        || payload["provider"].as_str() != Some(provider.as_str())
        || payload["userId"].as_str() != Some(binding.user_id)
        || !payload["expiresAt"].as_i64().is_some_and(|expiry| {
            expiry > now && expiry <= now + OAUTH_STATE_TTL_SECS
        })
    {
        return Err(invalid());
    }
    Ok(())
}

/// HTTP-based OAuth service implementation.
#[derive(Debug)]
pub struct HttpOAuthService {
    /// OAuth client credentials configured up front, by provider.
    credentials: HashMap<OAuthProvider, OAuthClientCredentials>,
    /// Resolves any credentials not configured up front.
    ///
    /// The real ones come from Tachyon's IaC configuration over the
    /// network, and asking for them here rather than at construction
    /// keeps that fetch off the startup path.
    credential_source: Option<std::sync::Arc<dyn OAuthCredentialsSource>>,
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
            credential_source: None,
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

    /// Resolve any credentials this was not given up front.
    pub fn with_credential_source(
        mut self,
        source: std::sync::Arc<dyn OAuthCredentialsSource>,
    ) -> Self {
        self.credential_source = Some(source);
        self
    }

    /// Credentials for `provider`, from what was configured up front or
    /// else from the source.
    async fn credentials_for(
        &self,
        provider: OAuthProvider,
    ) -> Option<OAuthClientCredentials> {
        if let Some(credentials) = self.credentials.get(&provider) {
            return Some(credentials.clone());
        }

        self.credential_source.as_ref()?.credentials(provider).await
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
        let credentials = self
            .credentials_for(input.provider)
            .await
            .ok_or_else(|| {
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

        // The state is minted here and bound to the caller, never taken
        // from the request.
        let state = mint_oauth_state(
            OAuthStateBinding {
                tenant_id: &input.tenant_id,
                provider: input.provider,
                user_id: &input.user_id,
            },
            input.state.as_deref(),
            &credentials.client_secret,
            chrono::Utc::now().timestamp(),
        )?;

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
        let credentials = self
            .credentials_for(input.provider)
            .await
            .ok_or_else(|| {
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

        // Only the user who started this authorization, for this tenant,
        // may finish it; otherwise anyone's code could be attached here.
        verify_oauth_state(
            &input.state,
            OAuthStateBinding {
                tenant_id: &input.tenant_id,
                provider: input.provider,
                user_id: &input.user_id,
            },
            &credentials.client_secret,
            chrono::Utc::now().timestamp(),
        )?;

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
            self.credentials_for(provider).await.ok_or_else(|| {
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

    const SECRET: &str = "client-secret";
    const NOW: i64 = 1_000;

    fn binding(tenant_id: &TenantId) -> OAuthStateBinding<'_> {
        OAuthStateBinding {
            tenant_id,
            provider: OAuthProvider::Linear,
            user_id: "us_alice",
        }
    }

    #[test]
    fn a_minted_state_is_accepted_only_for_its_binding() {
        let tenant = test_tenant_id();
        let other: TenantId =
            "tn_01hjryxysgey07h5jz5wagqj0n".parse().unwrap();
        let state =
            mint_oauth_state(binding(&tenant), Some("ui"), SECRET, NOW)
                .unwrap();

        assert!(verify_oauth_state(&state, binding(&tenant), SECRET, NOW)
            .is_ok());
        for wrong in [
            binding(&other),
            OAuthStateBinding {
                user_id: "us_mallory",
                ..binding(&tenant)
            },
            OAuthStateBinding {
                provider: OAuthProvider::Github,
                ..binding(&tenant)
            },
        ] {
            assert!(verify_oauth_state(&state, wrong, SECRET, NOW).is_err());
        }
        assert!(verify_oauth_state(
            &state,
            binding(&tenant),
            SECRET,
            NOW + OAUTH_STATE_TTL_SECS
        )
        .is_err());
        assert!(verify_oauth_state(
            &state,
            binding(&tenant),
            "other-secret",
            NOW
        )
        .is_err());
    }

    #[test]
    fn caller_supplied_or_unsigned_states_are_refused() {
        let tenant = test_tenant_id();
        let forged = URL_SAFE_NO_PAD.encode(
            serde_json::json!({
                "v": OAUTH_STATE_VERSION, "tenantId": tenant.as_str(),
                "provider": "Linear", "userId": "us_alice",
                "expiresAt": NOW + 60,
            })
            .to_string(),
        );
        for state in [
            forged.clone(),
            format!("{forged}.00"),
            "01jstate".to_string(),
            String::new(),
        ] {
            assert!(
                verify_oauth_state(&state, binding(&tenant), SECRET, NOW)
                    .is_err(),
                "{state}"
            );
        }
    }

    #[test]
    fn a_state_is_not_minted_without_a_user_or_a_secret() {
        let tenant = test_tenant_id();
        let anonymous = OAuthStateBinding {
            user_id: "",
            ..binding(&tenant)
        };
        assert!(mint_oauth_state(anonymous, None, SECRET, NOW).is_err());
        assert!(mint_oauth_state(binding(&tenant), None, "", NOW).is_err());
    }
}
