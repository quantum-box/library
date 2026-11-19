//! Use case for retrying failed webhook events.

use std::sync::Arc;

use inbound_sync_domain::{
    ProcessingStatus, WebhookEndpointRepository, WebhookEvent,
    WebhookEventId, WebhookEventRepository,
};
use value_object::TenantId;

/// Use case for retrying failed webhook events.
pub struct RetryWebhookEvent {
    event_repo: Arc<dyn WebhookEventRepository>,
    endpoint_repo: Arc<dyn WebhookEndpointRepository>,
}

impl RetryWebhookEvent {
    /// Create a new RetryWebhookEvent use case.
    pub fn new(
        event_repo: Arc<dyn WebhookEventRepository>,
        endpoint_repo: Arc<dyn WebhookEndpointRepository>,
    ) -> Self {
        Self {
            event_repo,
            endpoint_repo,
        }
    }

    /// Retry a failed webhook event.
    ///
    /// This resets the event status to PENDING so it can be picked up by
    /// the event processor again.
    pub async fn execute(
        &self,
        tenant_id: &TenantId,
        event_id: &WebhookEventId,
    ) -> errors::Result<WebhookEvent> {
        // Get the event
        let mut event = self
            .event_repo
            .find_by_id(event_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Webhook event"))?;

        // Events carry no tenant of their own; ownership comes from the
        // endpoint that received them. Another tenant's event is reported as
        // missing.
        let owned = self
            .endpoint_repo
            .find_by_id(event.endpoint_id())
            .await?
            .is_some_and(|endpoint| endpoint.tenant_id() == tenant_id);
        if !owned {
            return Err(errors::Error::not_found("Webhook event"));
        }

        // Check if event can be retried
        if *event.status() != ProcessingStatus::Failed {
            return Err(errors::Error::invalid(format!(
                "Cannot retry event with status {:?}",
                event.status()
            )));
        }

        // A failed event cannot be reintroduced into the pending queue while
        // its provider is unavailable in this runtime.
        ensure_retry_provider_available(&event)?;

        // Schedule a retry (allows up to 10 retries)
        if !event.schedule_retry(10) {
            return Err(errors::Error::invalid("Maximum retries exceeded"));
        }

        // Save the updated event
        self.event_repo.save(&event).await?;

        tracing::info!(
            event_id = %event_id,
            retry_count = event.retry_count(),
            "Webhook event queued for retry"
        );

        Ok(event)
    }
}

fn ensure_retry_provider_available(
    event: &WebhookEvent,
) -> errors::Result<()> {
    event.provider().ensure_runtime_available()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_provider_cannot_be_requeued() {
        let mut event = WebhookEvent::create(
            inbound_sync_domain::WebhookEndpointId::default(),
            inbound_sync_domain::Provider::Hubspot,
            "contact.creation",
            serde_json::json!({}),
            None,
            true,
        );
        event.mark_failed("provider disabled");

        assert!(ensure_retry_provider_available(&event).is_err());
        assert_eq!(*event.status(), ProcessingStatus::Failed);
        assert_eq!(*event.retry_count(), 0);
    }
}
