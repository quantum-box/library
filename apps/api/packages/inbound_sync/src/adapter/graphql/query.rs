//! GraphQL query resolvers for library sync.

use async_graphql::{Context, Object, Result};
use std::sync::Arc;

use inbound_sync_domain::{
    ConnectionRepository, IntegrationRepository, WebhookEndpointId,
    WebhookEndpointRepository, WebhookEventRepository,
};
use value_object::TenantId;

use super::types::{
    GqlConnection, GqlIntegration, GqlIntegrationCategory, GqlProvider,
    GqlWebhookEndpoint, GqlWebhookEvent,
};

/// Query resolver for library sync.
#[derive(Default)]
pub struct LibrarySyncQuery;

/// State required for library sync queries.
pub struct LibrarySyncQueryState {
    pub endpoint_repository: Arc<dyn WebhookEndpointRepository>,
    pub event_repository: Arc<dyn WebhookEventRepository>,
    pub operation_repo:
        Arc<dyn inbound_sync_domain::SyncOperationRepository>,
    pub integration_repository: Arc<dyn IntegrationRepository>,
    pub connection_repository: Arc<dyn ConnectionRepository>,
    pub base_url: String,
}

#[Object]
impl LibrarySyncQuery {
    /// Get a webhook endpoint by ID.
    async fn webhook_endpoint(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<GqlWebhookEndpoint>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;
        let endpoint_id = WebhookEndpointId::from(id);

        let endpoint =
            state.endpoint_repository.find_by_id(&endpoint_id).await?;

        Ok(endpoint
            .map(|e| GqlWebhookEndpoint::from_domain(e, &state.base_url)))
    }

    /// List webhook endpoints for a tenant.
    async fn webhook_endpoints(
        &self,
        ctx: &Context<'_>,
        tenant_id: String,
        provider: Option<GqlProvider>,
        repository_id: Option<String>,
    ) -> Result<Vec<GqlWebhookEndpoint>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;
        let tenant = tenant_id
            .parse::<TenantId>()
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let endpoints = if let Some(p) = provider {
            // Convert GqlProvider -> OAuthProvider -> Provider
            let oauth_provider: inbound_sync_domain::OAuthProvider =
                p.into();
            let provider: inbound_sync_domain::Provider =
                oauth_provider.into();

            state
                .endpoint_repository
                .find_by_tenant_and_provider(&tenant, provider)
                .await?
        } else {
            state.endpoint_repository.find_by_tenant(&tenant).await?
        };

        let filtered = if let Some(repo_id) = repository_id {
            endpoints
                .into_iter()
                .filter(|e| e.repository_id() == Some(repo_id.as_str()))
                .collect::<Vec<_>>()
        } else {
            endpoints
        };

        Ok(filtered
            .into_iter()
            .map(|e| GqlWebhookEndpoint::from_domain(e, &state.base_url))
            .collect())
    }

    /// Get webhook events for an endpoint.
    async fn webhook_events(
        &self,
        ctx: &Context<'_>,
        endpoint_id: String,
        #[graphql(default = 50)] limit: u32,
        #[graphql(default = 0)] offset: u32,
    ) -> Result<Vec<GqlWebhookEvent>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;
        let id = WebhookEndpointId::from(endpoint_id);

        let events = state
            .event_repository
            .find_by_endpoint(&id, limit, offset)
            .await?;

        Ok(events.into_iter().map(Into::into).collect())
    }

    /// Get a single webhook event by ID.
    async fn webhook_event(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<GqlWebhookEvent>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;
        let event_id = inbound_sync_domain::WebhookEventId::from(id);

        let event = state.event_repository.find_by_id(&event_id).await?;

        Ok(event.map(Into::into))
    }

    // =========================================================================
    // Marketplace Queries
    // =========================================================================

    /// Get all available integrations in the marketplace.
    async fn integrations(
        &self,
        ctx: &Context<'_>,
        category: Option<GqlIntegrationCategory>,
        featured_only: Option<bool>,
    ) -> Result<Vec<GqlIntegration>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;

        let integrations = if featured_only.unwrap_or(false) {
            state.integration_repository.find_featured().await?
        } else if let Some(cat) = category {
            state
                .integration_repository
                .find_by_category(cat.into())
                .await?
        } else {
            state.integration_repository.find_enabled().await?
        };

        Ok(integrations.into_iter().map(Into::into).collect())
    }

    /// Get a single integration by ID.
    async fn integration(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<GqlIntegration>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;
        let integration_id = inbound_sync_domain::IntegrationId::new(id);

        let integration = state
            .integration_repository
            .find_by_id(&integration_id)
            .await?;

        Ok(integration.map(Into::into))
    }

    /// Get an integration by provider.
    async fn integration_by_provider(
        &self,
        ctx: &Context<'_>,
        provider: GqlProvider,
    ) -> Result<Option<GqlIntegration>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;

        let integration = state
            .integration_repository
            .find_by_provider(provider.into())
            .await?;

        Ok(integration.map(Into::into))
    }

    /// Get all connections for a tenant.
    async fn connections(
        &self,
        ctx: &Context<'_>,
        tenant_id: String,
        active_only: Option<bool>,
    ) -> Result<Vec<GqlConnection>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;
        let tenant = tenant_id
            .parse::<TenantId>()
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let connections = if active_only.unwrap_or(false) {
            state
                .connection_repository
                .find_active_by_tenant(&tenant)
                .await?
        } else {
            state.connection_repository.find_by_tenant(&tenant).await?
        };

        Ok(connections.into_iter().map(Into::into).collect())
    }

    /// Get a single connection by ID.
    async fn connection(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<GqlConnection>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;
        let connection_id = inbound_sync_domain::ConnectionId::new(id);

        let connection = state
            .connection_repository
            .find_by_id(&connection_id)
            .await?;

        Ok(connection.map(Into::into))
    }

    /// Get a single sync operation by ID.
    async fn sync_operation(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<super::types::GqlSyncOperation>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;
        let operation_id = inbound_sync_domain::SyncOperationId::from(id);

        let operation =
            state.operation_repo.find_by_id(&operation_id).await?;

        Ok(operation.map(super::types::GqlSyncOperation::from_domain))
    }

    /// Get sync operations for an endpoint.
    async fn sync_operations(
        &self,
        ctx: &Context<'_>,
        endpoint_id: String,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<super::types::GqlSyncOperation>> {
        let state = ctx.data::<LibrarySyncQueryState>()?;
        let endpoint_id =
            inbound_sync_domain::WebhookEndpointId::from(endpoint_id);

        let operations = state
            .operation_repo
            .find_by_endpoint(
                &endpoint_id,
                limit.unwrap_or(20) as u32,
                offset.unwrap_or(0) as u32,
            )
            .await?;

        Ok(operations
            .into_iter()
            .map(super::types::GqlSyncOperation::from_domain)
            .collect())
    }
}
