//! API Pull processor trait for provider-specific sync operations.

use std::sync::Arc;

use async_trait::async_trait;

use inbound_sync_domain::{
    ProcessingStats, Provider, SyncOperation, SyncStateRepository,
    WebhookEndpoint,
};

/// Trait for provider-specific API pull processors.
///
/// Each provider (GitHub, Linear, Notion, etc.) implements this trait
/// to handle API-based data synchronization (Initial Sync, On-demand Pull).
#[async_trait]
pub trait ApiPullProcessor: Send + Sync + std::fmt::Debug {
    /// Get the provider this processor handles.
    fn provider(&self) -> Provider;

    /// Pull all resources from the provider (Initial Sync).
    ///
    /// This method should:
    /// 1. Fetch all resources from the provider's API
    /// 2. Filter by configuration (e.g., path_pattern, team_id)
    /// 3. Transform data using property mappings
    /// 4. Upsert/delete data in Library
    /// 5. Update SyncState for each resource
    /// 6. Update operation progress periodically
    ///
    /// Returns processing statistics.
    async fn pull_all(
        &self,
        endpoint: &WebhookEndpoint,
        sync_state_repo: &Arc<dyn SyncStateRepository>,
        operation: &mut SyncOperation,
    ) -> errors::Result<ProcessingStats>;

    /// Pull specific resources by external IDs (On-demand Pull).
    ///
    /// Returns processing statistics.
    async fn pull_specific(
        &self,
        endpoint: &WebhookEndpoint,
        external_ids: Vec<String>,
        sync_state_repo: &Arc<dyn SyncStateRepository>,
    ) -> errors::Result<ProcessingStats>;
}

/// Registry of API pull processors for all providers.
#[derive(Debug, Default)]
pub struct ApiPullProcessorRegistry {
    processors:
        std::collections::HashMap<Provider, Arc<dyn ApiPullProcessor>>,
}

impl ApiPullProcessorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a processor for a provider.
    pub fn register(&mut self, processor: Arc<dyn ApiPullProcessor>) {
        self.processors.insert(processor.provider(), processor);
    }

    /// Get processor for a provider.
    pub fn get(
        &self,
        provider: Provider,
    ) -> Option<Arc<dyn ApiPullProcessor>> {
        self.processors.get(&provider).cloned()
    }
}
