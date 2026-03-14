//! Use case for retrying failed webhook events.

use std::sync::Arc;

use inbound_sync_domain::{
    ProcessingStatus, WebhookEvent, WebhookEventId, WebhookEventRepository,
};

/// Use case for retrying failed webhook events.
pub struct RetryWebhookEvent {
    event_repo: Arc<dyn WebhookEventRepository>,
}

impl RetryWebhookEvent {
    /// Create a new RetryWebhookEvent use case.
    pub fn new(event_repo: Arc<dyn WebhookEventRepository>) -> Self {
        Self { event_repo }
    }

    /// Retry a failed webhook event.
    ///
    /// This resets the event status to PENDING so it can be picked up by
    /// the event processor again.
    pub async fn execute(
        &self,
        event_id: &WebhookEventId,
    ) -> errors::Result<WebhookEvent> {
        // Get the event
        let mut event = self
            .event_repo
            .find_by_id(event_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Webhook event"))?;

        // Check if event can be retried
        if *event.status() != ProcessingStatus::Failed {
            return Err(errors::Error::invalid(format!(
                "Cannot retry event with status {:?}",
                event.status()
            )));
        }

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
