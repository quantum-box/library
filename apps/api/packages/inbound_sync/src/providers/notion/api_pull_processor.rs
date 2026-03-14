//! Notion API pull processor (stub implementation).

use async_trait::async_trait;
use std::sync::Arc;

use crate::usecase::ApiPullProcessor;
use inbound_sync_domain::{
    ProcessingStats, Provider, SyncOperation, SyncStateRepository,
    WebhookEndpoint,
};

/// Notion API pull processor (stub).
#[derive(Debug)]
pub struct NotionApiPullProcessor;

impl NotionApiPullProcessor {
    pub fn new(
        _notion_client: Arc<dyn super::client::NotionClient>,
        _data_handler: Arc<dyn super::event_processor::NotionDataHandler>,
    ) -> Self {
        Self
    }
}

#[async_trait]
impl ApiPullProcessor for NotionApiPullProcessor {
    fn provider(&self) -> Provider {
        Provider::Notion
    }

    async fn pull_all(
        &self,
        _endpoint: &WebhookEndpoint,
        _sync_state_repo: &Arc<dyn SyncStateRepository>,
        _operation: &mut SyncOperation,
    ) -> errors::Result<ProcessingStats> {
        // TODO: Implement Notion API pull
        Ok(ProcessingStats::default())
    }

    async fn pull_specific(
        &self,
        _endpoint: &WebhookEndpoint,
        _external_ids: Vec<String>,
        _sync_state_repo: &Arc<dyn SyncStateRepository>,
    ) -> errors::Result<ProcessingStats> {
        // TODO: Implement Notion API pull
        Ok(ProcessingStats::default())
    }
}
