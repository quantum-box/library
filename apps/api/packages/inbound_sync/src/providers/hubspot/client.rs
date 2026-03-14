//! HubSpot CRM API client implementation.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

use super::event_processor::HubSpotClient;
use super::payload::{HubSpotObject, ObjectType};

const HUBSPOT_API_BASE: &str = "https://api.hubapi.com";
const USER_AGENT: &str = "inbound-sync/0.1.0";

/// Rate limiting configuration
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;
const MAX_BACKOFF_MS: u64 = 30000;

/// HubSpot CRM API client implementation.
#[derive(Debug)]
pub struct HubSpotApiClient {
    client: reqwest::Client,
    access_token: String,
}

impl HubSpotApiClient {
    /// Create a new HubSpot API client.
    pub fn new(access_token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            access_token,
        }
    }

    /// Build request headers.
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.access_token)
                .parse()
                .expect("Invalid token"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            USER_AGENT.parse().expect("Invalid user agent"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().expect("Invalid content type"),
        );
        headers
    }

    /// Calculate exponential backoff delay.
    fn calculate_backoff(attempt: u32) -> Duration {
        let delay_ms = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
        Duration::from_millis(delay_ms.min(MAX_BACKOFF_MS))
    }

    /// Get the API endpoint for an object type.
    fn object_endpoint(object_type: ObjectType) -> &'static str {
        match object_type {
            ObjectType::Contact => "crm/v3/objects/contacts",
            ObjectType::Company => "crm/v3/objects/companies",
            ObjectType::Deal => "crm/v3/objects/deals",
            ObjectType::Product => "crm/v3/objects/products",
            ObjectType::Ticket => "crm/v3/objects/tickets",
            ObjectType::LineItem => "crm/v3/objects/line_items",
        }
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
                    "HubSpot API rate limited, waiting before retry"
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
                    "HubSpot API server error, retrying with backoff"
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
struct HubSpotObjectResponse {
    id: String,
    properties: serde_json::Map<String, serde_json::Value>,
    #[allow(dead_code)]
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "updatedAt")]
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HubSpotErrorResponse {
    message: String,
    #[allow(dead_code)]
    correlation_id: Option<String>,
    #[allow(dead_code)]
    category: Option<String>,
}

impl From<HubSpotObjectResponse> for HubSpotObject {
    fn from(response: HubSpotObjectResponse) -> Self {
        HubSpotObject {
            id: response.id,
            properties: serde_json::Value::Object(response.properties),
            created_at: response.created_at.unwrap_or_default(),
            updated_at: response.updated_at.unwrap_or_default(),
            archived: false,
        }
    }
}

#[async_trait]
impl HubSpotClient for HubSpotApiClient {
    async fn get_object(
        &self,
        _tenant_id: &value_object::TenantId,
        object_type: ObjectType,
        object_id: i64,
    ) -> errors::Result<HubSpotObject> {
        let endpoint = Self::object_endpoint(object_type);
        let url = format!("{HUBSPOT_API_BASE}/{endpoint}/{object_id}");

        tracing::debug!(url = %url, "Fetching HubSpot object");

        let response = self.get_with_retry(&url).await?;
        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(errors::Error::not_found(format!(
                "HubSpot {} not found: {}",
                object_type.as_str(),
                object_id
            )));
        }

        if !status.is_success() {
            let error: HubSpotErrorResponse =
                response.json().await.unwrap_or(HubSpotErrorResponse {
                    message: "Unknown error".to_string(),
                    correlation_id: None,
                    category: None,
                });
            return Err(errors::Error::internal_server_error(format!(
                "HubSpot API error: {}",
                error.message
            )));
        }

        let response_body: HubSpotObjectResponse =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse HubSpot response: {e}"
                ))
            })?;

        Ok(response_body.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = HubSpotApiClient::new("test_token".to_string());
        assert!(!client.access_token.is_empty());
    }

    #[test]
    fn test_object_endpoint() {
        assert_eq!(
            HubSpotApiClient::object_endpoint(ObjectType::Contact),
            "crm/v3/objects/contacts"
        );
        assert_eq!(
            HubSpotApiClient::object_endpoint(ObjectType::Deal),
            "crm/v3/objects/deals"
        );
        assert_eq!(
            HubSpotApiClient::object_endpoint(ObjectType::Product),
            "crm/v3/objects/products"
        );
    }

    #[test]
    fn test_backoff_calculation() {
        assert_eq!(
            HubSpotApiClient::calculate_backoff(0),
            Duration::from_millis(1000)
        );
        assert_eq!(
            HubSpotApiClient::calculate_backoff(1),
            Duration::from_millis(2000)
        );
    }
}
