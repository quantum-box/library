//! GraphQL type definitions for library sync.

use async_graphql::{Enum, InputObject, SimpleObject};
use chrono::{DateTime, Utc};

use inbound_sync_domain::{
    Connection, ConnectionStatus, EndpointStatus, Integration,
    IntegrationCategory, OAuthProvider, ProcessingStats, ProcessingStatus,
    SyncCapability, SyncDirection, SyncState, WebhookEndpoint,
    WebhookEvent,
};

/// Provider type enum for GraphQL.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlProvider {
    Github,
    Linear,
    Hubspot,
    Stripe,
    Square,
    Notion,
    Airtable,
    Generic,
    Custom,
}

impl From<OAuthProvider> for GqlProvider {
    fn from(p: OAuthProvider) -> Self {
        match p {
            OAuthProvider::Github => GqlProvider::Github,
            OAuthProvider::Linear => GqlProvider::Linear,
            OAuthProvider::Hubspot => GqlProvider::Hubspot,
            OAuthProvider::Stripe => GqlProvider::Stripe,
            OAuthProvider::Square => GqlProvider::Square,
            OAuthProvider::Notion => GqlProvider::Notion,
            OAuthProvider::Airtable => GqlProvider::Airtable,
            OAuthProvider::Slack => GqlProvider::Generic,
            OAuthProvider::Discord => GqlProvider::Generic,
            OAuthProvider::Generic => GqlProvider::Generic,
            OAuthProvider::Custom => GqlProvider::Custom,
        }
    }
}

impl From<GqlProvider> for OAuthProvider {
    fn from(p: GqlProvider) -> Self {
        match p {
            GqlProvider::Github => OAuthProvider::Github,
            GqlProvider::Linear => OAuthProvider::Linear,
            GqlProvider::Hubspot => OAuthProvider::Hubspot,
            GqlProvider::Stripe => OAuthProvider::Stripe,
            GqlProvider::Square => OAuthProvider::Square,
            GqlProvider::Notion => OAuthProvider::Notion,
            GqlProvider::Airtable => OAuthProvider::Airtable,
            GqlProvider::Generic => OAuthProvider::Generic,
            GqlProvider::Custom => OAuthProvider::Custom,
        }
    }
}

/// Endpoint status enum for GraphQL.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlEndpointStatus {
    Active,
    Paused,
    Disabled,
}

impl From<EndpointStatus> for GqlEndpointStatus {
    fn from(s: EndpointStatus) -> Self {
        match s {
            EndpointStatus::Active => GqlEndpointStatus::Active,
            EndpointStatus::Paused => GqlEndpointStatus::Paused,
            EndpointStatus::Disabled => GqlEndpointStatus::Disabled,
        }
    }
}

impl From<GqlEndpointStatus> for EndpointStatus {
    fn from(s: GqlEndpointStatus) -> Self {
        match s {
            GqlEndpointStatus::Active => EndpointStatus::Active,
            GqlEndpointStatus::Paused => EndpointStatus::Paused,
            GqlEndpointStatus::Disabled => EndpointStatus::Disabled,
        }
    }
}

/// Processing status enum for GraphQL.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Skipped,
}

impl From<ProcessingStatus> for GqlProcessingStatus {
    fn from(s: ProcessingStatus) -> Self {
        match s {
            ProcessingStatus::Pending => GqlProcessingStatus::Pending,
            ProcessingStatus::Processing => GqlProcessingStatus::Processing,
            ProcessingStatus::Completed => GqlProcessingStatus::Completed,
            ProcessingStatus::Failed => GqlProcessingStatus::Failed,
            ProcessingStatus::Skipped => GqlProcessingStatus::Skipped,
        }
    }
}

/// Sync direction enum for GraphQL.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlSyncDirection {
    Inbound,
    Outbound,
    Both,
}

impl From<SyncDirection> for GqlSyncDirection {
    fn from(d: SyncDirection) -> Self {
        match d {
            SyncDirection::Inbound => GqlSyncDirection::Inbound,
            SyncDirection::Outbound => GqlSyncDirection::Outbound,
            SyncDirection::Both => GqlSyncDirection::Both,
        }
    }
}

/// Webhook endpoint GraphQL type.
#[derive(SimpleObject)]
pub struct GqlWebhookEndpoint {
    pub id: String,
    pub tenant_id: String,
    pub repository_id: Option<String>,
    pub name: String,
    pub provider: GqlProvider,
    pub config: String, // JSON string
    pub events: Vec<String>,
    pub mapping: Option<String>, // JSON string
    pub status: GqlEndpointStatus,
    pub webhook_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GqlWebhookEndpoint {
    pub fn from_domain(endpoint: WebhookEndpoint, base_url: &str) -> Self {
        // Convert Provider to OAuthProvider then to GqlProvider
        let oauth_provider: OAuthProvider = (*endpoint.provider()).into();

        Self {
            id: endpoint.id().to_string(),
            tenant_id: endpoint.tenant_id().to_string(),
            repository_id: endpoint.repository_id().map(String::from),
            name: endpoint.name().clone(),
            provider: oauth_provider.into(),
            config: serde_json::to_string(endpoint.config())
                .unwrap_or_default(),
            events: endpoint.events().clone(),
            mapping: endpoint
                .mapping()
                .map(|m| serde_json::to_string(m).unwrap_or_default()),
            status: (*endpoint.status()).into(),
            webhook_url: endpoint.webhook_url(base_url),
            created_at: *endpoint.created_at(),
            updated_at: *endpoint.updated_at(),
        }
    }
}

/// Processing stats GraphQL type.
#[derive(SimpleObject)]
pub struct GqlProcessingStats {
    pub created: u32,
    pub updated: u32,
    pub deleted: u32,
    pub skipped: u32,
    pub total: u32,
}

impl From<ProcessingStats> for GqlProcessingStats {
    fn from(s: ProcessingStats) -> Self {
        Self {
            created: s.created,
            updated: s.updated,
            deleted: s.deleted,
            skipped: s.skipped,
            total: s.total(),
        }
    }
}

/// Webhook event GraphQL type.
#[derive(SimpleObject)]
pub struct GqlWebhookEvent {
    pub id: String,
    pub endpoint_id: String,
    pub provider: GqlProvider,
    pub event_type: String,
    pub payload: String, // JSON string (truncated for large payloads)
    pub signature_valid: bool,
    pub status: GqlProcessingStatus,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub stats: Option<GqlProcessingStats>,
    pub received_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

impl From<WebhookEvent> for GqlWebhookEvent {
    fn from(event: WebhookEvent) -> Self {
        // Truncate large payloads
        let payload_str =
            serde_json::to_string(event.payload()).unwrap_or_default();
        let payload = if payload_str.len() > 10000 {
            format!("{}...(truncated)", &payload_str[..10000])
        } else {
            payload_str
        };

        // Convert Provider to OAuthProvider then to GqlProvider
        let oauth_provider: OAuthProvider = (*event.provider()).into();

        Self {
            id: event.id().to_string(),
            endpoint_id: event.endpoint_id().to_string(),
            provider: oauth_provider.into(),
            event_type: event.event_type().clone(),
            payload,
            signature_valid: *event.signature_valid(),
            status: (*event.status()).into(),
            error_message: event.error_message().map(String::from),
            retry_count: *event.retry_count(),
            stats: event.stats().cloned().map(Into::into),
            received_at: *event.received_at(),
            processed_at: event.processed_at(),
        }
    }
}

/// Sync state GraphQL type.
#[derive(SimpleObject)]
pub struct GqlSyncState {
    pub id: String,
    pub endpoint_id: String,
    pub data_id: String,
    pub external_id: String,
    pub external_version: Option<String>,
    pub local_version: Option<String>,
    pub sync_direction: GqlSyncDirection,
    pub last_synced_at: DateTime<Utc>,
}

impl From<SyncState> for GqlSyncState {
    fn from(state: SyncState) -> Self {
        Self {
            id: state.id().to_string(),
            endpoint_id: state.endpoint_id().to_string(),
            data_id: state.data_id().clone(),
            external_id: state.external_id().clone(),
            external_version: state.external_version().map(String::from),
            local_version: state.local_version().map(String::from),
            sync_direction: (*state.sync_direction()).into(),
            last_synced_at: *state.last_synced_at(),
        }
    }
}

// Input types

/// Input for creating a webhook endpoint.
#[derive(InputObject)]
pub struct CreateWebhookEndpointInput {
    pub name: String,
    pub provider: GqlProvider,
    /// Provider-specific configuration as JSON string
    pub config: String,
    /// Events to listen for (empty = all events)
    pub events: Vec<String>,
    /// Target repository ID (optional)
    pub repository_id: Option<String>,
    /// Property mapping as JSON string (optional)
    pub mapping: Option<String>,
}

/// Input for updating webhook endpoint status.
#[derive(InputObject)]
pub struct UpdateEndpointStatusInput {
    pub endpoint_id: String,
    pub status: GqlEndpointStatus,
}

/// Input for updating webhook endpoint events.
#[derive(InputObject)]
pub struct UpdateEndpointEventsInput {
    pub endpoint_id: String,
    pub events: Vec<String>,
}

/// Input for updating webhook endpoint mapping.
#[derive(InputObject)]
pub struct UpdateEndpointMappingInput {
    pub endpoint_id: String,
    /// Property mapping as JSON string (null to remove mapping)
    pub mapping: Option<String>,
}

/// Input for updating webhook endpoint configuration.
#[derive(InputObject)]
pub struct UpdateEndpointConfigInput {
    pub endpoint_id: String,
    /// Provider-specific configuration as JSON string
    pub config: String,
}

/// Output for creating a webhook endpoint.
#[derive(SimpleObject)]
pub struct CreateWebhookEndpointOutput {
    pub endpoint: GqlWebhookEndpoint,
    pub webhook_url: String,
    /// The secret (only returned on creation)
    pub secret: String,
}

// ============================================================================
// Marketplace Types
// ============================================================================

/// Integration category for GraphQL.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlIntegrationCategory {
    /// Code and repository management (GitHub, GitLab)
    CodeManagement,
    /// Issue and project tracking (Linear, Jira)
    ProjectManagement,
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

impl From<IntegrationCategory> for GqlIntegrationCategory {
    fn from(c: IntegrationCategory) -> Self {
        match c {
            IntegrationCategory::CodeManagement => {
                GqlIntegrationCategory::CodeManagement
            }
            IntegrationCategory::ProjectManagement => {
                GqlIntegrationCategory::ProjectManagement
            }
            IntegrationCategory::Communication => {
                GqlIntegrationCategory::Custom
            }
            IntegrationCategory::Crm => GqlIntegrationCategory::Crm,
            IntegrationCategory::Payments => {
                GqlIntegrationCategory::Payments
            }
            IntegrationCategory::ContentManagement => {
                GqlIntegrationCategory::ContentManagement
            }
            IntegrationCategory::Ecommerce => {
                GqlIntegrationCategory::Ecommerce
            }
            IntegrationCategory::Custom => GqlIntegrationCategory::Custom,
        }
    }
}

impl From<GqlIntegrationCategory> for IntegrationCategory {
    fn from(c: GqlIntegrationCategory) -> Self {
        match c {
            GqlIntegrationCategory::CodeManagement => {
                IntegrationCategory::CodeManagement
            }
            GqlIntegrationCategory::ProjectManagement => {
                IntegrationCategory::ProjectManagement
            }
            GqlIntegrationCategory::Crm => IntegrationCategory::Crm,
            GqlIntegrationCategory::Payments => {
                IntegrationCategory::Payments
            }
            GqlIntegrationCategory::ContentManagement => {
                IntegrationCategory::ContentManagement
            }
            GqlIntegrationCategory::Ecommerce => {
                IntegrationCategory::Ecommerce
            }
            GqlIntegrationCategory::Custom => IntegrationCategory::Custom,
        }
    }
}

/// Sync capability for GraphQL.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlSyncCapability {
    /// Can receive data from external service (webhooks)
    Inbound,
    /// Can push data to external service
    Outbound,
    /// Can both receive and push data
    Bidirectional,
}

impl From<SyncCapability> for GqlSyncCapability {
    fn from(c: SyncCapability) -> Self {
        match c {
            SyncCapability::Inbound => GqlSyncCapability::Inbound,
            SyncCapability::Outbound => GqlSyncCapability::Outbound,
            SyncCapability::Bidirectional => {
                GqlSyncCapability::Bidirectional
            }
        }
    }
}

/// Connection status for GraphQL.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlConnectionStatus {
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

impl From<ConnectionStatus> for GqlConnectionStatus {
    fn from(s: ConnectionStatus) -> Self {
        match s {
            ConnectionStatus::Active => GqlConnectionStatus::Active,
            ConnectionStatus::Expired => GqlConnectionStatus::Expired,
            ConnectionStatus::Paused => GqlConnectionStatus::Paused,
            ConnectionStatus::Disconnected => {
                GqlConnectionStatus::Disconnected
            }
            ConnectionStatus::Error => GqlConnectionStatus::Error,
        }
    }
}

/// OAuth configuration for an integration.
#[derive(SimpleObject, Clone)]
pub struct GqlOAuthConfig {
    /// Required OAuth scopes for this integration
    pub scopes: Vec<String>,
    /// OAuth authorization URL
    pub auth_url: String,
    /// OAuth token URL
    pub token_url: String,
    /// Whether refresh tokens are supported
    pub supports_refresh: bool,
}

/// Integration available in the marketplace.
#[derive(SimpleObject, Clone)]
pub struct GqlIntegration {
    /// Unique identifier
    pub id: String,
    /// The provider this integration is for
    pub provider: GqlProvider,
    /// Display name
    pub name: String,
    /// Description of the integration
    pub description: String,
    /// Icon URL or identifier
    pub icon: Option<String>,
    /// Category
    pub category: GqlIntegrationCategory,
    /// Sync capabilities
    pub sync_capability: GqlSyncCapability,
    /// Supported object types (e.g., "customers", "orders", "products")
    pub supported_objects: Vec<String>,
    /// Whether OAuth is required for this integration
    pub requires_oauth: bool,
    /// OAuth configuration (if applicable)
    pub oauth_config: Option<GqlOAuthConfig>,
    /// Whether this integration is enabled in the marketplace
    pub is_enabled: bool,
    /// Whether this is a featured/recommended integration
    pub is_featured: bool,
}

impl From<Integration> for GqlIntegration {
    fn from(i: Integration) -> Self {
        Self {
            id: i.id().to_string(),
            provider: i.provider().into(),
            name: i.name().to_string(),
            description: i.description().to_string(),
            icon: i.icon().map(String::from),
            category: i.category().into(),
            sync_capability: i.sync_capability().into(),
            supported_objects: i.supported_objects().to_vec(),
            requires_oauth: i.requires_oauth(),
            oauth_config: i.oauth_config().map(|c| GqlOAuthConfig {
                scopes: c.scopes.clone(),
                auth_url: c.auth_url.clone(),
                token_url: c.token_url.clone(),
                supports_refresh: c.supports_refresh,
            }),
            is_enabled: i.is_enabled(),
            is_featured: i.is_featured(),
        }
    }
}

/// Tenant's connection to an integration.
#[derive(SimpleObject, Clone)]
pub struct GqlConnection {
    /// Unique identifier
    pub id: String,
    /// Tenant ID that owns this connection
    pub tenant_id: String,
    /// Integration ID this connection is for
    pub integration_id: String,
    /// Provider name
    pub provider: GqlProvider,
    /// Current status
    pub status: GqlConnectionStatus,
    /// External account identifier (e.g., GitHub username)
    pub external_account_id: Option<String>,
    /// External account name for display
    pub external_account_name: Option<String>,
    /// When OAuth token expires (if applicable)
    pub token_expires_at: Option<DateTime<Utc>>,
    /// Error message if status is Error
    pub error_message: Option<String>,
    /// When the connection was created
    pub connected_at: DateTime<Utc>,
    /// When the connection was last synced
    pub last_synced_at: Option<DateTime<Utc>>,
}

impl From<Connection> for GqlConnection {
    fn from(c: Connection) -> Self {
        Self {
            id: c.id().to_string(),
            tenant_id: c.tenant_id().to_string(),
            integration_id: c.integration_id().to_string(),
            provider: c.provider().into(),
            status: c.status().into(),
            external_account_id: c.external_account_id().map(String::from),
            external_account_name: c
                .external_account_name()
                .map(String::from),
            token_expires_at: c.token_expires_at(),
            error_message: c.error_message().map(String::from),
            connected_at: c.connected_at(),
            last_synced_at: c.last_synced_at(),
        }
    }
}

// Marketplace Input Types

/// Input for connecting to an integration.
#[derive(InputObject, Debug)]
pub struct ConnectIntegrationInput {
    /// Integration ID to connect to
    pub integration_id: String,
    /// OAuth authorization code (for OAuth integrations)
    pub auth_code: Option<String>,
    /// API key or token (for non-OAuth integrations)
    pub api_key: Option<String>,
}

/// Input for updating connection status.
#[derive(InputObject, Debug)]
pub struct UpdateConnectionStatusInput {
    /// Connection ID to update
    pub connection_id: String,
    /// Action to perform
    pub action: GqlConnectionAction,
}

/// Actions for connection status changes.
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum GqlConnectionAction {
    /// Pause the connection
    Pause,
    /// Resume a paused connection
    Resume,
    /// Disconnect/remove the connection
    Disconnect,
    /// Re-authorize (refresh OAuth token)
    Reauthorize,
}

/// Input for exchanging OAuth authorization code.
#[derive(InputObject, Debug)]
pub struct ExchangeOAuthCodeInput {
    /// Integration ID
    pub integration_id: String,
    /// Authorization code from OAuth callback
    pub code: String,
    /// State parameter for CSRF verification
    pub state: Option<String>,
    /// Redirect URI used in authorization
    pub redirect_uri: String,
}

/// Input for initializing OAuth authorization.
#[derive(InputObject, Debug)]
pub struct InitOAuthInput {
    /// Integration ID to connect
    pub integration_id: String,
    /// Optional redirect URI (defaults to backend callback URL if not provided)
    pub redirect_uri: Option<String>,
    /// Optional custom state for CSRF protection (will be generated if not provided).
    /// Frontend can encode additional data (e.g., tenant_id, integration_id) in this field.
    pub state: Option<String>,
}

/// Output for OAuth authorization initialization.
#[derive(SimpleObject)]
pub struct OAuthInitOutput {
    /// URL to redirect user to for authorization
    pub authorization_url: String,
    /// State parameter for CSRF protection
    pub state: String,
}

/// Output for connection operations.
#[derive(SimpleObject)]
pub struct ConnectionOutput {
    /// The connection
    pub connection: GqlConnection,
    /// Success message
    pub message: String,
}

/// Sync operation type enum for GraphQL.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlSyncOperationType {
    Webhook,
    InitialSync,
    OnDemandPull,
    ScheduledSync,
}

impl From<inbound_sync_domain::SyncOperationType> for GqlSyncOperationType {
    fn from(t: inbound_sync_domain::SyncOperationType) -> Self {
        match t {
            inbound_sync_domain::SyncOperationType::Webhook => {
                GqlSyncOperationType::Webhook
            }
            inbound_sync_domain::SyncOperationType::InitialSync => {
                GqlSyncOperationType::InitialSync
            }
            inbound_sync_domain::SyncOperationType::OnDemandPull => {
                GqlSyncOperationType::OnDemandPull
            }
            inbound_sync_domain::SyncOperationType::ScheduledSync => {
                GqlSyncOperationType::ScheduledSync
            }
        }
    }
}

/// Sync operation status enum for GraphQL.
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlSyncOperationStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl From<inbound_sync_domain::SyncOperationStatus>
    for GqlSyncOperationStatus
{
    fn from(s: inbound_sync_domain::SyncOperationStatus) -> Self {
        match s {
            inbound_sync_domain::SyncOperationStatus::Queued => {
                GqlSyncOperationStatus::Queued
            }
            inbound_sync_domain::SyncOperationStatus::Running => {
                GqlSyncOperationStatus::Running
            }
            inbound_sync_domain::SyncOperationStatus::Completed => {
                GqlSyncOperationStatus::Completed
            }
            inbound_sync_domain::SyncOperationStatus::Failed => {
                GqlSyncOperationStatus::Failed
            }
            inbound_sync_domain::SyncOperationStatus::Cancelled => {
                GqlSyncOperationStatus::Cancelled
            }
        }
    }
}

/// Sync operation for GraphQL.
#[derive(SimpleObject)]
pub struct GqlSyncOperation {
    pub id: String,
    pub endpoint_id: String,
    pub operation_type: GqlSyncOperationType,
    pub status: GqlSyncOperationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub stats: Option<GqlProcessingStats>,
    pub error_message: Option<String>,
    pub progress: Option<String>,
}

impl GqlSyncOperation {
    pub fn from_domain(
        operation: inbound_sync_domain::SyncOperation,
    ) -> Self {
        Self {
            id: operation.id().to_string(),
            endpoint_id: operation.endpoint_id().to_string(),
            operation_type: (*operation.operation_type()).into(),
            status: (*operation.status()).into(),
            started_at: *operation.started_at(),
            completed_at: operation.completed_at(),
            stats: operation.stats().map(|s| {
                let total = s.created + s.updated + s.deleted + s.skipped;
                GqlProcessingStats {
                    created: s.created,
                    updated: s.updated,
                    deleted: s.deleted,
                    skipped: s.skipped,
                    total,
                }
            }),
            error_message: operation.error_message().map(String::from),
            progress: operation.progress().map(String::from),
        }
    }
}

/// Input for starting initial sync.
#[derive(InputObject)]
pub struct StartInitialSyncInput {
    pub endpoint_id: String,
}

/// Input for triggering on-demand sync.
#[derive(InputObject)]
pub struct TriggerSyncInput {
    pub endpoint_id: String,
    pub external_ids: Option<Vec<String>>,
}

/// Linear Team for GraphQL.
#[derive(SimpleObject, Debug, Clone)]
pub struct GqlLinearTeam {
    /// Team ID
    pub id: String,
    /// Team name
    pub name: String,
    /// Team key (e.g., "ENG")
    pub key: String,
}
