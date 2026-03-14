//! Receive and queue webhook events use case.

use std::sync::Arc;

use inbound_sync_domain::{
    Provider, WebhookEndpoint, WebhookEndpointId,
    WebhookEndpointRepository, WebhookEvent, WebhookEventId,
    WebhookEventRepository,
};

use crate::{WebhookSecretStore, WebhookVerifierRegistry};

/// Input for receiving a webhook.
pub struct ReceiveWebhookInput {
    /// Endpoint ID from URL
    pub endpoint_id: WebhookEndpointId,
    /// Provider from URL
    pub provider: Provider,
    /// Raw payload bytes
    pub payload: Vec<u8>,
    /// Signature header value
    pub signature: String,
    /// Event type (from header if available)
    pub event_type: Option<String>,
    /// All headers as JSON
    pub headers: Option<serde_json::Value>,
    /// Full webhook notification URL (used by Square for signature
    /// verification)
    pub webhook_url: Option<String>,
}

/// Output for receive webhook.
pub struct ReceiveWebhookOutput {
    /// Created event ID
    pub event_id: WebhookEventId,
    /// Whether signature was valid
    pub signature_valid: bool,
}

/// Receive webhook use case.
///
/// This use case handles incoming webhooks:
/// 1. Verifies the endpoint exists and is active
/// 2. Verifies the webhook signature
/// 3. Queues the event for background processing
///
/// The actual data processing happens asynchronously via ProcessWebhookEvent.
pub struct ReceiveWebhook {
    endpoint_repository: Arc<dyn WebhookEndpointRepository>,
    event_repository: Arc<dyn WebhookEventRepository>,
    verifier_registry: Arc<WebhookVerifierRegistry>,
    provider_secrets: Arc<WebhookSecretStore>,
}

impl ReceiveWebhook {
    pub fn new(
        endpoint_repository: Arc<dyn WebhookEndpointRepository>,
        event_repository: Arc<dyn WebhookEventRepository>,
        verifier_registry: Arc<WebhookVerifierRegistry>,
        provider_secrets: Arc<WebhookSecretStore>,
    ) -> Self {
        Self {
            endpoint_repository,
            event_repository,
            verifier_registry,
            provider_secrets,
        }
    }

    /// Execute the use case.
    ///
    /// Returns quickly to give the webhook sender a fast response.
    /// Actual processing happens in the background.
    pub async fn execute(
        &self,
        input: ReceiveWebhookInput,
    ) -> errors::Result<ReceiveWebhookOutput> {
        // 1. Find the endpoint
        let endpoint = self
            .endpoint_repository
            .find_by_id(&input.endpoint_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Webhook endpoint"))?;

        // 2. Verify provider matches
        if *endpoint.provider() != input.provider {
            return Err(errors::Error::invalid(format!(
                "Provider mismatch: expected {}, got {}",
                endpoint.provider(),
                input.provider
            )));
        }

        // 3. Check endpoint is not disabled
        if *endpoint.status()
            == inbound_sync_domain::EndpointStatus::Disabled
        {
            return Err(errors::Error::invalid(
                "Webhook endpoint is disabled",
            ));
        }

        // 4. Verify signature
        let secret =
            resolve_webhook_secret(&endpoint, &self.provider_secrets);
        let signature_valid = self.verifier_registry.verify(
            input.provider,
            &input.payload,
            &input.signature,
            secret,
            input.webhook_url.as_deref(),
        )?;

        // 5. Parse payload as JSON
        let payload: serde_json::Value =
            serde_json::from_slice(&input.payload).map_err(|e| {
                errors::Error::invalid(format!("Invalid JSON payload: {e}"))
            })?;

        // 6. Determine event type
        let event_type = input
            .event_type
            .or_else(|| extract_event_type(&payload, input.provider))
            .unwrap_or_else(|| "unknown".to_string());

        // 7. Check if endpoint is interested in this event
        if !endpoint.should_process_event(&event_type) {
            tracing::debug!(
                endpoint_id = %input.endpoint_id,
                event_type = %event_type,
                "Event type not in endpoint's event list, skipping"
            );

            // Still create the event but mark it as skipped
            let mut event = WebhookEvent::create(
                input.endpoint_id,
                input.provider,
                event_type,
                payload,
                input.headers,
                signature_valid,
            );
            event.mark_skipped("Event type not in configured events list");
            self.event_repository.save(&event).await?;

            return Ok(ReceiveWebhookOutput {
                event_id: event.id().clone(),
                signature_valid,
            });
        }

        // 8. Create and queue the event
        let event = WebhookEvent::create(
            input.endpoint_id.clone(),
            input.provider,
            event_type.clone(),
            payload,
            input.headers,
            signature_valid,
        );

        let event_id = event.id().clone();
        self.event_repository.save(&event).await?;

        tracing::info!(
            endpoint_id = %input.endpoint_id,
            event_id = %event_id,
            event_type = %event_type,
            signature_valid = signature_valid,
            "Webhook event queued for processing"
        );

        Ok(ReceiveWebhookOutput {
            event_id,
            signature_valid,
        })
    }
}

/// Extract event type from payload based on provider.
pub(crate) fn extract_event_type(
    payload: &serde_json::Value,
    provider: Provider,
) -> Option<String> {
    match provider {
        Provider::Github => {
            // GitHub sends event type in header, but also in payload for some events
            payload
                .get("action")
                .and_then(|a| a.as_str())
                .map(String::from)
        }
        Provider::Linear => {
            // Linear: { "type": "Issue", "action": "update" }
            // Use the event type directly so it matches configured filters.
            payload
                .get("type")
                .and_then(|t| t.as_str())
                .map(String::from)
        }
        Provider::Hubspot => {
            // HubSpot: subscriptionType field
            payload
                .get("subscriptionType")
                .and_then(|s| s.as_str())
                .map(String::from)
        }
        Provider::Stripe => {
            // Stripe: { "type": "product.created" }
            payload
                .get("type")
                .and_then(|t| t.as_str())
                .map(String::from)
        }
        Provider::Square => {
            // Square: { "type": "catalog.version.updated" }
            payload
                .get("type")
                .and_then(|t| t.as_str())
                .map(String::from)
        }
        Provider::Notion => {
            // Notion: { "type": "page.created" }
            payload
                .get("type")
                .and_then(|t| t.as_str())
                .map(String::from)
        }
        Provider::Airtable => {
            // Airtable: { "webhook": { "eventType": "RECORD_CREATED" } }
            payload
                .get("webhook")
                .and_then(|w| w.get("eventType"))
                .and_then(|e| e.as_str())
                .map(|s| s.to_lowercase())
        }
        Provider::Generic => {
            // Generic: try common fields
            payload
                .get("event")
                .or_else(|| payload.get("type"))
                .or_else(|| payload.get("event_type"))
                .and_then(|e| e.as_str())
                .map(String::from)
        }
    }
}

pub(crate) fn resolve_webhook_secret<'a>(
    endpoint: &'a WebhookEndpoint,
    provider_secrets: &'a WebhookSecretStore,
) -> &'a str {
    provider_secrets
        .get(*endpoint.provider())
        .or_else(|| endpoint.webhook_secret())
        .unwrap_or_else(|| endpoint.secret_hash())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_event_type_github() {
        let payload = serde_json::json!({
            "action": "opened",
            "issue": {}
        });
        assert_eq!(
            extract_event_type(&payload, Provider::Github),
            Some("opened".to_string())
        );
    }

    #[test]
    fn test_extract_event_type_linear() {
        let payload = serde_json::json!({
            "type": "Issue",
            "action": "update"
        });
        assert_eq!(
            extract_event_type(&payload, Provider::Linear),
            Some("Issue".to_string())
        );
    }

    #[test]
    fn test_extract_event_type_stripe() {
        let payload = serde_json::json!({
            "id": "evt_xxx",
            "type": "product.created"
        });
        assert_eq!(
            extract_event_type(&payload, Provider::Stripe),
            Some("product.created".to_string())
        );
    }
}
