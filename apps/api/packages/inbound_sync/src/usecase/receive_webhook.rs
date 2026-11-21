//! Receive and queue webhook events use case.

use std::sync::Arc;

use inbound_sync_domain::{
    Provider, WebhookEndpoint, WebhookEndpointId,
    WebhookEndpointRepository, WebhookEvent, WebhookEventId,
    WebhookEventRepository,
};

use crate::{WebhookSecretStore, WebhookVerifierRegistry};

use super::WebhookDispatcher;

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
    dispatcher: Option<Arc<dyn WebhookDispatcher>>,
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
            dispatcher: None,
        }
    }

    /// Wake a durable consumer after a valid event is committed.
    pub fn with_dispatcher(
        mut self,
        dispatcher: Arc<dyn WebhookDispatcher>,
    ) -> Self {
        self.dispatcher = Some(dispatcher);
        self
    }

    /// Execute the use case.
    ///
    /// Returns quickly to give the webhook sender a fast response.
    /// Actual processing happens in the background.
    pub async fn execute(
        &self,
        input: ReceiveWebhookInput,
    ) -> errors::Result<ReceiveWebhookOutput> {
        if !input.provider.is_runtime_available() {
            return Err(errors::Error::service_unavailable(
                input.provider.unavailable_reason().unwrap_or(
                    "Webhook processing is not available in this runtime.",
                ),
            ));
        }
        if input.provider == Provider::Github && self.dispatcher.is_none() {
            return Err(errors::Error::service_unavailable(
                "GitHub continuous sync requires a durable dispatcher",
            ));
        }

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
        // Every endpoint has a key (the provider-wide secret, the endpoint's
        // own secret, or its generated signing key), so an unverified
        // request is never one a configured provider sent. Refuse it
        // without storing anything: an unsigned row could never be
        // processed, and keeping it would let anyone who knows the endpoint
        // id fill the event table.
        if !signature_valid {
            tracing::warn!(
                endpoint_id = %input.endpoint_id,
                provider = %input.provider,
                signature_present = !input.signature.is_empty(),
                "Webhook rejected: signature did not verify"
            );
            return Err(invalid_signature());
        }

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
                true,
            );
            event.mark_skipped("Event type not in configured events list");
            self.event_repository.save(&event).await?;

            return Ok(ReceiveWebhookOutput {
                event_id: event.id().clone(),
            });
        }

        // 8. Create and queue the event
        let event = WebhookEvent::create(
            input.endpoint_id.clone(),
            input.provider,
            event_type.clone(),
            payload,
            input.headers,
            true,
        );

        let event_id = event.id().clone();
        self.event_repository.save(&event).await?;

        if let Some(dispatcher) = &self.dispatcher {
            dispatcher.dispatch(&event_id).await?;
        }

        tracing::info!(
            endpoint_id = %input.endpoint_id,
            event_id = %event_id,
            event_type = %event_type,
            "Webhook event queued for processing"
        );

        Ok(ReceiveWebhookOutput { event_id })
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

/// Error returned for a webhook whose signature is missing or wrong.
pub(crate) fn invalid_signature() -> errors::Error {
    errors::Error::unauthorized("Webhook signature is missing or invalid")
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
    use hmac::{Hmac, Mac};
    use inbound_sync_domain::{
        ProcessingStats, ProcessingStatus, ProviderConfig,
    };
    use std::sync::Mutex;
    use value_object::TenantId;

    const KEY: &str = "endpoint-signing-key";

    #[derive(Debug)]
    struct OneEndpoint(WebhookEndpoint);

    #[async_trait::async_trait]
    impl WebhookEndpointRepository for OneEndpoint {
        async fn save(&self, _: &WebhookEndpoint) -> errors::Result<()> {
            unreachable!()
        }
        async fn find_by_id(
            &self,
            id: &WebhookEndpointId,
        ) -> errors::Result<Option<WebhookEndpoint>> {
            Ok((self.0.id() == id).then(|| self.0.clone()))
        }
        async fn find_by_tenant(
            &self,
            _: &TenantId,
        ) -> errors::Result<Vec<WebhookEndpoint>> {
            unreachable!()
        }
        async fn find_by_tenant_and_provider(
            &self,
            _: &TenantId,
            _: Provider,
        ) -> errors::Result<Vec<WebhookEndpoint>> {
            unreachable!()
        }
        async fn delete(
            &self,
            _: &WebhookEndpointId,
        ) -> errors::Result<()> {
            unreachable!()
        }
    }

    #[derive(Debug, Default)]
    struct SavedEvents(Mutex<Vec<WebhookEvent>>);

    #[async_trait::async_trait]
    impl WebhookEventRepository for SavedEvents {
        async fn save(&self, event: &WebhookEvent) -> errors::Result<()> {
            self.0.lock().unwrap().push(event.clone());
            Ok(())
        }
        async fn find_by_id(
            &self,
            _: &WebhookEventId,
        ) -> errors::Result<Option<WebhookEvent>> {
            unreachable!()
        }
        async fn find_by_endpoint(
            &self,
            _: &WebhookEndpointId,
            _: u32,
            _: u32,
        ) -> errors::Result<Vec<WebhookEvent>> {
            unreachable!()
        }
        async fn find_pending_events(
            &self,
            _: u32,
        ) -> errors::Result<Vec<WebhookEvent>> {
            unreachable!()
        }
        async fn find_by_status(
            &self,
            _: &WebhookEndpointId,
            _: ProcessingStatus,
            _: u32,
        ) -> errors::Result<Vec<WebhookEvent>> {
            unreachable!()
        }
        async fn update_status(
            &self,
            _: &WebhookEventId,
            _: ProcessingStatus,
            _: Option<String>,
            _: Option<ProcessingStats>,
        ) -> errors::Result<()> {
            unreachable!()
        }
        async fn delete_older_than(
            &self,
            _: chrono::DateTime<chrono::Utc>,
        ) -> errors::Result<u64> {
            unreachable!()
        }
    }

    fn generic_endpoint() -> WebhookEndpoint {
        WebhookEndpoint::create(
            "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap(),
            "generic",
            Provider::Generic,
            ProviderConfig::Generic {
                signature_header: "x-webhook-signature".into(),
                signature_algorithm: "hmac-sha256".into(),
            },
            vec![],
            KEY,
        )
    }

    fn sign(payload: &[u8]) -> String {
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(KEY.as_bytes()).unwrap();
        mac.update(payload);
        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    }

    async fn receive(
        signature: String,
    ) -> (errors::Result<ReceiveWebhookOutput>, Vec<WebhookEvent>) {
        let endpoint = generic_endpoint();
        let endpoint_id = endpoint.id().clone();
        let events = Arc::new(SavedEvents::default());
        let usecase = ReceiveWebhook::new(
            Arc::new(OneEndpoint(endpoint)),
            events.clone(),
            Arc::new(WebhookVerifierRegistry::new()),
            Arc::new(WebhookSecretStore::new()),
        );
        let payload = br#"{"event":"item.updated"}"#.to_vec();
        let signature = if signature == "valid" {
            sign(&payload)
        } else {
            signature
        };
        let result = usecase
            .execute(ReceiveWebhookInput {
                endpoint_id,
                provider: Provider::Generic,
                payload,
                signature,
                event_type: None,
                headers: None,
                webhook_url: None,
            })
            .await;
        let saved = events.0.lock().unwrap().clone();
        (result, saved)
    }

    #[tokio::test]
    async fn missing_signature_is_refused_and_not_stored() {
        let (result, saved) = receive(String::new()).await;
        let err = result.err().expect("unsigned webhook must be refused");
        assert!(
            matches!(err, errors::Error::Unauthorized { .. }),
            "{err:?}"
        );
        assert!(saved.is_empty(), "unsigned payload was stored");
    }

    #[tokio::test]
    async fn wrong_signature_is_refused_and_not_stored() {
        let (result, saved) =
            receive(format!("sha256={}", "0".repeat(64))).await;
        let err = result.err().expect("forged webhook must be refused");
        assert!(
            matches!(err, errors::Error::Unauthorized { .. }),
            "{err:?}"
        );
        assert!(saved.is_empty(), "forged payload was stored");
    }

    #[tokio::test]
    async fn signed_webhook_is_queued() {
        let (result, saved) = receive("valid".into()).await;
        let output = result.unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].id(), &output.event_id);
        assert!(*saved[0].signature_valid());
    }

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
