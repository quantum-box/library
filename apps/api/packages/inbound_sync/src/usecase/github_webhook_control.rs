//! Broker-owned GitHub repository webhook operations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use value_object::TenantId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubManagedWebhook {
    pub id: u64,
    pub active: bool,
    pub events: Vec<String>,
    pub config: GitHubManagedWebhookConfig,
    pub last_response: Option<GitHubWebhookLastResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubManagedWebhookConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubWebhookLastResponse {
    pub code: Option<u16>,
    pub status: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubWebhookDelivery {
    pub id: u64,
    pub guid: String,
    pub delivered_at: Option<String>,
    pub status: Option<String>,
    pub status_code: Option<u16>,
    pub redelivery: bool,
}

#[async_trait]
pub trait GitHubWebhookControl: Send + Sync {
    async fn ensure(
        &self,
        tenant: &TenantId,
        repository: &str,
        url: &str,
        secret: &str,
    ) -> errors::Result<GitHubManagedWebhook>;

    async fn find(
        &self,
        tenant: &TenantId,
        repository: &str,
        url: &str,
    ) -> errors::Result<Option<GitHubManagedWebhook>>;

    async fn remove(
        &self,
        tenant: &TenantId,
        repository: &str,
        url: &str,
        hook_id: u64,
    ) -> errors::Result<()>;

    async fn deliveries(
        &self,
        tenant: &TenantId,
        repository: &str,
        url: &str,
        hook_id: u64,
    ) -> errors::Result<Vec<GitHubWebhookDelivery>>;

    async fn redeliver(
        &self,
        tenant: &TenantId,
        repository: &str,
        url: &str,
        hook_id: u64,
        delivery_id: u64,
    ) -> errors::Result<()>;
}
