//! HubSpot API pull processor (stub implementation).

use async_trait::async_trait;
use std::sync::Arc;

use crate::usecase::ApiPullProcessor;
use inbound_sync_domain::{
    ProcessingStats, Provider, SyncOperation, SyncStateRepository,
    WebhookEndpoint,
};

/// HubSpot API pull processor (stub).
#[derive(Debug)]
pub struct HubSpotApiPullProcessor;

impl Default for HubSpotApiPullProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl HubSpotApiPullProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ApiPullProcessor for HubSpotApiPullProcessor {
    fn provider(&self) -> Provider {
        Provider::Hubspot
    }

    async fn pull_all(
        &self,
        _endpoint: &WebhookEndpoint,
        _sync_state_repo: &Arc<dyn SyncStateRepository>,
        _operation: &mut SyncOperation,
    ) -> errors::Result<ProcessingStats> {
        // TODO: Implement HubSpot API pull
        Ok(ProcessingStats::default())
    }

    async fn pull_specific(
        &self,
        _endpoint: &WebhookEndpoint,
        _external_ids: Vec<String>,
        _sync_state_repo: &Arc<dyn SyncStateRepository>,
    ) -> errors::Result<ProcessingStats> {
        // TODO: Implement HubSpot API pull
        Ok(ProcessingStats::default())
    }
}
