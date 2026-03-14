//! Sync operation entity for API pull synchronization operations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use ulid::Ulid;
use util::{def_id, def_id_serde_impls};

use crate::{ProcessingStats, WebhookEndpointId};

def_id!(SyncOperationId, "syo_");

/// Type of sync operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperationType {
    /// Reactive sync via webhook (existing)
    Webhook,
    /// Initial full sync
    InitialSync,
    /// On-demand pull sync (user-triggered)
    OnDemandPull,
    /// Scheduled polling sync
    ScheduledSync,
}

impl SyncOperationType {
    pub fn as_str(&self) -> &str {
        match self {
            SyncOperationType::Webhook => "webhook",
            SyncOperationType::InitialSync => "initial_sync",
            SyncOperationType::OnDemandPull => "on_demand_pull",
            SyncOperationType::ScheduledSync => "scheduled_sync",
        }
    }
}

impl std::fmt::Display for SyncOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SyncOperationType {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "webhook" => Ok(SyncOperationType::Webhook),
            "initial_sync" => Ok(SyncOperationType::InitialSync),
            "on_demand_pull" => Ok(SyncOperationType::OnDemandPull),
            "scheduled_sync" => Ok(SyncOperationType::ScheduledSync),
            _ => Err(errors::Error::invalid(format!(
                "Invalid sync operation type: {s}"
            ))),
        }
    }
}

/// Status of a sync operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperationStatus {
    /// Operation is queued
    Queued,
    /// Operation is currently running
    Running,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
}

impl SyncOperationStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SyncOperationStatus::Queued => "queued",
            SyncOperationStatus::Running => "running",
            SyncOperationStatus::Completed => "completed",
            SyncOperationStatus::Failed => "failed",
            SyncOperationStatus::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SyncOperationStatus::Completed
                | SyncOperationStatus::Failed
                | SyncOperationStatus::Cancelled
        )
    }
}

impl std::fmt::Display for SyncOperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SyncOperationStatus {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(SyncOperationStatus::Queued),
            "running" => Ok(SyncOperationStatus::Running),
            "completed" => Ok(SyncOperationStatus::Completed),
            "failed" => Ok(SyncOperationStatus::Failed),
            "cancelled" => Ok(SyncOperationStatus::Cancelled),
            _ => Err(errors::Error::invalid(format!(
                "Invalid sync operation status: {s}"
            ))),
        }
    }
}

/// Sync operation entity.
///
/// Represents an API pull synchronization operation.
/// Operations are tracked for progress monitoring and history.
#[derive(Debug, Clone, Getters, new)]
pub struct SyncOperation {
    /// Unique identifier
    id: SyncOperationId,
    /// Endpoint that this operation syncs
    endpoint_id: WebhookEndpointId,
    /// Type of sync operation
    operation_type: SyncOperationType,
    /// Operation status
    status: SyncOperationStatus,
    /// When the operation started
    started_at: DateTime<Utc>,
    /// When the operation completed
    #[getter(skip)]
    completed_at: Option<DateTime<Utc>>,
    /// Processing statistics
    #[getter(skip)]
    stats: Option<ProcessingStats>,
    /// Error message (if failed)
    #[getter(skip)]
    error_message: Option<String>,
    /// Progress information (e.g., "10/100 files processed")
    #[getter(skip)]
    progress: Option<String>,
}

impl SyncOperation {
    /// Create a new sync operation.
    pub fn create(
        endpoint_id: WebhookEndpointId,
        operation_type: SyncOperationType,
    ) -> Self {
        Self {
            id: SyncOperationId::default(),
            endpoint_id,
            operation_type,
            status: SyncOperationStatus::Queued,
            started_at: Utc::now(),
            completed_at: None,
            stats: None,
            error_message: None,
            progress: None,
        }
    }

    /// Get completion time.
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    /// Get processing statistics.
    pub fn stats(&self) -> Option<&ProcessingStats> {
        self.stats.as_ref()
    }

    /// Get error message.
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Get progress information.
    pub fn progress(&self) -> Option<&str> {
        self.progress.as_deref()
    }

    /// Mark as running.
    pub fn mark_running(&mut self) {
        self.status = SyncOperationStatus::Running;
    }

    /// Mark as completed with statistics.
    pub fn mark_completed(&mut self, stats: ProcessingStats) {
        self.status = SyncOperationStatus::Completed;
        self.stats = Some(stats);
        self.completed_at = Some(Utc::now());
        self.error_message = None;
    }

    /// Mark as failed with error message.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = SyncOperationStatus::Failed;
        self.error_message = Some(error.into());
        self.completed_at = Some(Utc::now());
    }

    /// Mark as cancelled.
    pub fn mark_cancelled(&mut self) {
        self.status = SyncOperationStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// Update progress information.
    pub fn update_progress(&mut self, progress: impl Into<String>) {
        self.progress = Some(progress.into());
    }
}

/// Repository for sync operations.
#[async_trait]
pub trait SyncOperationRepository: Send + Sync + Debug {
    /// Save a sync operation.
    async fn save(&self, operation: &SyncOperation) -> errors::Result<()>;

    /// Find by ID.
    async fn find_by_id(
        &self,
        id: &SyncOperationId,
    ) -> errors::Result<Option<SyncOperation>>;

    /// Find operations by endpoint.
    async fn find_by_endpoint(
        &self,
        endpoint_id: &WebhookEndpointId,
        limit: u32,
        offset: u32,
    ) -> errors::Result<Vec<SyncOperation>>;

    /// Find pending operations.
    async fn find_pending(
        &self,
        limit: u32,
    ) -> errors::Result<Vec<SyncOperation>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_operation_status() {
        assert!(SyncOperationStatus::Completed.is_terminal());
        assert!(SyncOperationStatus::Failed.is_terminal());
        assert!(SyncOperationStatus::Cancelled.is_terminal());
        assert!(!SyncOperationStatus::Queued.is_terminal());
        assert!(!SyncOperationStatus::Running.is_terminal());
    }

    #[test]
    fn test_create_sync_operation() {
        let endpoint_id = WebhookEndpointId::default();
        let operation = SyncOperation::create(
            endpoint_id.clone(),
            SyncOperationType::InitialSync,
        );

        assert_eq!(
            *operation.operation_type(),
            SyncOperationType::InitialSync
        );
        assert_eq!(*operation.status(), SyncOperationStatus::Queued);
        assert_eq!(*operation.endpoint_id(), endpoint_id);
        assert!(operation.completed_at().is_none());
        assert!(operation.stats().is_none());
    }

    #[test]
    fn test_mark_completed() {
        let endpoint_id = WebhookEndpointId::default();
        let mut operation = SyncOperation::create(
            endpoint_id,
            SyncOperationType::InitialSync,
        );

        let stats = ProcessingStats {
            created: 5,
            updated: 3,
            deleted: 1,
            skipped: 2,
        };

        operation.mark_completed(stats.clone());

        assert_eq!(*operation.status(), SyncOperationStatus::Completed);
        assert!(operation.completed_at().is_some());
        assert_eq!(operation.stats(), Some(&stats));
        assert!(operation.error_message().is_none());
    }

    #[test]
    fn test_mark_failed() {
        let endpoint_id = WebhookEndpointId::default();
        let mut operation = SyncOperation::create(
            endpoint_id,
            SyncOperationType::InitialSync,
        );

        operation.mark_failed("Test error");

        assert_eq!(*operation.status(), SyncOperationStatus::Failed);
        assert!(operation.completed_at().is_some());
        assert_eq!(operation.error_message(), Some("Test error"));
    }

    #[test]
    fn test_update_progress() {
        let endpoint_id = WebhookEndpointId::default();
        let mut operation = SyncOperation::create(
            endpoint_id,
            SyncOperationType::InitialSync,
        );

        operation.update_progress("10/100 files");

        assert_eq!(operation.progress(), Some("10/100 files"));
    }
}
