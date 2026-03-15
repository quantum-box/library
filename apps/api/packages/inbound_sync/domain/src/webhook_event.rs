//! Webhook event entity for received webhook payloads.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use ulid::Ulid;
use util::{def_id, def_id_serde_impls};

use crate::{Provider, WebhookEndpointId};

def_id!(WebhookEventId, "wev_");

/// Processing status of a webhook event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingStatus {
    /// Event is queued for processing
    Pending,
    /// Event is currently being processed
    Processing,
    /// Event was processed successfully
    Completed,
    /// Event processing failed
    Failed,
    /// Event was skipped (e.g., not matching filters)
    Skipped,
}

impl ProcessingStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ProcessingStatus::Pending => "pending",
            ProcessingStatus::Processing => "processing",
            ProcessingStatus::Completed => "completed",
            ProcessingStatus::Failed => "failed",
            ProcessingStatus::Skipped => "skipped",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ProcessingStatus::Completed
                | ProcessingStatus::Failed
                | ProcessingStatus::Skipped
        )
    }
}

impl std::fmt::Display for ProcessingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ProcessingStatus {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(ProcessingStatus::Pending),
            "processing" => Ok(ProcessingStatus::Processing),
            "completed" => Ok(ProcessingStatus::Completed),
            "failed" => Ok(ProcessingStatus::Failed),
            "skipped" => Ok(ProcessingStatus::Skipped),
            _ => Err(errors::Error::invalid(format!(
                "Invalid processing status: {s}"
            ))),
        }
    }
}

/// Processing result statistics.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Number of items created
    pub created: u32,
    /// Number of items updated
    pub updated: u32,
    /// Number of items deleted
    pub deleted: u32,
    /// Number of items skipped
    pub skipped: u32,
}

impl ProcessingStats {
    pub fn total(&self) -> u32 {
        self.created + self.updated + self.deleted + self.skipped
    }
}

/// Webhook event entity.
///
/// Represents a received webhook from an external provider.
/// Events are queued and processed asynchronously.
#[derive(Debug, Clone, Getters, new)]
#[allow(clippy::too_many_arguments)]
pub struct WebhookEvent {
    /// Unique identifier
    id: WebhookEventId,
    /// Endpoint that received this event
    endpoint_id: WebhookEndpointId,
    /// Provider type
    provider: Provider,
    /// Event type (e.g., "push", "issue.updated")
    event_type: String,
    /// Raw webhook payload (JSON)
    payload: serde_json::Value,
    /// HTTP headers from the webhook request
    #[getter(skip)]
    headers: Option<serde_json::Value>,
    /// Whether signature verification passed
    signature_valid: bool,
    /// Processing status
    status: ProcessingStatus,
    /// Error message (if failed)
    #[getter(skip)]
    error_message: Option<String>,
    /// Number of retry attempts
    retry_count: u32,
    /// Next retry time (if pending retry)
    #[getter(skip)]
    next_retry_at: Option<DateTime<Utc>>,
    /// Processing statistics
    #[getter(skip)]
    stats: Option<ProcessingStats>,
    /// When the webhook was received
    received_at: DateTime<Utc>,
    /// When processing completed
    #[getter(skip)]
    processed_at: Option<DateTime<Utc>>,
}

impl WebhookEvent {
    /// Create a new webhook event.
    pub fn create(
        endpoint_id: WebhookEndpointId,
        provider: Provider,
        event_type: impl Into<String>,
        payload: serde_json::Value,
        headers: Option<serde_json::Value>,
        signature_valid: bool,
    ) -> Self {
        Self {
            id: WebhookEventId::default(),
            endpoint_id,
            provider,
            event_type: event_type.into(),
            payload,
            headers,
            signature_valid,
            status: ProcessingStatus::Pending,
            error_message: None,
            retry_count: 0,
            next_retry_at: None,
            stats: None,
            received_at: Utc::now(),
            processed_at: None,
        }
    }

    /// Get headers.
    pub fn headers(&self) -> Option<&serde_json::Value> {
        self.headers.as_ref()
    }

    /// Get error message.
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Get next retry time.
    pub fn next_retry_at(&self) -> Option<DateTime<Utc>> {
        self.next_retry_at
    }

    /// Get processing statistics.
    pub fn stats(&self) -> Option<&ProcessingStats> {
        self.stats.as_ref()
    }

    /// Get processed at time.
    pub fn processed_at(&self) -> Option<DateTime<Utc>> {
        self.processed_at
    }

    /// Mark as processing.
    pub fn mark_processing(&mut self) {
        self.status = ProcessingStatus::Processing;
    }

    /// Mark as completed with statistics.
    pub fn mark_completed(&mut self, stats: ProcessingStats) {
        self.status = ProcessingStatus::Completed;
        self.stats = Some(stats);
        self.processed_at = Some(Utc::now());
        self.error_message = None;
    }

    /// Mark as failed with error message.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = ProcessingStatus::Failed;
        self.error_message = Some(error.into());
        self.processed_at = Some(Utc::now());
    }

    /// Mark as skipped with reason.
    pub fn mark_skipped(&mut self, reason: impl Into<String>) {
        self.status = ProcessingStatus::Skipped;
        self.error_message = Some(reason.into());
        self.processed_at = Some(Utc::now());
    }

    /// Schedule a retry with exponential backoff.
    pub fn schedule_retry(&mut self, max_retries: u32) -> bool {
        if self.retry_count >= max_retries {
            return false;
        }

        self.retry_count += 1;
        self.status = ProcessingStatus::Pending;

        // Exponential backoff: 1min, 2min, 4min, 8min, 16min, ...
        let delay_seconds = 60 * 2u64.pow(self.retry_count - 1);
        self.next_retry_at = Some(
            Utc::now() + chrono::Duration::seconds(delay_seconds as i64),
        );

        true
    }

    /// Check if this event should be retried now.
    pub fn should_retry_now(&self) -> bool {
        if self.status != ProcessingStatus::Pending {
            return false;
        }

        match self.next_retry_at {
            Some(retry_at) => Utc::now() >= retry_at,
            None => true, // No specific retry time, can retry immediately
        }
    }
}

/// Repository for webhook events.
#[async_trait]
pub trait WebhookEventRepository: Send + Sync + Debug {
    /// Save a webhook event.
    async fn save(&self, event: &WebhookEvent) -> errors::Result<()>;

    /// Find by ID.
    async fn find_by_id(
        &self,
        id: &WebhookEventId,
    ) -> errors::Result<Option<WebhookEvent>>;

    /// Find events by endpoint.
    async fn find_by_endpoint(
        &self,
        endpoint_id: &WebhookEndpointId,
        limit: u32,
        offset: u32,
    ) -> errors::Result<Vec<WebhookEvent>>;

    /// Find pending events that are ready to process.
    async fn find_pending_events(
        &self,
        limit: u32,
    ) -> errors::Result<Vec<WebhookEvent>>;

    /// Find events by status.
    async fn find_by_status(
        &self,
        endpoint_id: &WebhookEndpointId,
        status: ProcessingStatus,
        limit: u32,
    ) -> errors::Result<Vec<WebhookEvent>>;

    /// Update event status (for optimistic locking).
    async fn update_status(
        &self,
        id: &WebhookEventId,
        status: ProcessingStatus,
        error_message: Option<String>,
        stats: Option<ProcessingStats>,
    ) -> errors::Result<()>;

    /// Delete old events (for cleanup).
    async fn delete_older_than(
        &self,
        before: DateTime<Utc>,
    ) -> errors::Result<u64>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_status() {
        assert!(ProcessingStatus::Completed.is_terminal());
        assert!(ProcessingStatus::Failed.is_terminal());
        assert!(ProcessingStatus::Skipped.is_terminal());
        assert!(!ProcessingStatus::Pending.is_terminal());
        assert!(!ProcessingStatus::Processing.is_terminal());
    }

    #[test]
    fn test_schedule_retry() {
        let mut event = WebhookEvent::create(
            WebhookEndpointId::default(),
            Provider::Github,
            "push",
            serde_json::json!({}),
            None,
            true,
        );

        // First retry
        assert!(event.schedule_retry(3));
        assert_eq!(event.retry_count, 1);
        assert!(event.next_retry_at.is_some());

        // Second retry
        assert!(event.schedule_retry(3));
        assert_eq!(event.retry_count, 2);

        // Third retry
        assert!(event.schedule_retry(3));
        assert_eq!(event.retry_count, 3);

        // Fourth retry should fail
        assert!(!event.schedule_retry(3));
        assert_eq!(event.retry_count, 3);
    }

    #[test]
    fn test_processing_stats() {
        let stats = ProcessingStats {
            created: 2,
            updated: 5,
            deleted: 1,
            skipped: 3,
        };
        assert_eq!(stats.total(), 11);
    }
}
