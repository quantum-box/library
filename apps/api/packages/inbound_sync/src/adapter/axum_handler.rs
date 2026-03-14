//! Axum HTTP handler for webhook endpoints.

use axum::{
    body::Bytes,
    extract::{OriginalUri, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use inbound_sync_domain::{Provider, WebhookEndpointId};

use crate::usecase::{ReceiveProviderWebhook, ReceiveWebhook};

/// State for webhook handlers.
#[derive(Clone)]
pub struct WebhookHandlerState {
    pub receive_webhook: Arc<ReceiveWebhook>,
    pub receive_provider_webhook: Arc<ReceiveProviderWebhook>,
    /// API base URL (e.g. `https://library.api.n1.tachy.one`).
    /// Used to construct the full webhook notification URL for
    /// providers like Square that include it in signature
    /// verification.
    pub base_url: Option<String>,
}

/// Response for webhook reception.
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub event_id: String,
    pub status: String,
}

/// Response for provider webhook reception.
#[derive(Debug, Serialize)]
pub struct WebhookBatchResponse {
    pub event_ids: Vec<String>,
    pub status: String,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Path parameters for webhook endpoint.
#[derive(Debug, Deserialize)]
pub struct WebhookPath {
    pub provider: String,
    pub endpoint_id: String,
}

/// Path parameters for provider-only webhook endpoint.
#[derive(Debug, Deserialize)]
pub struct ProviderPath {
    pub provider: String,
}

/// Create the webhook router.
///
/// # Routes
///
/// - `POST /webhooks/:provider` - Receive a provider-only webhook
/// - `POST /webhooks/:provider/:endpoint_id` - Receive a webhook
///
/// # Example
///
/// ```ignore
/// let state = WebhookHandlerState {
///     receive_webhook: Arc::new(receive_webhook_usecase),
/// };
/// let router = create_webhook_router(state);
/// ```
pub fn create_webhook_router(state: WebhookHandlerState) -> Router {
    Router::new()
        .route("/webhooks/:provider", post(handle_provider_webhook))
        .route("/webhooks/:provider/:endpoint_id", post(handle_webhook))
        .with_state(state)
}

/// Handle incoming webhook for provider-only endpoint.
async fn handle_provider_webhook(
    State(state): State<WebhookHandlerState>,
    Path(path): Path<ProviderPath>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let provider: Provider = match path.provider.parse() {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_provider".to_string(),
                    message: format!("Unknown provider: {}", path.provider),
                }),
            )
                .into_response();
        }
    };

    // Get signature from headers
    let signature_header = provider.signature_header();
    let signature = headers
        .get(signature_header)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if signature.is_empty() {
        tracing::warn!(
            provider = %provider,
            "Webhook received without signature header"
        );
    }

    // Get event type from header (if available)
    let event_type = provider
        .event_header()
        .and_then(|h| headers.get(h))
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Collect headers as JSON
    let headers_json = headers_to_json(&headers);

    let webhook_url = state
        .base_url
        .as_ref()
        .map(|base| format!("{}{}", base, uri.path()));

    let input = crate::usecase::ReceiveProviderWebhookInput {
        provider,
        payload: body.to_vec(),
        signature,
        event_type,
        headers: Some(headers_json),
        webhook_url,
    };

    match state.receive_provider_webhook.execute(input).await {
        Ok(output) => (
            StatusCode::OK,
            Json(WebhookBatchResponse {
                event_ids: output
                    .event_ids
                    .into_iter()
                    .map(|id| id.to_string())
                    .collect(),
                status: "queued".to_string(),
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to receive provider webhook");

            let (status_code, error_code) = if e.is_not_found() {
                (StatusCode::NOT_FOUND, "endpoint_not_found")
            } else if e.is_forbidden() {
                (StatusCode::FORBIDDEN, "forbidden")
            } else if e.is_bad_request() {
                (StatusCode::BAD_REQUEST, "bad_request")
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            };

            (
                status_code,
                Json(ErrorResponse {
                    error: error_code.to_string(),
                    message: e.to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// Handle incoming webhook.
///
/// This handler:
/// 1. Parses the provider and endpoint ID from the URL
/// 2. Extracts the signature from headers
/// 3. Passes to the ReceiveWebhook usecase
/// 4. Returns quickly to give the sender a fast response
async fn handle_webhook(
    State(state): State<WebhookHandlerState>,
    Path(path): Path<WebhookPath>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Parse provider
    let provider: Provider = match path.provider.parse() {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_provider".to_string(),
                    message: format!("Unknown provider: {}", path.provider),
                }),
            )
                .into_response();
        }
    };

    // Parse endpoint ID
    let endpoint_id = WebhookEndpointId::from(path.endpoint_id);

    // Get signature from headers
    let signature_header = provider.signature_header();
    let signature = headers
        .get(signature_header)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if signature.is_empty() {
        tracing::warn!(
            endpoint_id = %endpoint_id,
            provider = %provider,
            "Webhook received without signature header"
        );
    }

    // Get event type from header (if available)
    let event_type = provider
        .event_header()
        .and_then(|h| headers.get(h))
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Collect headers as JSON
    let headers_json = headers_to_json(&headers);

    // Execute usecase
    let webhook_url = state
        .base_url
        .as_ref()
        .map(|base| format!("{}{}", base, uri.path()));

    let input = crate::usecase::ReceiveWebhookInput {
        endpoint_id,
        provider,
        payload: body.to_vec(),
        signature,
        event_type,
        headers: Some(headers_json),
        webhook_url,
    };

    match state.receive_webhook.execute(input).await {
        Ok(output) => {
            let status = if output.signature_valid {
                "queued"
            } else {
                "queued_unverified"
            };

            (
                StatusCode::OK,
                Json(WebhookResponse {
                    event_id: output.event_id.to_string(),
                    status: status.to_string(),
                }),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to receive webhook");

            let (status_code, error_code) = if e.is_not_found() {
                (StatusCode::NOT_FOUND, "endpoint_not_found")
            } else if e.is_forbidden() {
                (StatusCode::FORBIDDEN, "forbidden")
            } else if e.is_bad_request() {
                (StatusCode::BAD_REQUEST, "bad_request")
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            };

            (
                status_code,
                Json(ErrorResponse {
                    error: error_code.to_string(),
                    message: e.to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// Convert headers to JSON for storage.
fn headers_to_json(headers: &HeaderMap) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (key, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            map.insert(
                key.as_str().to_string(),
                serde_json::Value::String(v.to_string()),
            );
        }
    }
    serde_json::Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headers_to_json() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert(
            "x-hub-signature-256",
            "sha256=abc123".parse().unwrap(),
        );

        let json = headers_to_json(&headers);
        assert!(json.get("content-type").is_some());
        assert!(json.get("x-hub-signature-256").is_some());
    }
}
