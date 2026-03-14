//! Stripe API pull processor (stub implementation).

use async_trait::async_trait;
use std::sync::Arc;

use crate::usecase::ApiPullProcessor;
use inbound_sync_domain::{
    ProcessingStats, Provider, SyncOperation, SyncStateRepository,
    WebhookEndpoint,
};

/// Stripe API pull processor (stub).
#[derive(Debug)]
pub struct StripeApiPullProcessor;

impl Default for StripeApiPullProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl StripeApiPullProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ApiPullProcessor for StripeApiPullProcessor {
    fn provider(&self) -> Provider {
        Provider::Stripe
    }

    async fn pull_all(
        &self,
        _endpoint: &WebhookEndpoint,
        _sync_state_repo: &Arc<dyn SyncStateRepository>,
        _operation: &mut SyncOperation,
    ) -> errors::Result<ProcessingStats> {
        // TODO: Implement Stripe API pull
        Ok(ProcessingStats::default())
    }

    async fn pull_specific(
        &self,
        _endpoint: &WebhookEndpoint,
        _external_ids: Vec<String>,
        _sync_state_repo: &Arc<dyn SyncStateRepository>,
    ) -> errors::Result<ProcessingStats> {
        // TODO: Implement Stripe API pull
        Ok(ProcessingStats::default())
    }
}
