//! Webhook endpoint configuration entity.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use ulid::Ulid;
use util::{def_id, def_id_serde_impls};
use value_object::TenantId;

use crate::{PropertyMapping, Provider};

def_id!(WebhookEndpointId, "whe_");

/// Status of a webhook endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointStatus {
    /// Endpoint is active and receiving webhooks
    Active,
    /// Endpoint is paused (webhooks are received but not processed)
    Paused,
    /// Endpoint is disabled (webhooks are rejected)
    Disabled,
}

impl EndpointStatus {
    pub fn as_str(&self) -> &str {
        match self {
            EndpointStatus::Active => "active",
            EndpointStatus::Paused => "paused",
            EndpointStatus::Disabled => "disabled",
        }
    }
}

impl std::fmt::Display for EndpointStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for EndpointStatus {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(EndpointStatus::Active),
            "paused" => Ok(EndpointStatus::Paused),
            "disabled" => Ok(EndpointStatus::Disabled),
            _ => Err(errors::Error::invalid(format!(
                "Invalid endpoint status: {s}"
            ))),
        }
    }
}

/// Provider-specific configuration.
///
/// Each provider has different configuration requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ProviderConfig {
    /// GitHub configuration
    Github {
        /// GitHub repository (owner/repo)
        repository: String,
        /// Branch to watch (default: main)
        #[serde(default = "default_branch")]
        branch: String,
        /// Path pattern to filter (e.g., "docs/**/*.md")
        #[serde(default)]
        path_pattern: Option<String>,
    },
    /// Linear configuration
    Linear {
        /// Team ID to filter (optional)
        #[serde(default)]
        team_id: Option<String>,
        /// Project ID to filter (optional)
        #[serde(default)]
        project_id: Option<String>,
        /// Webhook secret for signature verification (optional)
        #[serde(default)]
        webhook_secret: Option<String>,
    },
    /// HubSpot configuration
    Hubspot {
        /// Portal ID
        portal_id: String,
        /// Object types to sync (contacts, companies, deals, products)
        #[serde(default)]
        object_types: Vec<String>,
    },
    /// Stripe configuration
    Stripe {
        /// Object types to sync (products, prices, customers)
        #[serde(default)]
        sync_objects: Vec<String>,
        /// Whether to fetch latest data from API (recommended: true)
        #[serde(default = "default_true")]
        fetch_latest: bool,
    },
    /// Square configuration
    Square {
        /// Object types to sync (catalog_item, customer, order, payment, inventory)
        #[serde(default)]
        sync_objects: Vec<String>,
        /// Whether to fetch latest data from API (recommended: true)
        #[serde(default = "default_true")]
        fetch_latest: bool,
    },
    /// Notion configuration
    Notion {
        /// Database ID
        database_id: String,
    },
    /// Airtable configuration
    Airtable {
        /// Base ID
        base_id: String,
        /// Table ID
        table_id: String,
    },
    /// Generic configuration
    Generic {
        /// Signature header name
        #[serde(default = "default_signature_header")]
        signature_header: String,
        /// Signature algorithm (hmac-sha256, hmac-sha1)
        #[serde(default = "default_signature_algorithm")]
        signature_algorithm: String,
    },
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_true() -> bool {
    true
}

fn default_signature_header() -> String {
    "x-signature-256".to_string()
}

fn default_signature_algorithm() -> String {
    "hmac-sha256".to_string()
}

impl ProviderConfig {
    /// Get the provider type for this configuration.
    pub fn provider(&self) -> Provider {
        match self {
            ProviderConfig::Github { .. } => Provider::Github,
            ProviderConfig::Linear { .. } => Provider::Linear,
            ProviderConfig::Hubspot { .. } => Provider::Hubspot,
            ProviderConfig::Stripe { .. } => Provider::Stripe,
            ProviderConfig::Square { .. } => Provider::Square,
            ProviderConfig::Notion { .. } => Provider::Notion,
            ProviderConfig::Airtable { .. } => Provider::Airtable,
            ProviderConfig::Generic { .. } => Provider::Generic,
        }
    }
}

/// Webhook endpoint entity.
///
/// Represents a configured webhook receiver for a specific external provider.
/// Each endpoint has a unique URL and secret for signature verification.
#[derive(Debug, Clone, Getters, new)]
pub struct WebhookEndpoint {
    /// Unique identifier
    id: WebhookEndpointId,
    /// Tenant (organization) this endpoint belongs to
    tenant_id: TenantId,
    /// Target repository ID in Library (optional, for repo-specific endpoints)
    #[getter(skip)]
    repository_id: Option<String>,
    /// Display name
    name: String,
    /// Provider type
    provider: Provider,
    /// Provider-specific configuration
    config: ProviderConfig,
    /// Events to listen for
    events: Vec<String>,
    /// Property mapping configuration
    #[getter(skip)]
    mapping: Option<PropertyMapping>,
    /// Secret for signature verification (hashed)
    secret_hash: String,
    /// Endpoint status
    status: EndpointStatus,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last update timestamp
    updated_at: DateTime<Utc>,
}

impl WebhookEndpoint {
    /// Create a new webhook endpoint.
    pub fn create(
        tenant_id: TenantId,
        name: impl Into<String>,
        provider: Provider,
        config: ProviderConfig,
        events: Vec<String>,
        secret_hash: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: WebhookEndpointId::default(),
            tenant_id,
            repository_id: None,
            name: name.into(),
            provider,
            config,
            events,
            mapping: None,
            secret_hash: secret_hash.into(),
            status: EndpointStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get repository ID.
    pub fn repository_id(&self) -> Option<&str> {
        self.repository_id.as_deref()
    }

    /// Get property mapping.
    pub fn mapping(&self) -> Option<&PropertyMapping> {
        self.mapping.as_ref()
    }

    /// Get webhook secret (if stored in config).
    pub fn webhook_secret(&self) -> Option<&str> {
        match &self.config {
            ProviderConfig::Linear { webhook_secret, .. } => {
                webhook_secret.as_deref()
            }
            _ => None,
        }
    }

    /// Set repository ID.
    pub fn set_repository_id(&mut self, repository_id: Option<String>) {
        self.repository_id = repository_id;
        self.updated_at = Utc::now();
    }

    /// Set property mapping.
    pub fn set_mapping(&mut self, mapping: Option<PropertyMapping>) {
        self.mapping = mapping;
        self.updated_at = Utc::now();
    }

    /// Update secret hash.
    pub fn set_secret_hash(&mut self, secret_hash: String) {
        self.secret_hash = secret_hash;
        self.updated_at = Utc::now();
    }

    /// Update events to listen for.
    pub fn set_events(&mut self, events: Vec<String>) {
        self.events = events;
        self.updated_at = Utc::now();
    }

    /// Update status.
    pub fn set_status(&mut self, status: EndpointStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Update provider configuration.
    pub fn set_config(&mut self, config: ProviderConfig) {
        self.config = config;
        self.updated_at = Utc::now();
    }

    /// Check if this endpoint should process the given event type.
    pub fn should_process_event(&self, event_type: &str) -> bool {
        if self.status != EndpointStatus::Active {
            return false;
        }
        // Empty events list means accept all events
        if self.events.is_empty() {
            return true;
        }
        self.events.iter().any(|e| e == event_type)
    }

    /// Generate the webhook URL for this endpoint.
    pub fn webhook_url(&self, base_url: &str) -> String {
        match self.provider {
            Provider::Linear => format!(
                "{}/webhooks/{}",
                base_url.trim_end_matches('/'),
                self.provider
            ),
            _ => format!(
                "{}/webhooks/{}/{}",
                base_url.trim_end_matches('/'),
                self.provider,
                self.id
            ),
        }
    }
}

/// Repository for webhook endpoints.
#[async_trait]
pub trait WebhookEndpointRepository: Send + Sync + Debug {
    /// Save a webhook endpoint.
    async fn save(&self, endpoint: &WebhookEndpoint) -> errors::Result<()>;

    /// Find by ID.
    async fn find_by_id(
        &self,
        id: &WebhookEndpointId,
    ) -> errors::Result<Option<WebhookEndpoint>>;

    /// Find all endpoints for a tenant.
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<WebhookEndpoint>>;

    /// Find all endpoints for a tenant and provider.
    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &TenantId,
        provider: Provider,
    ) -> errors::Result<Vec<WebhookEndpoint>>;

    /// Delete an endpoint.
    async fn delete(&self, id: &WebhookEndpointId) -> errors::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tenant_id() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap()
    }

    #[test]
    fn test_webhook_url_generation() {
        let endpoint = WebhookEndpoint::create(
            test_tenant_id(),
            "Test Endpoint",
            Provider::Github,
            ProviderConfig::Github {
                repository: "owner/repo".to_string(),
                branch: "main".to_string(),
                path_pattern: None,
            },
            vec!["push".to_string()],
            "secret_hash",
        );

        let url = endpoint.webhook_url("https://api.example.com");
        assert!(
            url.starts_with("https://api.example.com/webhooks/github/whe_")
        );
    }

    #[test]
    fn test_should_process_event() {
        let mut endpoint = WebhookEndpoint::create(
            test_tenant_id(),
            "Test",
            Provider::Github,
            ProviderConfig::Github {
                repository: "owner/repo".to_string(),
                branch: "main".to_string(),
                path_pattern: None,
            },
            vec!["push".to_string(), "pull_request".to_string()],
            "secret",
        );

        assert!(endpoint.should_process_event("push"));
        assert!(endpoint.should_process_event("pull_request"));
        assert!(!endpoint.should_process_event("issues"));

        // Disabled endpoint
        endpoint.set_status(EndpointStatus::Disabled);
        assert!(!endpoint.should_process_event("push"));
    }
}
