//! Stripe API client implementation.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

use super::event_processor::StripeClient;
use super::payload::StripeObjectType;

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";
const USER_AGENT: &str = "inbound-sync/0.1.0";

/// Rate limiting configuration
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;
const MAX_BACKOFF_MS: u64 = 30000;

/// Stripe API client implementation.
#[derive(Debug)]
pub struct StripeApiClient {
    client: reqwest::Client,
    secret_key: String,
}

impl StripeApiClient {
    /// Create a new Stripe API client.
    pub fn new(secret_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            secret_key,
        }
    }

    /// Build request headers.
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.secret_key)
                .parse()
                .expect("Invalid secret key"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            USER_AGENT.parse().expect("Invalid user agent"),
        );
        headers
    }

    /// Calculate exponential backoff delay.
    fn calculate_backoff(attempt: u32) -> Duration {
        let delay_ms = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
        Duration::from_millis(delay_ms.min(MAX_BACKOFF_MS))
    }

    /// Get the API endpoint for an object type.
    fn object_endpoint(object_type: StripeObjectType) -> &'static str {
        match object_type {
            StripeObjectType::Product => "products",
            StripeObjectType::Price => "prices",
            StripeObjectType::Customer => "customers",
            StripeObjectType::Subscription => "subscriptions",
            StripeObjectType::Invoice => "invoices",
            StripeObjectType::PaymentIntent => "payment_intents",
            StripeObjectType::Charge => "charges",
            StripeObjectType::Coupon => "coupons",
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
                    "Stripe API rate limited, waiting before retry"
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
                    "Stripe API server error, retrying with backoff"
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
struct StripeErrorResponse {
    error: StripeError,
}

#[derive(Debug, Deserialize)]
struct StripeError {
    message: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    error_type: Option<String>,
    #[allow(dead_code)]
    code: Option<String>,
}

#[async_trait]
impl StripeClient for StripeApiClient {
    async fn get_object(
        &self,
        _tenant_id: &value_object::TenantId,
        object_type: StripeObjectType,
        object_id: &str,
    ) -> errors::Result<serde_json::Value> {
        let endpoint = Self::object_endpoint(object_type);
        let url = format!("{STRIPE_API_BASE}/{endpoint}/{object_id}");

        tracing::debug!(url = %url, "Fetching Stripe object");

        let response = self.get_with_retry(&url).await?;
        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(errors::Error::not_found(format!(
                "Stripe {} not found: {}",
                object_type.as_str(),
                object_id
            )));
        }

        if !status.is_success() {
            let error: StripeErrorResponse =
                response.json().await.unwrap_or(StripeErrorResponse {
                    error: StripeError {
                        message: "Unknown error".to_string(),
                        error_type: None,
                        code: None,
                    },
                });
            return Err(errors::Error::internal_server_error(format!(
                "Stripe API error: {}",
                error.error.message
            )));
        }

        let response_body: serde_json::Value =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse Stripe response: {e}"
                ))
            })?;

        Ok(response_body)
    }

    async fn list_objects(
        &self,
        _tenant_id: &value_object::TenantId,
        object_type: StripeObjectType,
        limit: Option<u32>,
    ) -> errors::Result<Vec<serde_json::Value>> {
        let endpoint = Self::object_endpoint(object_type);
        let url = if let Some(lim) = limit {
            format!("{STRIPE_API_BASE}/{endpoint}?limit={lim}")
        } else {
            format!("{STRIPE_API_BASE}/{endpoint}")
        };

        tracing::debug!(url = %url, "Listing Stripe objects");

        let response = self.get_with_retry(&url).await?;

        #[derive(serde::Deserialize)]
        struct ListResponse {
            data: Vec<serde_json::Value>,
        }

        let list_response: ListResponse =
            response.json().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse Stripe list response: {e}"
                ))
            })?;

        Ok(list_response.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = StripeApiClient::new("sk_test_xxx".to_string());
        assert!(!client.secret_key.is_empty());
    }

    #[test]
    fn test_object_endpoint() {
        assert_eq!(
            StripeApiClient::object_endpoint(StripeObjectType::Product),
            "products"
        );
        assert_eq!(
            StripeApiClient::object_endpoint(StripeObjectType::Price),
            "prices"
        );
        assert_eq!(
            StripeApiClient::object_endpoint(StripeObjectType::Customer),
            "customers"
        );
        assert_eq!(
            StripeApiClient::object_endpoint(
                StripeObjectType::Subscription
            ),
            "subscriptions"
        );
    }

    #[test]
    fn test_backoff_calculation() {
        assert_eq!(
            StripeApiClient::calculate_backoff(0),
            Duration::from_millis(1000)
        );
        assert_eq!(
            StripeApiClient::calculate_backoff(1),
            Duration::from_millis(2000)
        );
    }
}
