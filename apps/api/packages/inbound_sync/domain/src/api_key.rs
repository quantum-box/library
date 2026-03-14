//! API key validation for integration authentication.
//!
//! This module provides API key validation for integrations that
//! don't use OAuth, such as API key-based services.

use serde::{Deserialize, Serialize};

use crate::OAuthProvider;

/// Result of API key validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyValidationResult {
    /// Whether the API key is valid
    pub is_valid: bool,
    /// External account ID associated with the API key
    pub external_account_id: Option<String>,
    /// External account name for display
    pub external_account_name: Option<String>,
    /// Error message if validation failed
    pub error_message: Option<String>,
}

impl ApiKeyValidationResult {
    /// Create a successful validation result.
    pub fn success(
        account_id: impl Into<String>,
        account_name: Option<String>,
    ) -> Self {
        Self {
            is_valid: true,
            external_account_id: Some(account_id.into()),
            external_account_name: account_name,
            error_message: None,
        }
    }

    /// Create a failed validation result.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            external_account_id: None,
            external_account_name: None,
            error_message: Some(message.into()),
        }
    }
}

/// Service for validating API keys.
#[async_trait::async_trait]
pub trait ApiKeyValidator: Send + Sync + std::fmt::Debug {
    /// Validate an API key for a provider.
    ///
    /// Returns validation result with account info if successful.
    async fn validate(
        &self,
        provider: OAuthProvider,
        api_key: &str,
    ) -> errors::Result<ApiKeyValidationResult>;

    /// Check if a provider uses API key authentication.
    fn uses_api_key(&self, provider: OAuthProvider) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_success() {
        let result = ApiKeyValidationResult::success(
            "acct_123",
            Some("Test Account".to_string()),
        );
        assert!(result.is_valid);
        assert_eq!(
            result.external_account_id,
            Some("acct_123".to_string())
        );
        assert_eq!(
            result.external_account_name,
            Some("Test Account".to_string())
        );
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_validation_result_failure() {
        let result = ApiKeyValidationResult::failure("Invalid API key");
        assert!(!result.is_valid);
        assert!(result.external_account_id.is_none());
        assert_eq!(
            result.error_message,
            Some("Invalid API key".to_string())
        );
    }
}
