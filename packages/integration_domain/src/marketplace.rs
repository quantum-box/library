//! Marketplace domain models for integration app store.
//!
//! This module provides domain models for the integration marketplace,
//! enabling tenants to discover, connect, and manage external service
//! integrations with OAuth support.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Marketplace                              │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
//! │  │   GitHub    │  │   Stripe    │  │   Square    │  ...     │
//! │  │ Integration │  │ Integration │  │ Integration │          │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘          │
//! └─────────│────────────────│────────────────│─────────────────┘
//!           │                │                │
//!           ▼                ▼                ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Tenant Connections                        │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
//! │  │ Connection  │  │ Connection  │  │ Connection  │          │
//! │  │  (Active)   │  │  (Expired)  │  │  (Active)   │          │
//! │  └─────────────┘  └─────────────┘  └─────────────┘          │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use crate::oauth::OAuthProvider;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use value_object::TenantId;

/// OAuth credentials for a provider (client_id + client_secret).
#[derive(Debug, Clone)]
pub struct OAuthCredentials {
    pub client_id: String,
    pub client_secret: String,
}

/// Resolves OAuth credentials for a given provider and tenant.
///
/// Implementations typically look up credentials from IaC
/// manifests or secrets stores rather than environment
/// variables.
#[async_trait::async_trait]
pub trait OAuthCredentialResolver: Send + Sync + std::fmt::Debug {
    async fn resolve(
        &self,
        provider: &OAuthProvider,
        tenant_id: &TenantId,
    ) -> errors::Result<OAuthCredentials>;
}

/// No-op resolver that always returns an error.
///
/// Used when no OAuth credential source is configured.
#[derive(Debug)]
pub struct NoOpOAuthCredentialResolver;

#[async_trait::async_trait]
impl OAuthCredentialResolver for NoOpOAuthCredentialResolver {
    async fn resolve(
        &self,
        provider: &OAuthProvider,
        _tenant_id: &TenantId,
    ) -> errors::Result<OAuthCredentials> {
        Err(errors::Error::internal_server_error(format!(
            "OAuth credentials not configured for provider: \
             {provider}"
        )))
    }
}

/// Unique identifier for an integration in the marketplace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntegrationId(String);

impl IntegrationId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IntegrationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Category of integration in the marketplace.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum IntegrationCategory {
    /// Code and repository management (GitHub, GitLab)
    CodeManagement,
    /// Issue and project tracking (Linear, Jira)
    ProjectManagement,
    /// Communication and messaging (Slack, Discord)
    Communication,
    /// Customer relationship management (HubSpot, Salesforce)
    Crm,
    /// Payment processing (Stripe, Square)
    Payments,
    /// Databases and content (Notion, Airtable)
    ContentManagement,
    /// E-commerce and inventory (Square, Shopify)
    Ecommerce,
    /// Custom/generic integrations
    Custom,
}

impl IntegrationCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            IntegrationCategory::CodeManagement => "code_management",
            IntegrationCategory::ProjectManagement => "project_management",
            IntegrationCategory::Communication => "communication",
            IntegrationCategory::Crm => "crm",
            IntegrationCategory::Payments => "payments",
            IntegrationCategory::ContentManagement => "content_management",
            IntegrationCategory::Ecommerce => "ecommerce",
            IntegrationCategory::Custom => "custom",
        }
    }
}

/// Sync direction capability.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SyncCapability {
    /// Can receive data from external service (webhooks)
    Inbound,
    /// Can push data to external service
    Outbound,
    /// Can both receive and push data
    Bidirectional,
}

/// OAuth configuration for an integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Required OAuth scopes for this integration
    pub scopes: Vec<String>,
    /// OAuth authorization URL
    pub auth_url: String,
    /// OAuth token URL
    pub token_url: String,
    /// Whether refresh tokens are supported
    pub supports_refresh: bool,
}

/// An integration available in the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    /// Unique identifier
    id: IntegrationId,
    /// The provider this integration is for
    provider: OAuthProvider,
    /// Display name
    name: String,
    /// Description of the integration
    description: String,
    /// Icon URL or identifier
    icon: Option<String>,
    /// Category
    category: IntegrationCategory,
    /// Sync capabilities
    sync_capability: SyncCapability,
    /// Supported object types (e.g., "customers", "orders", "products")
    supported_objects: Vec<String>,
    /// OAuth configuration
    oauth_config: Option<OAuthConfig>,
    /// Whether this integration is enabled in the marketplace
    is_enabled: bool,
    /// Whether this is a featured/recommended integration
    is_featured: bool,
    /// Version of the integration
    version: String,
    /// When the integration was added
    created_at: DateTime<Utc>,
    /// When the integration was last updated
    updated_at: DateTime<Utc>,
}

impl Integration {
    /// Create a new integration.
    pub fn new(
        id: IntegrationId,
        provider: OAuthProvider,
        name: impl Into<String>,
        description: impl Into<String>,
        category: IntegrationCategory,
        sync_capability: SyncCapability,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            provider,
            name: name.into(),
            description: description.into(),
            icon: None,
            category,
            sync_capability,
            supported_objects: vec![],
            oauth_config: None,
            is_enabled: true,
            is_featured: false,
            version: "1.0.0".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new integration with OAuth support.
    pub fn with_oauth(mut self, config: OAuthConfig) -> Self {
        self.oauth_config = Some(config);
        self
    }

    /// Set the supported object types.
    pub fn with_objects(mut self, objects: Vec<String>) -> Self {
        self.supported_objects = objects;
        self
    }

    /// Set the icon.
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Mark as featured.
    pub fn as_featured(mut self) -> Self {
        self.is_featured = true;
        self
    }

    /// Mark as disabled.
    pub fn set_disabled(mut self) -> Self {
        self.is_enabled = false;
        self
    }

    // Getters
    pub fn id(&self) -> &IntegrationId {
        &self.id
    }

    pub fn provider(&self) -> OAuthProvider {
        self.provider
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }

    pub fn category(&self) -> IntegrationCategory {
        self.category
    }

    pub fn sync_capability(&self) -> SyncCapability {
        self.sync_capability
    }

    pub fn supported_objects(&self) -> &[String] {
        &self.supported_objects
    }

    pub fn oauth_config(&self) -> Option<&OAuthConfig> {
        self.oauth_config.as_ref()
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    pub fn is_featured(&self) -> bool {
        self.is_featured
    }

    pub fn requires_oauth(&self) -> bool {
        self.oauth_config.is_some()
    }
}

/// Status of a tenant's connection to an integration.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ConnectionStatus {
    /// Connection is active and working
    Active,
    /// OAuth token has expired, needs re-authorization
    Expired,
    /// Connection is paused by the user
    Paused,
    /// Connection has been disconnected
    Disconnected,
    /// Connection has an error
    Error,
}

/// Unique identifier for a connection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(String);

impl ConnectionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new unique connection ID.
    pub fn generate() -> Self {
        use ulid::Ulid;
        Self(format!("con_{}", Ulid::new().to_string().to_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A tenant's connection to an integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Unique identifier
    id: ConnectionId,
    /// Tenant that owns this connection
    tenant_id: TenantId,
    /// Integration this connection is for
    integration_id: IntegrationId,
    /// Provider name for quick lookup
    provider: OAuthProvider,
    /// Current status
    status: ConnectionStatus,
    /// External account identifier (e.g., GitHub username, Stripe account ID)
    external_account_id: Option<String>,
    /// External account name for display
    external_account_name: Option<String>,
    /// When OAuth token expires (if applicable)
    token_expires_at: Option<DateTime<Utc>>,
    /// Error message if status is Error
    error_message: Option<String>,
    /// When the connection was created
    connected_at: DateTime<Utc>,
    /// When the connection was last synced
    last_synced_at: Option<DateTime<Utc>>,
    /// When the connection was last updated
    updated_at: DateTime<Utc>,
    /// Additional metadata (e.g., webhook_url for Slack)
    metadata: HashMap<String, serde_json::Value>,
}

impl Connection {
    /// Create a new connection with default status.
    pub fn create(
        id: ConnectionId,
        tenant_id: TenantId,
        integration_id: IntegrationId,
        provider: OAuthProvider,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            tenant_id,
            integration_id,
            provider,
            status: ConnectionStatus::Active,
            external_account_id: None,
            external_account_name: None,
            token_expires_at: None,
            error_message: None,
            connected_at: now,
            last_synced_at: None,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Reconstruct a connection from stored data.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ConnectionId,
        tenant_id: TenantId,
        integration_id: IntegrationId,
        provider: OAuthProvider,
        status: ConnectionStatus,
        external_account_id: Option<String>,
        external_account_name: Option<String>,
        token_expires_at: Option<DateTime<Utc>>,
        last_synced_at: Option<DateTime<Utc>>,
        error_message: Option<String>,
        connected_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            integration_id,
            provider,
            status,
            external_account_id,
            external_account_name,
            token_expires_at,
            error_message,
            connected_at,
            last_synced_at,
            updated_at,
            metadata,
        }
    }

    /// Set external account information.
    pub fn with_external_account(
        mut self,
        account_id: impl Into<String>,
        account_name: Option<String>,
    ) -> Self {
        self.external_account_id = Some(account_id.into());
        self.external_account_name = account_name;
        self
    }

    /// Set token expiration.
    pub fn with_token_expiration(
        mut self,
        expires_at: DateTime<Utc>,
    ) -> Self {
        self.token_expires_at = Some(expires_at);
        self
    }

    /// Mark the connection as expired.
    pub fn mark_expired(&mut self) {
        self.status = ConnectionStatus::Expired;
        self.updated_at = Utc::now();
    }

    /// Mark the connection as having an error.
    pub fn mark_error(&mut self, message: impl Into<String>) {
        self.status = ConnectionStatus::Error;
        self.error_message = Some(message.into());
        self.updated_at = Utc::now();
    }

    /// Pause the connection.
    pub fn pause(&mut self) {
        self.status = ConnectionStatus::Paused;
        self.updated_at = Utc::now();
    }

    /// Resume the connection.
    pub fn resume(&mut self) {
        self.status = ConnectionStatus::Active;
        self.error_message = None;
        self.updated_at = Utc::now();
    }

    /// Disconnect the connection.
    pub fn disconnect(&mut self) {
        self.status = ConnectionStatus::Disconnected;
        self.updated_at = Utc::now();
    }

    /// Record a successful sync.
    pub fn record_sync(&mut self) {
        self.last_synced_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Set external account information (mutable version).
    pub fn set_external_account(
        &mut self,
        account_id: Option<String>,
        account_name: Option<String>,
    ) {
        self.external_account_id = account_id;
        self.external_account_name = account_name;
        self.updated_at = Utc::now();
    }

    /// Set token expiration time (mutable version).
    pub fn set_token_expires_at(
        &mut self,
        expires_at: Option<DateTime<Utc>>,
    ) {
        self.token_expires_at = expires_at;
        self.updated_at = Utc::now();
    }

    /// Check if the token is expired.
    pub fn is_token_expired(&self) -> bool {
        self.token_expires_at
            .map(|exp| exp <= Utc::now())
            .unwrap_or(false)
    }

    // Getters
    pub fn id(&self) -> &ConnectionId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn integration_id(&self) -> &IntegrationId {
        &self.integration_id
    }

    pub fn provider(&self) -> OAuthProvider {
        self.provider
    }

    pub fn status(&self) -> ConnectionStatus {
        self.status
    }

    pub fn external_account_id(&self) -> Option<&str> {
        self.external_account_id.as_deref()
    }

    pub fn external_account_name(&self) -> Option<&str> {
        self.external_account_name.as_deref()
    }

    pub fn token_expires_at(&self) -> Option<DateTime<Utc>> {
        self.token_expires_at
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn connected_at(&self) -> DateTime<Utc> {
        self.connected_at
    }

    pub fn last_synced_at(&self) -> Option<DateTime<Utc>> {
        self.last_synced_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn is_active(&self) -> bool {
        self.status == ConnectionStatus::Active
    }

    pub fn metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.metadata
    }

    /// Get a metadata value by key.
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    /// Get a metadata string value by key.
    pub fn get_metadata_str(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).and_then(|v| v.as_str())
    }

    /// Set a metadata value.
    pub fn set_metadata(
        &mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) {
        self.metadata.insert(key.into(), value);
        self.updated_at = Utc::now();
    }

    /// Set metadata with builder pattern.
    pub fn with_metadata(
        mut self,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.metadata = metadata;
        self
    }
}

// ============================================================================
// Repository Traits
// ============================================================================

/// Repository for marketplace integrations.
///
/// Integrations are typically static/built-in, but this trait allows
/// for custom integrations to be stored in a database.
#[async_trait::async_trait]
pub trait IntegrationRepository: Send + Sync + std::fmt::Debug {
    /// Get all available integrations.
    async fn find_all(&self) -> errors::Result<Vec<Integration>>;

    /// Get all enabled integrations.
    async fn find_enabled(&self) -> errors::Result<Vec<Integration>>;

    /// Get integrations by category.
    async fn find_by_category(
        &self,
        category: IntegrationCategory,
    ) -> errors::Result<Vec<Integration>>;

    /// Get featured integrations.
    async fn find_featured(&self) -> errors::Result<Vec<Integration>>;

    /// Get an integration by ID.
    async fn find_by_id(
        &self,
        id: &IntegrationId,
    ) -> errors::Result<Option<Integration>>;

    /// Get an integration by provider.
    async fn find_by_provider(
        &self,
        provider: OAuthProvider,
    ) -> errors::Result<Option<Integration>>;
}

/// Repository for tenant connections to integrations.
#[async_trait::async_trait]
pub trait ConnectionRepository: Send + Sync + std::fmt::Debug {
    /// Save a connection.
    async fn save(&self, connection: &Connection) -> errors::Result<()>;

    /// Get a connection by ID.
    async fn find_by_id(
        &self,
        id: &ConnectionId,
    ) -> errors::Result<Option<Connection>>;

    /// Get all connections for a tenant.
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Connection>>;

    /// Get active connections for a tenant.
    async fn find_active_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<Connection>>;

    /// Get a connection by tenant and integration.
    async fn find_by_tenant_and_integration(
        &self,
        tenant_id: &TenantId,
        integration_id: &IntegrationId,
    ) -> errors::Result<Option<Connection>>;

    /// Get a connection by tenant and provider.
    async fn find_by_tenant_and_provider(
        &self,
        tenant_id: &TenantId,
        provider: OAuthProvider,
    ) -> errors::Result<Option<Connection>>;

    /// Get a connection by provider and external account ID.
    async fn find_by_provider_and_external_account_id(
        &self,
        provider: OAuthProvider,
        external_account_id: &str,
    ) -> errors::Result<Option<Connection>>;

    /// Delete a connection.
    async fn delete(&self, id: &ConnectionId) -> errors::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tenant_id() -> TenantId {
        "tn_01hjryxysgey07h5jz5wagqj0m".parse().unwrap()
    }

    #[test]
    fn test_integration_creation() {
        let integration = Integration::new(
            IntegrationId::new("int_github"),
            OAuthProvider::Github,
            "GitHub",
            "Connect your GitHub repositories",
            IntegrationCategory::CodeManagement,
            SyncCapability::Inbound,
        )
        .with_objects(vec![
            "repository".to_string(),
            "pull_request".to_string(),
            "issue".to_string(),
        ])
        .as_featured();

        assert_eq!(integration.name(), "GitHub");
        assert_eq!(integration.provider(), OAuthProvider::Github);
        assert!(integration.is_featured());
        assert_eq!(integration.supported_objects().len(), 3);
    }

    #[test]
    fn test_connection_lifecycle() {
        let mut connection = Connection::create(
            ConnectionId::new("con_123"),
            test_tenant_id(),
            IntegrationId::new("int_github"),
            OAuthProvider::Github,
        );

        assert!(connection.is_active());

        connection.pause();
        assert_eq!(connection.status(), ConnectionStatus::Paused);

        connection.resume();
        assert!(connection.is_active());

        connection.mark_error("API rate limit exceeded");
        assert_eq!(connection.status(), ConnectionStatus::Error);
        assert_eq!(
            connection.error_message(),
            Some("API rate limit exceeded")
        );

        connection.disconnect();
        assert_eq!(connection.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_token_expiration_check() {
        let mut connection = Connection::create(
            ConnectionId::new("con_123"),
            test_tenant_id(),
            IntegrationId::new("int_github"),
            OAuthProvider::Github,
        );

        // No expiration set
        assert!(!connection.is_token_expired());

        // Set future expiration
        connection = connection
            .with_token_expiration(Utc::now() + chrono::Duration::hours(1));
        assert!(!connection.is_token_expired());

        // Set past expiration
        connection.token_expires_at =
            Some(Utc::now() - chrono::Duration::hours(1));
        assert!(connection.is_token_expired());
    }
}
