//! GraphQL mutation resolvers for library sync.

use async_graphql::{Context, Object, Result, SimpleObject};
use std::sync::Arc;
use tachyon_sdk::auth::MultiTenancyAction;

use inbound_sync_domain::{
    ApiKeyValidator, Connection, ConnectionId, ConnectionRepository,
    ConnectionStatus, ExchangeOAuthCodeInput as DomainExchangeInput,
    InitOAuthInput as DomainInitInput, IntegrationId,
    IntegrationRepository, OAuthService, PropertyMapping, ProviderConfig,
    WebhookEndpointId, WebhookEventId,
};

use crate::usecase::{
    DeleteWebhookEndpoint, DeleteWebhookEndpointInputData,
    DeleteWebhookEndpointInputPort, InitialSyncInputData,
    OnDemandPullInputData, RegisterWebhookEndpoint,
    RegisterWebhookEndpointInputData, RegisterWebhookEndpointInputPort,
    RetryWebhookEvent, SendTestWebhook, UpdateWebhookEndpoint,
    UpdateWebhookEndpointInputData, UpdateWebhookEndpointInputPort,
    WebhookEndpointUpdate,
};

use super::types::{
    ConnectIntegrationInput, CreateWebhookEndpointInput,
    CreateWebhookEndpointOutput, ExchangeOAuthCodeInput, GqlConnection,
    GqlConnectionAction, GqlSyncOperation, GqlWebhookEndpoint,
    GqlWebhookEvent, InitOAuthInput, OAuthInitOutput,
    StartInitialSyncInput, TriggerSyncInput, UpdateEndpointConfigInput,
    UpdateEndpointEventsInput, UpdateEndpointMappingInput,
    UpdateEndpointStatusInput,
};

/// Mutation resolver for library sync.
#[derive(Default)]
pub struct LibrarySyncMutation;

/// State required for library sync mutations.
pub struct LibrarySyncMutationState {
    pub register_endpoint: Arc<RegisterWebhookEndpoint>,
    pub update_endpoint: Arc<UpdateWebhookEndpoint>,
    pub delete_endpoint: Arc<DeleteWebhookEndpoint>,
    pub send_test_webhook: Option<Arc<SendTestWebhook>>,
    pub retry_webhook_event: Option<Arc<RetryWebhookEvent>>,
    pub initial_sync: Arc<crate::usecase::InitialSync>,
    pub on_demand_pull: Arc<crate::usecase::OnDemandPull>,
    pub operation_repo:
        Arc<dyn inbound_sync_domain::SyncOperationRepository>,
    pub integration_repository: Arc<dyn IntegrationRepository>,
    pub connection_repository: Arc<dyn ConnectionRepository>,
    pub oauth_service: Option<Arc<dyn OAuthService>>,
    pub api_key_validator: Option<Arc<dyn ApiKeyValidator>>,
    pub base_url: String,
}

/// Output for send test webhook mutation.
#[derive(SimpleObject)]
pub struct SendTestWebhookOutput {
    /// Whether the test was sent successfully
    pub success: bool,
    /// ID of the created event
    pub event_id: Option<String>,
}

#[Object]
impl LibrarySyncMutation {
    /// Create a new webhook endpoint.
    ///
    /// Returns the created endpoint along with the webhook URL and secret.
    /// **Important**: The secret is only returned once. Store it securely.
    async fn create_webhook_endpoint(
        &self,
        ctx: &Context<'_>,
        input: CreateWebhookEndpointInput,
    ) -> Result<CreateWebhookEndpointOutput> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        // Parse provider config from JSON
        let config: ProviderConfig = serde_json::from_str(&input.config)
            .map_err(|e| {
                async_graphql::Error::new(format!(
                    "Invalid config JSON: {e}"
                ))
            })?;

        // Parse mapping if provided
        let mapping: Option<PropertyMapping> = input
            .mapping
            .map(|m| serde_json::from_str(&m))
            .transpose()
            .map_err(|e| {
                async_graphql::Error::new(format!(
                    "Invalid mapping JSON: {e}"
                ))
            })?;

        // Convert GqlProvider -> OAuthProvider -> Provider
        let oauth_provider: inbound_sync_domain::OAuthProvider =
            input.provider.into();
        let provider: inbound_sync_domain::Provider = oauth_provider.into();

        let usecase_input = RegisterWebhookEndpointInputData {
            executor,
            multi_tenancy,
            name: input.name,
            provider,
            config,
            events: input.events,
            repository_id: input.repository_id,
            mapping,
        };

        let output = state.register_endpoint.execute(usecase_input).await?;

        Ok(CreateWebhookEndpointOutput {
            endpoint: GqlWebhookEndpoint::from_domain(
                output.endpoint,
                &state.base_url,
            ),
            webhook_url: output.webhook_url,
            secret: output.secret,
        })
    }

    /// Update webhook endpoint status.
    async fn update_webhook_endpoint_status(
        &self,
        ctx: &Context<'_>,
        input: UpdateEndpointStatusInput,
    ) -> Result<GqlWebhookEndpoint> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let endpoint_id = WebhookEndpointId::from(input.endpoint_id);

        let usecase_input = UpdateWebhookEndpointInputData {
            executor,
            multi_tenancy,
            endpoint_id: endpoint_id.clone(),
            update: WebhookEndpointUpdate::Status(input.status.into()),
        };

        let output = state.update_endpoint.execute(usecase_input).await?;

        Ok(GqlWebhookEndpoint::from_domain(
            output.endpoint,
            &state.base_url,
        ))
    }

    /// Update webhook endpoint events.
    async fn update_webhook_endpoint_events(
        &self,
        ctx: &Context<'_>,
        input: UpdateEndpointEventsInput,
    ) -> Result<GqlWebhookEndpoint> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let endpoint_id = WebhookEndpointId::from(input.endpoint_id);

        let usecase_input = UpdateWebhookEndpointInputData {
            executor,
            multi_tenancy,
            endpoint_id: endpoint_id.clone(),
            update: WebhookEndpointUpdate::Events(input.events),
        };

        let output = state.update_endpoint.execute(usecase_input).await?;

        Ok(GqlWebhookEndpoint::from_domain(
            output.endpoint,
            &state.base_url,
        ))
    }

    /// Update webhook endpoint property mapping.
    async fn update_webhook_endpoint_mapping(
        &self,
        ctx: &Context<'_>,
        input: UpdateEndpointMappingInput,
    ) -> Result<GqlWebhookEndpoint> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let endpoint_id = WebhookEndpointId::from(input.endpoint_id);

        // Parse mapping if provided
        let mapping: Option<PropertyMapping> = input
            .mapping
            .map(|m| serde_json::from_str(&m))
            .transpose()
            .map_err(|e| {
                async_graphql::Error::new(format!(
                    "Invalid mapping JSON: {e}"
                ))
            })?;

        let usecase_input = UpdateWebhookEndpointInputData {
            executor,
            multi_tenancy,
            endpoint_id: endpoint_id.clone(),
            update: WebhookEndpointUpdate::Mapping(mapping),
        };

        let output = state.update_endpoint.execute(usecase_input).await?;

        Ok(GqlWebhookEndpoint::from_domain(
            output.endpoint,
            &state.base_url,
        ))
    }

    /// Update webhook endpoint configuration.
    async fn update_webhook_endpoint_config(
        &self,
        ctx: &Context<'_>,
        input: UpdateEndpointConfigInput,
    ) -> Result<GqlWebhookEndpoint> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let endpoint_id = WebhookEndpointId::from(input.endpoint_id);

        // Parse provider config from JSON
        let config: ProviderConfig = serde_json::from_str(&input.config)
            .map_err(|e| {
                async_graphql::Error::new(format!(
                    "Invalid config JSON: {e}"
                ))
            })?;

        let usecase_input = UpdateWebhookEndpointInputData {
            executor,
            multi_tenancy,
            endpoint_id: endpoint_id.clone(),
            update: WebhookEndpointUpdate::Config(config),
        };

        let output = state.update_endpoint.execute(usecase_input).await?;

        Ok(GqlWebhookEndpoint::from_domain(
            output.endpoint,
            &state.base_url,
        ))
    }

    /// Delete a webhook endpoint.
    async fn delete_webhook_endpoint(
        &self,
        ctx: &Context<'_>,
        endpoint_id: String,
    ) -> Result<bool> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let id = WebhookEndpointId::from(endpoint_id);

        let usecase_input = DeleteWebhookEndpointInputData {
            executor,
            multi_tenancy,
            endpoint_id: id,
        };

        state.delete_endpoint.execute(usecase_input).await?;

        Ok(true)
    }

    /// Send a test webhook to an endpoint.
    async fn send_test_webhook(
        &self,
        ctx: &Context<'_>,
        endpoint_id: String,
        event_type: String,
    ) -> Result<SendTestWebhookOutput> {
        let state = ctx.data::<LibrarySyncMutationState>()?;

        let send_test =
            state.send_test_webhook.as_ref().ok_or_else(|| {
                async_graphql::Error::new(
                    "Test webhook feature is not enabled",
                )
            })?;

        let endpoint_id = WebhookEndpointId::from(endpoint_id);
        let event = send_test.execute(&endpoint_id, &event_type).await?;

        Ok(SendTestWebhookOutput {
            success: true,
            event_id: Some(event.id().to_string()),
        })
    }

    /// Retry a failed webhook event.
    async fn retry_webhook_event(
        &self,
        ctx: &Context<'_>,
        event_id: String,
    ) -> Result<GqlWebhookEvent> {
        let state = ctx.data::<LibrarySyncMutationState>()?;

        let retry =
            state.retry_webhook_event.as_ref().ok_or_else(|| {
                async_graphql::Error::new(
                    "Event retry feature is not enabled",
                )
            })?;

        let event_id = WebhookEventId::from(event_id);
        let event = retry.execute(&event_id).await?;

        Ok(event.into())
    }

    // =========================================================================
    // Connection Management Mutations
    // =========================================================================

    /// Connect to an integration.
    ///
    /// Creates a new connection to the specified integration.
    /// For OAuth integrations, provide the authorization code.
    /// For API key integrations, provide the API key.
    async fn connect_integration(
        &self,
        ctx: &Context<'_>,
        input: ConnectIntegrationInput,
    ) -> Result<GqlConnection> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let tenant = multi_tenancy
            .get_operator_id()
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let integration_id = IntegrationId::new(&input.integration_id);

        // Verify integration exists
        let integration = state
            .integration_repository
            .find_by_id(&integration_id)
            .await?
            .ok_or_else(|| {
                async_graphql::Error::new("Integration not found")
            })?;

        // Check if connection already exists
        if let Some(existing) = state
            .connection_repository
            .find_by_tenant_and_integration(&tenant, &integration_id)
            .await?
        {
            // If disconnected, we can reconnect; otherwise error
            if existing.status() != ConnectionStatus::Disconnected {
                return Err(async_graphql::Error::new(
                    "Connection already exists for this integration",
                ));
            }
        }

        // Determine authentication method and validate
        let (external_account_id, external_account_name) = if let Some(
            api_key,
        ) =
            &input.api_key
        {
            // Validate API key
            let validator =
                state.api_key_validator.as_ref().ok_or_else(|| {
                    async_graphql::Error::new(
                        "API key validation is not configured",
                    )
                })?;

            let result = validator
                .validate(integration.provider(), api_key)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;

            if !result.is_valid {
                return Err(async_graphql::Error::new(
                    result
                        .error_message
                        .unwrap_or_else(|| "Invalid API key".to_string()),
                ));
            }

            (result.external_account_id, result.external_account_name)
        } else if input.auth_code.is_some() {
            // OAuth flow should use exchangeOAuthCode mutation instead
            return Err(async_graphql::Error::new(
                "For OAuth integrations, use initOAuth and exchangeOAuthCode mutations",
            ));
        } else {
            // No credentials provided, create connection without external account info
            (None, None)
        };

        // Create the connection
        let mut connection = Connection::create(
            ConnectionId::generate(),
            tenant,
            integration_id,
            integration.provider(),
        );

        // Set external account info if available
        connection.set_external_account(
            external_account_id,
            external_account_name,
        );

        state.connection_repository.save(&connection).await?;

        Ok(connection.into())
    }

    /// Update a connection's status.
    ///
    /// Allows pausing, resuming, disconnecting, or reauthorizing a connection.
    async fn update_connection(
        &self,
        ctx: &Context<'_>,
        connection_id: String,
        action: GqlConnectionAction,
    ) -> Result<GqlConnection> {
        let state = ctx.data::<LibrarySyncMutationState>()?;

        let id = ConnectionId::new(&connection_id);

        let mut connection = state
            .connection_repository
            .find_by_id(&id)
            .await?
            .ok_or_else(|| {
                async_graphql::Error::new("Connection not found")
            })?;

        match action {
            GqlConnectionAction::Pause => {
                connection.pause();
            }
            GqlConnectionAction::Resume => {
                connection.resume();
            }
            GqlConnectionAction::Disconnect => {
                connection.disconnect();
            }
            GqlConnectionAction::Reauthorize => {
                // TODO: Implement reauthorization flow
                // For now, just mark as active
                connection.resume();
            }
        }

        state.connection_repository.save(&connection).await?;

        Ok(connection.into())
    }

    /// Delete a connection permanently.
    async fn delete_connection(
        &self,
        ctx: &Context<'_>,
        connection_id: String,
    ) -> Result<bool> {
        let state = ctx.data::<LibrarySyncMutationState>()?;

        let id = ConnectionId::new(&connection_id);

        state.connection_repository.delete(&id).await?;

        Ok(true)
    }

    // =========================================================================
    // OAuth Mutations
    // =========================================================================

    /// Initialize OAuth authorization flow.
    ///
    /// Returns the authorization URL to redirect the user to.
    async fn init_oauth(
        &self,
        ctx: &Context<'_>,
        input: InitOAuthInput,
    ) -> Result<OAuthInitOutput> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let oauth = state.oauth_service.as_ref().ok_or_else(|| {
            async_graphql::Error::new("OAuth service is not configured")
        })?;

        let tenant = multi_tenancy
            .get_operator_id()
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let integration_id = IntegrationId::new(&input.integration_id);

        // Find the integration to get the provider
        let integration = state
            .integration_repository
            .find_by_id(&integration_id)
            .await?
            .ok_or_else(|| {
                async_graphql::Error::new("Integration not found")
            })?;

        if !integration.requires_oauth() {
            return Err(async_graphql::Error::new(
                "This integration does not use OAuth authentication",
            ));
        }

        let domain_input = DomainInitInput {
            tenant_id: tenant,
            provider: integration.provider(),
            base_url: state.base_url.clone(),
            redirect_uri: input.redirect_uri,
            state: input.state, // Use provided state or generate if None
        };

        let output = oauth.init_authorization(domain_input).await?;

        Ok(OAuthInitOutput {
            authorization_url: output.authorization_url,
            state: output.state,
        })
    }

    /// Exchange OAuth authorization code for tokens.
    ///
    /// Creates or updates a connection with the tokens from the OAuth provider.
    async fn exchange_oauth_code(
        &self,
        ctx: &Context<'_>,
        input: ExchangeOAuthCodeInput,
    ) -> Result<GqlConnection> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let oauth = state.oauth_service.as_ref().ok_or_else(|| {
            async_graphql::Error::new("OAuth service is not configured")
        })?;

        let tenant = multi_tenancy
            .get_operator_id()
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let integration_id = IntegrationId::new(&input.integration_id);

        // Find the integration to get the provider
        let integration = state
            .integration_repository
            .find_by_id(&integration_id)
            .await?
            .ok_or_else(|| {
                async_graphql::Error::new("Integration not found")
            })?;

        // Exchange the authorization code for tokens
        let domain_input = DomainExchangeInput {
            tenant_id: tenant.clone(),
            provider: integration.provider(),
            code: input.code,
            state: input.state,
            redirect_uri: input.redirect_uri,
        };

        let stored_token = oauth.exchange_code(domain_input).await?;

        // Create or update the connection
        let existing = state
            .connection_repository
            .find_by_tenant_and_integration(&tenant, &integration_id)
            .await?;

        let connection = if let Some(mut conn) = existing {
            // Update existing connection with new token info
            conn.set_external_account(
                stored_token.external_account_id.clone(),
                stored_token.external_account_name.clone(),
            );
            conn.set_token_expires_at(stored_token.expires_at);
            conn.resume(); // Re-activate if it was paused/expired
            conn
        } else {
            // Create new connection
            let mut conn = Connection::create(
                ConnectionId::generate(),
                tenant,
                integration_id,
                integration.provider(),
            );
            conn.set_external_account(
                stored_token.external_account_id.clone(),
                stored_token.external_account_name.clone(),
            );
            conn.set_token_expires_at(stored_token.expires_at);
            conn
        };

        state.connection_repository.save(&connection).await?;

        Ok(connection.into())
    }

    /// Start initial sync operation.
    async fn start_initial_sync(
        &self,
        ctx: &Context<'_>,
        input: StartInitialSyncInput,
    ) -> Result<GqlSyncOperation> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let endpoint_id = WebhookEndpointId::from(input.endpoint_id);

        let usecase_input = InitialSyncInputData {
            executor,
            multi_tenancy,
            endpoint_id: endpoint_id.clone(),
        };

        let output = state.initial_sync.execute(usecase_input).await?;

        // Fetch the created operation
        let operation = state
            .operation_repo
            .find_by_id(&output.operation_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Sync operation"))?;

        Ok(GqlSyncOperation::from_domain(operation))
    }

    // =========================================================================
    // GitHub App Mutations
    // =========================================================================

    /// Complete GitHub App installation.
    ///
    /// Called after the user installs the GitHub App. GitHub redirects to the
    /// callback URL with `installation_id` and `state` (hex-encoded
    /// `tenant_id:integration_id`). This mutation creates a Connection with
    /// `installation_id` stored in metadata.
    async fn complete_github_install(
        &self,
        ctx: &Context<'_>,
        installation_id: i64,
        integration_id: String,
    ) -> Result<GqlConnection> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let tenant = multi_tenancy
            .get_operator_id()
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let int_id = IntegrationId::new(&integration_id);

        let integration = state
            .integration_repository
            .find_by_id(&int_id)
            .await?
            .ok_or_else(|| {
                async_graphql::Error::new("Integration not found")
            })?;

        // Create or update connection
        let existing = state
            .connection_repository
            .find_by_tenant_and_integration(&tenant, &int_id)
            .await?;

        let mut connection = if let Some(mut conn) = existing {
            conn.resume();
            conn
        } else {
            Connection::create(
                ConnectionId::generate(),
                tenant,
                int_id,
                integration.provider(),
            )
        };

        connection.set_metadata(
            "installation_id",
            serde_json::json!(installation_id),
        );

        state.connection_repository.save(&connection).await?;

        Ok(connection.into())
    }

    /// Trigger on-demand pull sync.
    async fn trigger_sync(
        &self,
        ctx: &Context<'_>,
        input: TriggerSyncInput,
    ) -> Result<GqlSyncOperation> {
        let state = ctx.data::<LibrarySyncMutationState>()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let endpoint_id = WebhookEndpointId::from(input.endpoint_id);

        let usecase_input = OnDemandPullInputData {
            executor,
            multi_tenancy,
            endpoint_id: endpoint_id.clone(),
            external_ids: input.external_ids,
        };

        let output = state.on_demand_pull.execute(usecase_input).await?;

        // Fetch the created operation
        let operation = state
            .operation_repo
            .find_by_id(&output.operation_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Sync operation"))?;

        Ok(GqlSyncOperation::from_domain(operation))
    }
}
