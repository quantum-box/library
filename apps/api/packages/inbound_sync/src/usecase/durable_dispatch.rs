//! Port for waking a durable webhook consumer after the database enqueue.

use inbound_sync_domain::WebhookEventId;

#[async_trait::async_trait]
pub trait WebhookDispatcher: Send + Sync + std::fmt::Debug {
    async fn dispatch(
        &self,
        event_id: &WebhookEventId,
    ) -> errors::Result<()>;
}
