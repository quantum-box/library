//! API key validation implementations for various providers.
//!
//! This module provides concrete implementations for validating API keys
//! against external provider APIs.

use async_trait::async_trait;
use inbound_sync_domain::{
    ApiKeyValidationResult, ApiKeyValidator, OAuthProvider,
};
use serde::Deserialize;

/// HTTP-based API key validator that validates keys against provider APIs.
#[derive(Debug)]
pub struct HttpApiKeyValidator {
    http_client: reqwest::Client,
    /// Stripe API base URL (for testing/mocking)
    stripe_base_url: String,
}

impl HttpApiKeyValidator {
    /// Create a new HTTP API key validator.
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            stripe_base_url: "https://api.stripe.com".to_string(),
        }
    }

    /// Create with custom Stripe base URL (for testing).
    #[cfg(test)]
    pub fn with_stripe_url(stripe_base_url: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            stripe_base_url,
        }
    }

    /// Validate a Stripe API key by fetching account info.
    async fn validate_stripe_key(
        &self,
        api_key: &str,
    ) -> errors::Result<ApiKeyValidationResult> {
        // Use the /v1/account endpoint to validate the API key
        let url = format!("{}/v1/account", self.stripe_base_url);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await
            .map_err(|e| {
                errors::Error::service_unavailable(format!(
                    "Failed to connect to Stripe API: {e}"
                ))
            })?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Ok(ApiKeyValidationResult::failure(
                "Invalid Stripe API key",
            ));
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::warn!(
                error = %error_text,
                "Stripe API returned non-success status"
            );
            return Ok(ApiKeyValidationResult::failure(format!(
                "Stripe API error: {error_text}"
            )));
        }

        let account: StripeAccount =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse Stripe account response: {e}"
                ))
            })?;

        Ok(ApiKeyValidationResult::success(
            account.id,
            account.business_profile.and_then(|p| p.name),
        ))
    }

    /// Validate a generic API key (placeholder - always succeeds).
    async fn validate_generic_key(
        &self,
        api_key: &str,
    ) -> errors::Result<ApiKeyValidationResult> {
        // For generic providers, we can't validate the key
        // Just return success with a hash-based ID
        let id = format!(
            "generic_{}",
            &hex::encode(&api_key.as_bytes()[..8.min(api_key.len())])
        );
        Ok(ApiKeyValidationResult::success(id, None))
    }
}

impl Default for HttpApiKeyValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApiKeyValidator for HttpApiKeyValidator {
    async fn validate(
        &self,
        provider: OAuthProvider,
        api_key: &str,
    ) -> errors::Result<ApiKeyValidationResult> {
        match provider {
            OAuthProvider::Stripe => {
                self.validate_stripe_key(api_key).await
            }
            OAuthProvider::Generic => {
                self.validate_generic_key(api_key).await
            }
            _ => {
                // For providers that use OAuth, API key validation is not applicable
                Ok(ApiKeyValidationResult::failure(format!(
                    "Provider {provider:?} does not use API key authentication"
                )))
            }
        }
    }

    fn uses_api_key(&self, provider: OAuthProvider) -> bool {
        matches!(provider, OAuthProvider::Stripe | OAuthProvider::Generic)
    }
}

/// Stripe account response structure.
#[derive(Debug, Deserialize)]
struct StripeAccount {
    /// Account ID (e.g., "acct_xxxxx")
    id: String,
    /// Business profile information
    business_profile: Option<StripeBusinessProfile>,
}

/// Stripe business profile.
#[derive(Debug, Deserialize)]
struct StripeBusinessProfile {
    /// Business name
    name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uses_api_key() {
        let validator = HttpApiKeyValidator::new();

        assert!(validator.uses_api_key(OAuthProvider::Stripe));
        assert!(validator.uses_api_key(OAuthProvider::Generic));
        assert!(!validator.uses_api_key(OAuthProvider::Github));
        assert!(!validator.uses_api_key(OAuthProvider::Linear));
        assert!(!validator.uses_api_key(OAuthProvider::Notion));
    }
}
