//! Square API client implementation.
//!
//! This module provides two implementations of the `SquareClient` trait:
//!
//! 1. `SquareApiClient` - Uses a pre-configured access token (for testing/static configs)
//! 2. `OAuthSquareClient` - Dynamically fetches tokens via `OAuthTokenProvider`
//!
//! # OAuth Integration
//!
//! For production use, prefer `OAuthSquareClient` which integrates with the
//! AuthApp OAuth system to automatically manage token retrieval and refresh.
//!
//! ```ignore
//! use inbound_sync::sdk::{AuthAppTokenProvider, OAuthTokenProvider};
//! use inbound_sync::providers::square::OAuthSquareClient;
//!
//! let token_provider = Arc::new(AuthAppTokenProvider::new(auth_app));
//! let client = Arc::new(OAuthSquareClient::new(token_provider));
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;
use value_object::TenantId;

use super::event_processor::SquareClient;
use super::payload::SquareObjectType;
use crate::sdk::OAuthTokenProvider;

const SQUARE_API_BASE: &str = "https://connect.squareup.com/v2";
const SQUARE_SANDBOX_API_BASE: &str =
    "https://connect.squareupsandbox.com/v2";
const USER_AGENT: &str = "inbound-sync/0.1.0";

/// Rate limiting configuration
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;
const MAX_BACKOFF_MS: u64 = 30000;

/// Square API client implementation.
#[derive(Debug)]
pub struct SquareApiClient {
    client: reqwest::Client,
    access_token: String,
    base_url: String,
}

impl SquareApiClient {
    /// Create a new Square API client.
    pub fn new(access_token: String) -> Self {
        let sandbox = std::env::var("SQUARE_SANDBOX")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let base_url = if sandbox {
            SQUARE_SANDBOX_API_BASE
        } else {
            SQUARE_API_BASE
        }
        .to_string();

        Self {
            client: reqwest::Client::new(),
            access_token,
            base_url,
        }
    }

    /// Build request headers.
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.access_token)
                .parse()
                .expect("Invalid access token"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            USER_AGENT.parse().expect("Invalid user agent"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().expect("Invalid content type"),
        );
        headers.insert(
            "Square-Version",
            "2024-01-18".parse().expect("Invalid Square version"),
        );
        headers
    }

    /// Calculate exponential backoff delay.
    fn calculate_backoff(attempt: u32) -> Duration {
        let delay_ms = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
        Duration::from_millis(delay_ms.min(MAX_BACKOFF_MS))
    }

    /// Get the API endpoint for an object type and ID.
    fn object_endpoint(
        object_type: SquareObjectType,
        object_id: &str,
    ) -> String {
        match object_type {
            SquareObjectType::Catalog
            | SquareObjectType::CatalogItem
            | SquareObjectType::CatalogCategory
            | SquareObjectType::CatalogItemVariation
            | SquareObjectType::CatalogModifier
            | SquareObjectType::CatalogTax
            | SquareObjectType::CatalogDiscount => {
                format!("catalog/object/{object_id}")
            }
            SquareObjectType::Customer => {
                format!("customers/{object_id}")
            }
            SquareObjectType::Order => format!("orders/{object_id}"),
            SquareObjectType::Payment => format!("payments/{object_id}"),
            SquareObjectType::Inventory => {
                // Inventory uses batch retrieve endpoint
                format!("inventory/{object_id}")
            }
            SquareObjectType::Subscription => {
                format!("subscriptions/{object_id}")
            }
            SquareObjectType::Invoice => format!("invoices/{object_id}"),
        }
    }

    /// Parse a paginated list response.
    async fn parse_list_response(
        response: reqwest::Response,
        items_key: &str,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)> {
        let status = response.status();
        if !status.is_success() {
            return Err(errors::Error::internal_server_error(format!(
                "Square API error: {status}"
            )));
        }

        let body: serde_json::Value =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse Square response: {e}"
                ))
            })?;

        let items = body
            .get(items_key)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let next_cursor = body
            .get("cursor")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok((items, next_cursor))
    }

    /// Execute a GET request with retry logic.
    async fn get_with_retry(
        &self,
        url: &str,
    ) -> errors::Result<reqwest::Response> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            let response = self
                .client
                .get(url)
                .headers(self.build_headers())
                .send()
                .await
                .map_err(|e| {
                    errors::Error::internal_server_error(format!(
                        "HTTP request failed: {e}"
                    ))
                })?;

            let status = response.status();

            // Handle rate limiting (429)
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                // Check for Retry-After header
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(60);

                let wait = Duration::from_secs(retry_after.min(300));
                tracing::warn!(
                    attempt = attempt + 1,
                    wait_secs = wait.as_secs(),
                    "Square API rate limited, waiting before retry"
                );
                sleep(wait).await;
                continue;
            }

            // Handle server errors
            if status.is_server_error() {
                let backoff = Self::calculate_backoff(attempt);
                tracing::warn!(
                    attempt = attempt + 1,
                    status = %status,
                    backoff_ms = backoff.as_millis() as u64,
                    "Square API server error, retrying with backoff"
                );
                last_error = Some(format!("Server error: {status}"));
                sleep(backoff).await;
                continue;
            }

            return Ok(response);
        }

        Err(errors::Error::internal_server_error(
            last_error
                .unwrap_or_else(|| "Max retries exceeded".to_string()),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct SquareErrorResponse {
    errors: Vec<SquareError>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SquareError {
    category: String,
    code: String,
    detail: Option<String>,
}

/// Static token implementation of SquareClient.
///
/// This implementation uses a pre-configured access token and ignores the
/// tenant_id parameter. Useful for testing or simple static configurations.
#[async_trait]
impl SquareClient for SquareApiClient {
    async fn get_object(
        &self,
        _tenant_id: &TenantId,
        object_type: SquareObjectType,
        object_id: &str,
    ) -> errors::Result<serde_json::Value> {
        let endpoint = Self::object_endpoint(object_type, object_id);
        let url = format!("{}/{endpoint}", self.base_url);

        tracing::debug!(url = %url, "Fetching Square object");

        let response = self.get_with_retry(&url).await?;
        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(errors::Error::not_found(format!(
                "Square {} not found: {}",
                object_type.as_str(),
                object_id
            )));
        }

        if !status.is_success() {
            let error: SquareErrorResponse =
                response.json().await.unwrap_or(SquareErrorResponse {
                    errors: vec![SquareError {
                        category: "UNKNOWN".to_string(),
                        code: "UNKNOWN_ERROR".to_string(),
                        detail: Some("Unknown error".to_string()),
                    }],
                });
            let error_message = error
                .errors
                .first()
                .and_then(|e| e.detail.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(errors::Error::internal_server_error(format!(
                "Square API error: {error_message}"
            )));
        }

        let response_body: serde_json::Value =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse Square response: {e}"
                ))
            })?;

        Ok(response_body)
    }

    async fn batch_retrieve_catalog_objects(
        &self,
        _tenant_id: &TenantId,
        object_ids: &[String],
    ) -> errors::Result<Vec<serde_json::Value>> {
        let url = format!("{}/catalog/batch-retrieve", self.base_url);

        let body = serde_json::json!({
            "object_ids": object_ids,
            "include_related_objects": true
        });

        tracing::debug!(
            url = %url,
            object_count = object_ids.len(),
            "Batch retrieving Square catalog objects"
        );

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "HTTP request failed: {e}"
                ))
            })?;

        let status = response.status();

        if !status.is_success() {
            let error: SquareErrorResponse =
                response.json().await.unwrap_or(SquareErrorResponse {
                    errors: vec![SquareError {
                        category: "UNKNOWN".to_string(),
                        code: "UNKNOWN_ERROR".to_string(),
                        detail: Some("Unknown error".to_string()),
                    }],
                });
            let error_message = error
                .errors
                .first()
                .and_then(|e| e.detail.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(errors::Error::internal_server_error(format!(
                "Square API error: {error_message}"
            )));
        }

        let response_body: serde_json::Value =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse Square response: {e}"
                ))
            })?;

        // Extract objects from response
        let objects = response_body
            .get("objects")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(objects)
    }

    async fn list_catalog_items(
        &self,
        _tenant_id: &TenantId,
        cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)> {
        let mut url = format!("{}/catalog/list?types=ITEM", self.base_url);
        if let Some(c) = cursor {
            url = format!("{url}&cursor={c}");
        }

        let response = self.get_with_retry(&url).await?;

        Self::parse_list_response(response, "objects").await
    }

    async fn list_customers(
        &self,
        _tenant_id: &TenantId,
        cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)> {
        let mut url = format!("{}/customers", self.base_url);
        if let Some(c) = cursor {
            url = format!("{url}?cursor={c}");
        }

        let response = self.get_with_retry(&url).await?;
        Self::parse_list_response(response, "customers").await
    }

    async fn list_orders(
        &self,
        _tenant_id: &TenantId,
        location_ids: &[String],
        cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)> {
        let url = format!("{}/orders/search", self.base_url);

        let mut body = serde_json::json!({
            "location_ids": location_ids
        });
        if let Some(c) = cursor {
            body["cursor"] = serde_json::Value::String(c.to_string());
        }

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "HTTP request failed: {e}"
                ))
            })?;

        Self::parse_list_response(response, "orders").await
    }
}

/// OAuth-integrated Square API client.
///
/// This implementation dynamically fetches OAuth tokens from the `OAuthTokenProvider`
/// for each API call, supporting multi-tenant scenarios where each tenant has
/// their own Square OAuth connection.
///
/// # Example
///
/// ```ignore
/// use inbound_sync::sdk::{AuthAppTokenProvider, OAuthTokenProvider};
/// use inbound_sync::providers::square::OAuthSquareClient;
///
/// let auth_app = /* ... */;
/// let token_provider = Arc::new(AuthAppTokenProvider::new(auth_app));
/// let client = OAuthSquareClient::new(token_provider);
///
/// // The client will automatically fetch the OAuth token for this tenant
/// let data = client.get_object(&tenant_id, SquareObjectType::Customer, "CUST123").await?;
/// ```
#[derive(Debug)]
pub struct OAuthSquareClient {
    client: reqwest::Client,
    token_provider: Arc<dyn OAuthTokenProvider>,
    base_url: String,
}

impl OAuthSquareClient {
    /// Create a new OAuth-integrated Square client.
    pub fn new(token_provider: Arc<dyn OAuthTokenProvider>) -> Self {
        let sandbox = std::env::var("SQUARE_SANDBOX")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let base_url = if sandbox {
            SQUARE_SANDBOX_API_BASE
        } else {
            SQUARE_API_BASE
        }
        .to_string();

        Self {
            client: reqwest::Client::new(),
            token_provider,
            base_url,
        }
    }

    /// Build request headers with the provided access token.
    fn build_headers(access_token: &str) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {access_token}")
                .parse()
                .expect("Invalid access token"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            USER_AGENT.parse().expect("Invalid user agent"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().expect("Invalid content type"),
        );
        headers.insert(
            "Square-Version",
            "2024-01-18".parse().expect("Invalid Square version"),
        );
        headers
    }

    /// Get a valid access token for the tenant.
    async fn get_access_token(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<String> {
        let token = self
            .token_provider
            .get_token(tenant_id, "square")
            .await?
            .ok_or_else(|| {
                errors::Error::unauthorized(
                    "Square is not connected for this tenant. Please complete \
                     OAuth authorization first.",
                )
            })?;

        if token.is_expired() {
            return Err(errors::Error::unauthorized(
                "Square OAuth token has expired. Please reconnect.",
            ));
        }

        Ok(token.access_token)
    }

    /// Execute a GET request with retry logic.
    async fn get_with_retry(
        &self,
        url: &str,
        access_token: &str,
    ) -> errors::Result<reqwest::Response> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            let response = self
                .client
                .get(url)
                .headers(Self::build_headers(access_token))
                .send()
                .await
                .map_err(|e| {
                    errors::Error::internal_server_error(format!(
                        "HTTP request failed: {e}"
                    ))
                })?;

            let status = response.status();

            // Handle rate limiting (429)
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(60);

                let wait = Duration::from_secs(retry_after.min(300));
                tracing::warn!(
                    attempt = attempt + 1,
                    wait_secs = wait.as_secs(),
                    "Square API rate limited, waiting before retry"
                );
                sleep(wait).await;
                continue;
            }

            // Handle server errors
            if status.is_server_error() {
                let backoff = SquareApiClient::calculate_backoff(attempt);
                tracing::warn!(
                    attempt = attempt + 1,
                    status = %status,
                    backoff_ms = backoff.as_millis() as u64,
                    "Square API server error, retrying with backoff"
                );
                last_error = Some(format!("Server error: {status}"));
                sleep(backoff).await;
                continue;
            }

            return Ok(response);
        }

        Err(errors::Error::internal_server_error(
            last_error
                .unwrap_or_else(|| "Max retries exceeded".to_string()),
        ))
    }
}

#[async_trait]
impl SquareClient for OAuthSquareClient {
    async fn get_object(
        &self,
        tenant_id: &TenantId,
        object_type: SquareObjectType,
        object_id: &str,
    ) -> errors::Result<serde_json::Value> {
        let access_token = self.get_access_token(tenant_id).await?;
        let endpoint =
            SquareApiClient::object_endpoint(object_type, object_id);
        let url = format!("{}/{endpoint}", self.base_url);

        tracing::debug!(
            url = %url,
            tenant_id = %tenant_id,
            "Fetching Square object with OAuth token"
        );

        let response = self.get_with_retry(&url, &access_token).await?;
        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(errors::Error::not_found(format!(
                "Square {} not found: {}",
                object_type.as_str(),
                object_id
            )));
        }

        if !status.is_success() {
            let error: SquareErrorResponse =
                response.json().await.unwrap_or(SquareErrorResponse {
                    errors: vec![SquareError {
                        category: "UNKNOWN".to_string(),
                        code: "UNKNOWN_ERROR".to_string(),
                        detail: Some("Unknown error".to_string()),
                    }],
                });
            let error_message = error
                .errors
                .first()
                .and_then(|e| e.detail.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(errors::Error::internal_server_error(format!(
                "Square API error: {error_message}"
            )));
        }

        let response_body: serde_json::Value =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse Square response: {e}"
                ))
            })?;

        Ok(response_body)
    }

    async fn batch_retrieve_catalog_objects(
        &self,
        tenant_id: &TenantId,
        object_ids: &[String],
    ) -> errors::Result<Vec<serde_json::Value>> {
        let access_token = self.get_access_token(tenant_id).await?;
        let url = format!("{}/catalog/batch-retrieve", self.base_url);

        let body = serde_json::json!({
            "object_ids": object_ids,
            "include_related_objects": true
        });

        tracing::debug!(
            url = %url,
            tenant_id = %tenant_id,
            object_count = object_ids.len(),
            "Batch retrieving Square catalog objects with OAuth token"
        );

        let response = self
            .client
            .post(&url)
            .headers(Self::build_headers(&access_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "HTTP request failed: {e}"
                ))
            })?;

        let status = response.status();

        if !status.is_success() {
            let error: SquareErrorResponse =
                response.json().await.unwrap_or(SquareErrorResponse {
                    errors: vec![SquareError {
                        category: "UNKNOWN".to_string(),
                        code: "UNKNOWN_ERROR".to_string(),
                        detail: Some("Unknown error".to_string()),
                    }],
                });
            let error_message = error
                .errors
                .first()
                .and_then(|e| e.detail.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(errors::Error::internal_server_error(format!(
                "Square API error: {error_message}"
            )));
        }

        let response_body: serde_json::Value =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse Square response: {e}"
                ))
            })?;

        // Extract objects from response
        let objects = response_body
            .get("objects")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(objects)
    }

    async fn list_catalog_items(
        &self,
        tenant_id: &TenantId,
        cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)> {
        let access_token = self.get_access_token(tenant_id).await?;
        let mut url = format!("{}/catalog/list?types=ITEM", self.base_url);
        if let Some(c) = cursor {
            url = format!("{url}&cursor={c}");
        }

        let response = self.get_with_retry(&url, &access_token).await?;

        SquareApiClient::parse_list_response(response, "objects").await
    }

    async fn list_customers(
        &self,
        tenant_id: &TenantId,
        cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)> {
        let access_token = self.get_access_token(tenant_id).await?;
        let mut url = format!("{}/customers", self.base_url);
        if let Some(c) = cursor {
            url = format!("{url}?cursor={c}");
        }

        let response = self.get_with_retry(&url, &access_token).await?;
        SquareApiClient::parse_list_response(response, "customers").await
    }

    async fn list_orders(
        &self,
        tenant_id: &TenantId,
        location_ids: &[String],
        cursor: Option<&str>,
    ) -> errors::Result<(Vec<serde_json::Value>, Option<String>)> {
        let access_token = self.get_access_token(tenant_id).await?;
        let url = format!("{}/orders/search", self.base_url);

        let mut body = serde_json::json!({
            "location_ids": location_ids
        });
        if let Some(c) = cursor {
            body["cursor"] = serde_json::Value::String(c.to_string());
        }

        let response = self
            .client
            .post(&url)
            .headers(Self::build_headers(&access_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "HTTP request failed: {e}"
                ))
            })?;

        SquareApiClient::parse_list_response(response, "orders").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SquareApiClient::new("sq0atp-xxx".to_string());
        assert!(!client.access_token.is_empty());
    }

    #[test]
    fn test_object_endpoint() {
        assert_eq!(
            SquareApiClient::object_endpoint(
                SquareObjectType::CatalogItem,
                "ITEM123"
            ),
            "catalog/object/ITEM123"
        );
        assert_eq!(
            SquareApiClient::object_endpoint(
                SquareObjectType::Customer,
                "CUST123"
            ),
            "customers/CUST123"
        );
        assert_eq!(
            SquareApiClient::object_endpoint(
                SquareObjectType::Order,
                "ORDER123"
            ),
            "orders/ORDER123"
        );
        assert_eq!(
            SquareApiClient::object_endpoint(
                SquareObjectType::Payment,
                "PAY123"
            ),
            "payments/PAY123"
        );
    }

    #[test]
    fn test_backoff_calculation() {
        assert_eq!(
            SquareApiClient::calculate_backoff(0),
            Duration::from_millis(1000)
        );
        assert_eq!(
            SquareApiClient::calculate_backoff(1),
            Duration::from_millis(2000)
        );
        assert_eq!(
            SquareApiClient::calculate_backoff(5),
            Duration::from_millis(30000)
        );
    }
}
