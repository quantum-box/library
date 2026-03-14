//! Process queued webhook events use case.

use std::sync::Arc;

use inbound_sync_domain::{
    ProcessingStats, ProcessingStatus, WebhookEndpointRepository,
    WebhookEvent, WebhookEventId, WebhookEventRepository,
};

/// Trait for provider-specific event processors.
///
/// Each provider (GitHub, Linear, HubSpot, etc.) implements this trait
/// to handle their specific webhook payloads and API interactions.
#[async_trait::async_trait]
pub trait EventProcessor: Send + Sync + std::fmt::Debug {
    /// Get the provider this processor handles.
    fn provider(&self) -> inbound_sync_domain::Provider;

    /// Process a webhook event.
    ///
    /// This method should:
    /// 1. Parse the webhook payload
    /// 2. Fetch detailed data from the provider's API
    /// 3. Transform the data using property mappings
    /// 4. Upsert/delete data in Library
    ///
    /// Returns processing statistics.
    async fn process(
        &self,
        event: &WebhookEvent,
        endpoint: &inbound_sync_domain::WebhookEndpoint,
    ) -> errors::Result<ProcessingStats>;
}

/// Registry of event processors for all providers.
#[derive(Debug, Default)]
pub struct EventProcessorRegistry {
    processors: std::collections::HashMap<
        inbound_sync_domain::Provider,
        Arc<dyn EventProcessor>,
    >,
}

impl EventProcessorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a processor for a provider.
    pub fn register(&mut self, processor: Arc<dyn EventProcessor>) {
        self.processors.insert(processor.provider(), processor);
    }

    /// Get processor for a provider.
    pub fn get(
        &self,
        provider: inbound_sync_domain::Provider,
    ) -> Option<Arc<dyn EventProcessor>> {
        self.processors.get(&provider).cloned()
    }
}

/// Process webhook event use case.
///
/// Processes a single queued webhook event by:
/// 1. Fetching the event and endpoint details
/// 2. Delegating to the appropriate provider processor
/// 3. Updating the event status
pub struct ProcessWebhookEvent {
    endpoint_repository: Arc<dyn WebhookEndpointRepository>,
    event_repository: Arc<dyn WebhookEventRepository>,
    processor_registry: Arc<EventProcessorRegistry>,
    max_retries: u32,
}

impl ProcessWebhookEvent {
    pub fn new(
        endpoint_repository: Arc<dyn WebhookEndpointRepository>,
        event_repository: Arc<dyn WebhookEventRepository>,
        processor_registry: Arc<EventProcessorRegistry>,
    ) -> Self {
        Self {
            endpoint_repository,
            event_repository,
            processor_registry,
            max_retries: 5,
        }
    }

    /// Set maximum retry attempts.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Process a single event by ID.
    pub async fn process_by_id(
        &self,
        event_id: &WebhookEventId,
    ) -> errors::Result<ProcessingStats> {
        let event = self
            .event_repository
            .find_by_id(event_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Webhook event"))?;

        self.process_event(event).await
    }

    /// Process a webhook event.
    pub async fn process_event(
        &self,
        mut event: WebhookEvent,
    ) -> errors::Result<ProcessingStats> {
        // Skip if not pending
        if *event.status() != ProcessingStatus::Pending {
            tracing::debug!(
                event_id = %event.id(),
                status = %event.status(),
                "Event is not pending, skipping"
            );
            return Ok(ProcessingStats::default());
        }

        // Skip if signature was invalid
        if !event.signature_valid() {
            event.mark_failed("Invalid webhook signature");
            self.event_repository.save(&event).await?;
            return Err(errors::Error::forbidden(
                "Invalid webhook signature",
            ));
        }

        // Mark as processing
        event.mark_processing();
        self.event_repository.save(&event).await?;

        // Get the endpoint
        let endpoint = match self
            .endpoint_repository
            .find_by_id(event.endpoint_id())
            .await?
        {
            Some(e) => e,
            None => {
                event.mark_failed("Endpoint not found");
                self.event_repository.save(&event).await?;
                return Err(errors::Error::not_found("Webhook endpoint"));
            }
        };

        // Get the processor
        let processor = match self.processor_registry.get(*event.provider())
        {
            Some(p) => p,
            None => {
                event.mark_failed(format!(
                    "No processor registered for provider: {}",
                    event.provider()
                ));
                self.event_repository.save(&event).await?;
                return Err(errors::Error::invalid(format!(
                    "No processor for provider: {}",
                    event.provider()
                )));
            }
        };

        // Process the event
        match processor.process(&event, &endpoint).await {
            Ok(stats) => {
                tracing::info!(
                    event_id = %event.id(),
                    created = stats.created,
                    updated = stats.updated,
                    deleted = stats.deleted,
                    skipped = stats.skipped,
                    "Event processed successfully"
                );

                event.mark_completed(stats.clone());
                self.event_repository.save(&event).await?;
                Ok(stats)
            }
            Err(e) => {
                tracing::error!(
                    event_id = %event.id(),
                    error = %e,
                    retry_count = event.retry_count(),
                    "Event processing failed"
                );

                // Try to schedule a retry
                if event.schedule_retry(self.max_retries) {
                    tracing::info!(
                        event_id = %event.id(),
                        retry_count = event.retry_count(),
                        next_retry_at = ?event.next_retry_at(),
                        "Event scheduled for retry"
                    );
                    self.event_repository.save(&event).await?;
                } else {
                    event.mark_failed(e.to_string());
                    self.event_repository.save(&event).await?;
                }

                Err(e)
            }
        }
    }

    /// Process all pending events (batch processing).
    pub async fn process_pending(
        &self,
        batch_size: u32,
    ) -> errors::Result<BatchResult> {
        let events = self
            .event_repository
            .find_pending_events(batch_size)
            .await?;

        let mut result = BatchResult::default();

        for event in events {
            match self.process_event(event).await {
                Ok(stats) => {
                    result.succeeded += 1;
                    result.items_created += stats.created;
                    result.items_updated += stats.updated;
                    result.items_deleted += stats.deleted;
                }
                Err(_) => {
                    result.failed += 1;
                }
            }
        }

        Ok(result)
    }
}

/// Result of batch processing.
#[derive(Debug, Default)]
pub struct BatchResult {
    /// Number of events processed successfully
    pub succeeded: u32,
    /// Number of events that failed
    pub failed: u32,
    /// Total items created across all events
    pub items_created: u32,
    /// Total items updated across all events
    pub items_updated: u32,
    /// Total items deleted across all events
    pub items_deleted: u32,
}

impl BatchResult {
    pub fn total_processed(&self) -> u32 {
        self.succeeded + self.failed
    }
}

/// Background worker for processing webhook events.
pub struct WebhookEventWorker {
    processor: Arc<ProcessWebhookEvent>,
    batch_size: u32,
    poll_interval: std::time::Duration,
}

impl WebhookEventWorker {
    pub fn new(processor: Arc<ProcessWebhookEvent>) -> Self {
        Self {
            processor,
            batch_size: 10,
            poll_interval: std::time::Duration::from_secs(5),
        }
    }

    /// Set batch size.
    pub fn with_batch_size(mut self, size: u32) -> Self {
        self.batch_size = size;
        self
    }

    /// Set poll interval.
    pub fn with_poll_interval(
        mut self,
        interval: std::time::Duration,
    ) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Run the worker loop.
    ///
    /// This should be spawned as a background task.
    pub async fn run(
        &self,
        mut shutdown: tokio::sync::broadcast::Receiver<()>,
    ) {
        tracing::info!(
            batch_size = self.batch_size,
            poll_interval_secs = self.poll_interval.as_secs(),
            "Webhook event worker started"
        );

        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    tracing::info!("Webhook event worker shutting down");
                    break;
                }
                _ = tokio::time::sleep(self.poll_interval) => {
                    match self.processor.process_pending(self.batch_size).await {
                        Ok(result) if result.total_processed() > 0 => {
                            tracing::debug!(
                                succeeded = result.succeeded,
                                failed = result.failed,
                                "Processed webhook events batch"
                            );
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Error processing webhook events");
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_result() {
        let result = BatchResult {
            succeeded: 5,
            failed: 2,
            items_created: 10,
            items_updated: 3,
            items_deleted: 1,
        };
        assert_eq!(result.total_processed(), 7);
    }
}
