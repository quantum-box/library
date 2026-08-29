use crate::app::LibraryApp;
use crate::collaboration::handler::CollaborationState;
use crate::collaboration::manager::DocumentManager;
use crate::collaboration::persistence::SqlxDocumentPersistence;
use crate::database_layout::DatabasePools;
use crate::handler;
use crate::handler::graphql;
use crate::interface_adapter::gateway::LibraryDataRepositoryImpl;
use crate::sdk_auth::SdkAuthApp;
use async_graphql::EmptySubscription;
use async_graphql::Schema;
use axum::extract::DefaultBodyLimit;
use axum::http::method::Method;
use axum::middleware;
use axum::routing::get;
use axum::routing::post;
use axum::Extension;
use axum::Json;
use persistence::{MinioConfiguration, MinioDriver, S3Driver, Storage};
use serde::Serialize;
use std::sync::Arc;
use telemetry::http::{
    create_propagate_request_id_layer, create_request_id_layer,
    create_trace_layer,
};
use tower_http::cors::{Any, CorsLayer};

use crate::handler::data::ParquetStorage;
use crate::handler::image::{
    BucketImageStore, ImageObjectStore, TachyonImageStore, MAX_IMAGE_BYTES,
};

use inbound_sync::interface_adapter::{
    BuiltinIntegrationRegistry, HttpApiKeyValidator, NoOpHubSpotClient,
    NoOpHubSpotDataHandler, NoOpNotionClient, NoOpNotionDataHandler,
    NoOpSquareClient, NoOpStripeClient, NoOpStripeDataHandler,
    SqlxConnectionRepository, SqlxSyncStateRepository,
};
use inbound_sync::providers::github::{
    DefaultGitHubDataHandler, OAuthGitHubClient,
};
use inbound_sync::providers::linear::{
    DefaultLinearDataHandler, OAuthLinearClient,
};
use inbound_sync::providers::{
    GitHubEventProcessor, HubSpotEventProcessor, LinearEventProcessor,
    NotionEventProcessor, SquareEventProcessor, StripeEventProcessor,
};
use inbound_sync::sdk::AuthAppTokenProvider;
use inbound_sync::usecase::{
    EventProcessorRegistry, ProcessWebhookEvent, WebhookEventWorker,
};
use inbound_sync::{
    ApiKeyValidator, ConnectionRepository, IntegrationRepository,
    WebhookSecretStore,
};

const COLLAB_WS_ENABLED_ENV: &str = "LIBRARY_COLLAB_WS_ENABLED";
const WEBHOOK_WORKER_ENABLED_ENV: &str = "LIBRARY_WEBHOOK_WORKER_ENABLED";

#[derive(Serialize)]
struct VersionResponse {
    version: &'static str,
}

fn collaboration_ws_enabled() -> bool {
    std::env::var(COLLAB_WS_ENABLED_ENV)
        .map(|value| env_flag_enabled(&value))
        .unwrap_or(false)
}

fn env_flag_enabled(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("true")
}

// Every argument is a distinct runtime dependency the router wires
// into handlers. Grouping the OAuth-related ones behind a struct would
// read better and is worth doing, but not as a side effect of a
// latency fix.
#[allow(clippy::too_many_arguments)]
pub async fn router(
    pools: DatabasePools,
    sdk: Arc<SdkAuthApp>,
    database_app: Arc<database_manager::App>,
    github: Arc<github_provider::GitHub>,
    oauth_service: Arc<dyn inbound_sync_domain::OAuthService>,
    oauth_token_repo: Arc<dyn inbound_sync_domain::OAuthTokenRepository>,
    provider_secrets: Arc<WebhookSecretStore>,
    oauth_bootstrap: Arc<crate::oauth_bootstrap::OAuthBootstrap>,
) -> Result<axum::Router, Box<dyn std::error::Error>> {
    // Each repository executes unqualified SQL on the pool for its logical
    // role. Production resolves these roles to two physical databases, while
    // ADR-0049 previews intentionally resolve both to the injected PR DB.
    let DatabasePools {
        library: library_db,
        database_manager: database_manager_db,
    } = pools;
    let _db_pool_metric_tasks =
        crate::db_pool_metrics::start_default_pool_acquire_metrics([
            ("library", library_db.pool()),
            ("database_manager", database_manager_db.pool()),
        ]);

    // Database sync setup (must be created before LibraryApp)
    let sync_config_repo: Arc<dyn outbound_sync::SyncConfigRepository> =
        Arc::new(
            outbound_sync::interface_adapter::SqlxSyncConfigRepository::new(
                library_db.pool(),
            ),
        );
    let sync_provider_registry =
        Arc::new(outbound_sync::build_default_registry());
    let auth_app_trait: Arc<dyn tachyon_sdk::auth::AuthApp> = sdk.clone();
    let sync_data: Arc<dyn outbound_sync::SyncDataInputPort> =
        Arc::new(outbound_sync::SyncData::new(
            auth_app_trait.clone(),
            sync_config_repo.clone(),
            sync_provider_registry,
        ));

    // Library Sync (Inbound Webhooks) setup
    let webhook_endpoint_repo: Arc<
        dyn inbound_sync::WebhookEndpointRepository,
    > = Arc::new(
        inbound_sync::interface_adapter::SqlxWebhookEndpointRepository::new(
            library_db.pool(),
        ),
    );
    let webhook_event_repo: Arc<dyn inbound_sync::WebhookEventRepository> =
        Arc::new(
            inbound_sync::interface_adapter::SqlxWebhookEventRepository::new(
                library_db.pool(),
            ),
        );
    let webhook_verifier_registry =
        Arc::new(inbound_sync::WebhookVerifierRegistry::new());

    // Base URL for webhook endpoints
    let base_url = std::env::var("LIBRARY_API_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:50055".to_string());

    // Usecases
    let register_endpoint =
        Arc::new(inbound_sync::usecase::RegisterWebhookEndpoint::new(
            auth_app_trait.clone(),
            webhook_endpoint_repo.clone(),
            base_url.clone(),
        ));
    let update_endpoint =
        Arc::new(inbound_sync::usecase::UpdateWebhookEndpoint::new(
            auth_app_trait.clone(),
            webhook_endpoint_repo.clone(),
        ));
    let delete_endpoint =
        Arc::new(inbound_sync::usecase::DeleteWebhookEndpoint::new(
            auth_app_trait.clone(),
            webhook_endpoint_repo.clone(),
        ));
    let receive_webhook =
        Arc::new(inbound_sync::usecase::ReceiveWebhook::new(
            webhook_endpoint_repo.clone(),
            webhook_event_repo.clone(),
            webhook_verifier_registry.clone(),
            provider_secrets.clone(),
        ));

    // SyncState repository for tracking sync status
    let sync_state_repo: Arc<dyn inbound_sync::SyncStateRepository> =
        Arc::new(SqlxSyncStateRepository::new(library_db.pool()));

    // Integration Marketplace repositories
    let builtin_integration_registry =
        Arc::new(BuiltinIntegrationRegistry::new());
    let integration_repository: Arc<dyn IntegrationRepository> =
        builtin_integration_registry.clone();
    let sqlx_connection_repository =
        Arc::new(SqlxConnectionRepository::new(library_db.pool()));
    let connection_repository: Arc<dyn ConnectionRepository> =
        sqlx_connection_repository.clone();

    let receive_provider_webhook =
        Arc::new(inbound_sync::usecase::ReceiveProviderWebhook::new(
            webhook_endpoint_repo.clone(),
            webhook_event_repo.clone(),
            webhook_verifier_registry.clone(),
            connection_repository.clone(),
            provider_secrets.clone(),
        ));

    // List usecases for inbound sync
    let list_integrations: Arc<
        dyn inbound_sync::usecase::ListIntegrationsInputPort,
    > = Arc::new(inbound_sync::usecase::ListIntegrations::new(
        auth_app_trait.clone(),
        integration_repository.clone(),
    ));
    let list_connections: Arc<
        dyn inbound_sync::usecase::ListConnectionsInputPort,
    > = Arc::new(inbound_sync::usecase::ListConnections::new(
        auth_app_trait.clone(),
        connection_repository.clone(),
    ));
    let integration_query_state = graphql::IntegrationQueryState {
        list_integrations,
        list_connections,
    };

    // API Key Validator for non-OAuth integrations (e.g., Stripe)
    let api_key_validator: Arc<dyn ApiKeyValidator> =
        Arc::new(HttpApiKeyValidator::new());

    // Default clients and data handlers
    // GitHub and Linear use real implementations; others remain NoOp for now
    let repo_repository: Arc<dyn crate::domain::RepoRepository> =
        Arc::new(
            crate::interface_adapter::gateway::repo_repository::RepoRepositoryImpl::new(
                library_db.clone(),
            ),
        );
    let library_data_repository: Arc<
        dyn inbound_sync::providers::github::LibraryDataRepository,
    > = Arc::new(LibraryDataRepositoryImpl::new(
        database_app.clone(),
        repo_repository.clone(),
        sync_state_repo.clone(),
        database_manager_db.clone(),
    ));
    let github_token_provider =
        Arc::new(AuthAppTokenProvider::new(auth_app_trait.clone()));
    let github_client: Arc<
        dyn inbound_sync::providers::github::GitHubClient,
    > = Arc::new(OAuthGitHubClient::new(github_token_provider));
    let github_data_handler: Arc<
        dyn inbound_sync::providers::github::GitHubDataHandler,
    > = Arc::new(DefaultGitHubDataHandler::new(
        library_data_repository.clone(),
    ));
    let linear_token_provider =
        Arc::new(AuthAppTokenProvider::new(auth_app_trait.clone()));
    let linear_client: Arc<
        dyn inbound_sync::providers::linear::LinearClient,
    > = Arc::new(OAuthLinearClient::new(linear_token_provider));
    let linear_data_handler: Arc<
        dyn inbound_sync::providers::linear::LinearDataHandler,
    > = Arc::new(DefaultLinearDataHandler::new(
        library_data_repository.clone(),
    ));
    let hubspot_client: Arc<
        dyn inbound_sync::providers::hubspot::HubSpotClient,
    > = Arc::new(NoOpHubSpotClient);
    let hubspot_data_handler: Arc<
        dyn inbound_sync::providers::hubspot::HubSpotDataHandler,
    > = Arc::new(NoOpHubSpotDataHandler);
    let stripe_client: Arc<
        dyn inbound_sync::providers::stripe::StripeClient,
    > = Arc::new(NoOpStripeClient);
    let stripe_data_handler: Arc<
        dyn inbound_sync::providers::stripe::StripeDataHandler,
    > = Arc::new(NoOpStripeDataHandler);
    let notion_client: Arc<
        dyn inbound_sync::providers::notion::NotionClient,
    > = Arc::new(NoOpNotionClient);
    let notion_data_handler: Arc<
        dyn inbound_sync::providers::notion::NotionDataHandler,
    > = Arc::new(NoOpNotionDataHandler);
    let square_client: Arc<
        dyn inbound_sync::providers::square::SquareClient,
    > = match std::env::var("SQUARE_API_KEY") {
        Ok(api_key) if !api_key.is_empty() => {
            tracing::info!("Using SquareApiClient with SQUARE_API_KEY");
            Arc::new(inbound_sync::providers::square::SquareApiClient::new(
                api_key,
            ))
        }
        _ => {
            tracing::warn!(
                "SQUARE_API_KEY not set, using NoOp Square client"
            );
            Arc::new(NoOpSquareClient)
        }
    };
    let square_data_handler: Arc<
        dyn inbound_sync::providers::square::SquareDataHandler,
    > = Arc::new(
        inbound_sync::providers::square::DefaultSquareDataHandler::new(
            library_data_repository.clone(),
        ),
    );

    // Event Processor Registry
    let mut processor_registry = EventProcessorRegistry::new();
    processor_registry.register(Arc::new(GitHubEventProcessor::new(
        github_client.clone(),
        sync_state_repo.clone(),
        github_data_handler.clone(),
    )));
    processor_registry.register(Arc::new(LinearEventProcessor::new(
        linear_client.clone(),
        sync_state_repo.clone(),
        linear_data_handler.clone(),
    )));
    processor_registry.register(Arc::new(HubSpotEventProcessor::new(
        hubspot_client.clone(),
        sync_state_repo.clone(),
        hubspot_data_handler.clone(),
    )));
    processor_registry.register(Arc::new(StripeEventProcessor::new(
        stripe_client.clone(),
        sync_state_repo.clone(),
        stripe_data_handler.clone(),
    )));
    processor_registry.register(Arc::new(NotionEventProcessor::new(
        notion_client.clone(),
        notion_data_handler.clone(),
    )));
    processor_registry.register(Arc::new(SquareEventProcessor::new(
        square_client.clone(),
        sync_state_repo.clone(),
        square_data_handler.clone(),
    )));
    let processor_registry = Arc::new(processor_registry);

    // ProcessWebhookEvent usecase
    let process_webhook_event = Arc::new(ProcessWebhookEvent::new(
        webhook_endpoint_repo.clone(),
        webhook_event_repo.clone(),
        processor_registry,
    ));

    // SyncOperationRepository
    let sync_operation_repo: Arc<
        dyn inbound_sync_domain::SyncOperationRepository,
    > = Arc::new(
        inbound_sync::interface_adapter::SqlxSyncOperationRepository::new(
            library_db.pool(),
        ),
    );

    // SendTestWebhook and RetryWebhookEvent usecases
    let send_test_webhook =
        Arc::new(inbound_sync::usecase::SendTestWebhook::new(
            webhook_endpoint_repo.clone(),
            webhook_event_repo.clone(),
        ));
    let retry_webhook_event =
        Arc::new(inbound_sync::usecase::RetryWebhookEvent::new(
            webhook_event_repo.clone(),
        ));

    // API Pull processor registry
    let mut api_pull_registry =
        inbound_sync::usecase::ApiPullProcessorRegistry::new();

    // GitHub
    api_pull_registry.register(Arc::new(
        inbound_sync::providers::github::GitHubApiPullProcessor::new(
            github_client.clone(),
            github_data_handler.clone(),
        ),
    ));

    // Linear
    api_pull_registry.register(Arc::new(
        inbound_sync::providers::linear::LinearApiPullProcessor::new(
            linear_client.clone(),
            linear_data_handler.clone(),
        ),
    ));

    // Notion
    api_pull_registry.register(Arc::new(
        inbound_sync::providers::notion::NotionApiPullProcessor::new(
            notion_client.clone(),
            notion_data_handler.clone(),
        ),
    ));

    // Stripe (stub)
    api_pull_registry.register(Arc::new(
        inbound_sync::providers::stripe::StripeApiPullProcessor::new(),
    ));

    // HubSpot (stub)
    api_pull_registry.register(Arc::new(
        inbound_sync::providers::hubspot::HubSpotApiPullProcessor::new(),
    ));

    // Square
    api_pull_registry.register(Arc::new(
        inbound_sync::providers::square::SquareApiPullProcessor::new(
            square_client.clone(),
            square_data_handler.clone(),
        ),
    ));

    let api_pull_registry = Arc::new(api_pull_registry);

    // InitialSync and OnDemandPull usecases
    let initial_sync = Arc::new(inbound_sync::usecase::InitialSync::new(
        auth_app_trait.clone(),
        webhook_endpoint_repo.clone(),
        sync_operation_repo.clone(),
        sync_state_repo.clone(),
        api_pull_registry.clone(),
    ));
    let on_demand_pull =
        Arc::new(inbound_sync::usecase::OnDemandPull::new(
            auth_app_trait.clone(),
            webhook_endpoint_repo.clone(),
            sync_operation_repo.clone(),
            sync_state_repo.clone(),
            api_pull_registry.clone(),
        ));

    let (shutdown_tx, webhook_handler_state) = build_webhook_runtime(
        process_webhook_event,
        receive_webhook,
        receive_provider_webhook,
    );

    // Clone repositories for GraphQL schema before moving to mutation state
    let integration_registry_for_schema: Arc<
        dyn inbound_sync_domain::IntegrationRepository,
    > = Arc::clone(&integration_repository);
    let connection_repo_for_schema: Arc<
        dyn inbound_sync_domain::ConnectionRepository,
    > = Arc::clone(&connection_repository);
    let oauth_service_for_schema: Arc<
        dyn inbound_sync_domain::OAuthService,
    > = Arc::clone(&oauth_service);
    let oauth_token_repo_for_schema: Arc<
        dyn inbound_sync_domain::OAuthTokenRepository,
    > = Arc::clone(&oauth_token_repo);

    // GraphQL state for library sync
    let inbound_sync_query_state =
        inbound_sync::adapter::LibrarySyncQueryState {
            endpoint_repository: webhook_endpoint_repo.clone(),
            event_repository: webhook_event_repo.clone(),
            operation_repo: sync_operation_repo.clone(),
            integration_repository: integration_repository.clone(),
            connection_repository: connection_repository.clone(),
            base_url: base_url.clone(),
        };
    let inbound_sync_mutation_state =
        inbound_sync::adapter::LibrarySyncMutationState {
            register_endpoint: register_endpoint.clone(),
            update_endpoint: update_endpoint.clone(),
            delete_endpoint: delete_endpoint.clone(),
            send_test_webhook: Some(send_test_webhook),
            retry_webhook_event: Some(retry_webhook_event),
            initial_sync: initial_sync.clone(),
            on_demand_pull: on_demand_pull.clone(),
            operation_repo: sync_operation_repo.clone(),
            integration_repository: integration_repository.clone(),
            connection_repository: connection_repository.clone(),
            oauth_service: Some(oauth_service),
            api_key_validator: Some(api_key_validator),
            base_url: base_url.clone(),
        };

    let library_app: Arc<LibraryApp> = Arc::new(
        LibraryApp::new(
            library_db.clone(),
            database_app.clone(),
            sdk.clone(),
            sync_data.clone(),
            webhook_endpoint_repo.clone(),
            sync_state_repo.clone(),
        )
        .await,
    );

    let parquet_storage = build_parquet_storage().await?;
    let image_store = build_image_store().await?;

    let schema: graphql::AppSchema = Schema::build(
        graphql::Query::default(),
        graphql::Mutation::default(),
        EmptySubscription,
    )
    .data(sdk.clone())
    .data(oauth_bootstrap.clone())
    .data(auth_app_trait.clone())
    .data(library_app.clone())
    // Registered on its own as well as inside `library_app` so that
    // `User::organizations` can tell a Library organization from any
    // other tenant without pulling in the whole application.
    .data(library_app.organization_repo.clone())
    .data(integration_query_state)
    .data(github.clone())
    .data(inbound_sync_query_state)
    .data(inbound_sync_mutation_state)
    .data(integration_registry_for_schema)
    .data(connection_repo_for_schema)
    .data(oauth_service_for_schema)
    .data(oauth_token_repo_for_schema)
    .data(builtin_integration_registry.clone())
    .data(sqlx_connection_repository.clone())
    .finish();

    // let environment =
    //     std::env::var("ENVIRONMENT").unwrap_or("production".into());
    // if environment == "development" {
    //     let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect(
    //         "This program should be run as part of a Cargo build script",
    //     );
    //     let mut file = std::fs::File::create(format!(
    //         "{}/schema.graphql",
    //         manifest_dir
    //     ))?;

    //     use std::io::Write;
    //     file.write_all(schema.clone().sdl().as_bytes())?;
    // }

    // Collaboration WebSocket is Non-GA and must stay unregistered
    // unless a validation environment explicitly opts in.
    let collab_router = if collaboration_ws_enabled() {
        tracing::warn!(
            env = COLLAB_WS_ENABLED_ENV,
            "enabling Non-GA collaboration WebSocket route"
        );
        let collab_persistence =
            Arc::new(SqlxDocumentPersistence::new(library_db.pool()));
        let doc_manager =
            Arc::new(DocumentManager::new(collab_persistence));
        doc_manager.start_background_persistence();
        let collab_state = CollaborationState {
            manager: doc_manager,
        };

        axum::Router::new()
            .route(
                "/ws/collab/:document_key",
                get(crate::collaboration::ws_handler),
            )
            .with_state(collab_state)
    } else {
        tracing::info!(
            env = COLLAB_WS_ENABLED_ENV,
            "collaboration WebSocket route disabled"
        );
        axum::Router::new()
    };

    // Webhook router
    let webhook_router =
        inbound_sync::adapter::create_webhook_router(webhook_handler_state);

    // Docs routes use optional authentication. Public repos are
    // readable anonymously; private repos require normal repo/data
    // permissions.
    let docs_router = axum::Router::new()
        .route("/docs/:org/:repo", get(handler::docs::list_docs))
        .route("/docs/:org/:repo/:data_id", get(handler::docs::view_doc))
        .route(
            "/docs/:org/:repo/:data_id/md",
            get(handler::docs::view_doc_markdown),
        );

    // Reading an image is unauthenticated on purpose: an `<img>` carries no
    // bearer token, so the unguessable id in the URL is the credential.
    let image_router = axum::Router::new()
        .route(
            "/v1beta/repos/:org/:repo/images",
            post(handler::image::upload_image),
        )
        .route(
            "/v1beta/repos/:org/:repo/images/:image_id",
            get(handler::image::view_image),
        )
        .layer(DefaultBodyLimit::max(MAX_IMAGE_BYTES));

    let app = axum::Router::new()
        .route("/", axum::routing::get(health_check))
        .route("/version", get(version))
        .route(
            "/.well-known/oauth-protected-resource",
            get(handler::mcp::protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            get(handler::mcp::protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(handler::mcp::mcp_oauth_authorization_server_metadata),
        )
        .route(
            "/.well-known/oauth-authorization-server/mcp/oauth",
            get(handler::mcp::mcp_oauth_authorization_server_metadata),
        )
        .route("/mcp", post(handler::mcp::mcp_handler))
        .route(
            "/mcp/oauth/register",
            post(handler::mcp::mcp_oauth_register),
        )
        .route(
            "/mcp/oauth/authorize",
            get(handler::mcp::mcp_oauth_authorize)
                .post(handler::mcp::mcp_oauth_authorize_submit),
        )
        .route("/mcp/oauth/token", post(handler::mcp::mcp_oauth_token))
        .route("/v1/graphql", get(graphql::graphiql))
        .route("/v1/graphql", post(graphql::graphql_handler))
        .route(
            "/v1/graphql/introspection",
            get(graphql::graphql_introspection),
        )
        .merge(handler::create_router())
        .merge(docs_router)
        .merge(image_router)
        .merge(collab_router)
        .merge(webhook_router)
        // Layer order matters: outermost (first in chain) to innermost
        // Layers are applied in reverse order of declaration:
        // 1. SetRequestIdLayer - generates UUID and sets x-request-id header on request
        // 2. TraceLayer - creates tracing span with request_id for all log output
        // 3. PropagateRequestIdLayer - copies x-request-id to response headers
        // 4. CorsLayer - handles CORS preflight and headers
        .layer(
            CorsLayer::new()
                .allow_methods(vec![
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                ])
                .allow_headers(Any)
                .allow_origin(Any),
        )
        .layer(create_propagate_request_id_layer())
        .layer(create_trace_layer())
        .layer(middleware::from_fn(
            crate::sentry_context::sentry_request_context_middleware,
        ))
        // Must wrap the handlers (and their extractors) so organization
        // resolution can authenticate against tachyon-api as the caller.
        .layer(middleware::from_fn(
            crate::sdk_auth::caller_token_middleware,
        ))
        .layer(create_request_id_layer())
        .layer(Extension(sdk))
        .layer(Extension(library_app))
        .layer(Extension(database_app))
        .layer(Extension(parquet_storage))
        .layer(Extension(image_store))
        // Keep webhook worker alive by retaining the shutdown sender when enabled.
        .layer(Extension(shutdown_tx))
        .layer(Extension(schema));
    Ok(app)
}

async fn health_check() -> &'static str {
    tracing::debug!("health check");
    "OK"
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn build_parquet_storage(
) -> Result<ParquetStorage, Box<dyn std::error::Error>> {
    let bucket = std::env::var("LIBRARY_PARQUET_BUCKET")
        .unwrap_or_else(|_| "library-parquet".to_string());
    let (storage, presign_storage) = build_bucket_storage(&bucket).await?;
    Ok(ParquetStorage::new(storage, presign_storage, bucket))
}

/// Where record body images go. Production speaks to Tachyon storage as
/// the Library service account; everywhere else falls back to the MinIO
/// the API already runs against, so development needs no Tachyon account.
/// `LIBRARY_IMAGE_STORAGE=tachyon|bucket` overrides the default choice.
async fn build_image_store(
) -> Result<Arc<dyn ImageObjectStore>, Box<dyn std::error::Error>> {
    let environment = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "dev".to_string())
        .to_lowercase();
    let is_production =
        environment == "prod" || environment == "production";
    let use_tachyon = match std::env::var("LIBRARY_IMAGE_STORAGE")
        .unwrap_or_default()
        .as_str()
    {
        "tachyon" => true,
        "bucket" => false,
        _ => is_production,
    };

    if use_tachyon {
        let base_url = std::env::var("TACHYON_API_URL")
            .unwrap_or_else(|_| "https://api.n1.tachy.one".to_string());
        let auth_token = std::env::var("SERVICE_AUTH_TOKEN").map_err(
            |_| "SERVICE_AUTH_TOKEN must be set to store images in Tachyon storage",
        )?;
        return Ok(Arc::new(TachyonImageStore::new(
            base_url,
            auth_token,
            crate::domain::LIBRARY_TENANT.to_string(),
        )));
    }

    let bucket = std::env::var("LIBRARY_IMAGE_BUCKET")
        .unwrap_or_else(|_| "library-images".to_string());
    let (storage, presign_storage) = build_bucket_storage(&bucket).await?;
    Ok(Arc::new(BucketImageStore::new(
        storage,
        presign_storage,
        bucket,
    )))
}

/// Object storage for one bucket: S3 in production, MinIO everywhere else.
/// The second driver signs URLs against the endpoint clients can reach,
/// which in local development is not the endpoint the API talks to.
async fn build_bucket_storage(
    bucket: &str,
) -> Result<(Arc<dyn Storage>, Arc<dyn Storage>), Box<dyn std::error::Error>>
{
    let environment =
        std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string());
    let environment_lower = environment.to_lowercase();
    let is_production =
        environment_lower == "prod" || environment_lower == "production";
    let skip_minio_setup = environment_lower == "test"
        || std::env::var("SKIP_MINIO_SETUP")
            .map(|value| matches!(value.as_str(), "true" | "1" | "TRUE"))
            .unwrap_or(false);

    if is_production {
        let s3 = S3Driver::new()? as Arc<dyn Storage>;
        return Ok((s3.clone(), s3));
    }

    let access_key = std::env::var("MINIO_ROOT_USER")
        .unwrap_or_else(|_| "admin".to_string());
    let secret_key = std::env::var("MINIO_ROOT_PASSWORD")
        .unwrap_or_else(|_| "password".to_string());
    let storage_url = std::env::var("MINIO_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:9000".to_string());
    let public_storage_url = std::env::var("MINIO_PUBLIC_ENDPOINT")
        .unwrap_or_else(|_| storage_url.clone());
    let minio = MinioDriver::new(&MinioConfiguration {
        storage_url,
        access_key: access_key.clone(),
        secret_key: secret_key.clone(),
    })?;
    if !skip_minio_setup {
        minio.create_bucket(bucket).await?;
    }
    let public_minio = MinioDriver::new(&MinioConfiguration {
        storage_url: public_storage_url,
        access_key,
        secret_key,
    })?;
    Ok((minio as Arc<dyn Storage>, public_minio as Arc<dyn Storage>))
}

fn build_webhook_runtime(
    process_webhook_event: Arc<inbound_sync::usecase::ProcessWebhookEvent>,
    receive_webhook: Arc<inbound_sync::usecase::ReceiveWebhook>,
    receive_provider_webhook: Arc<
        inbound_sync::usecase::ReceiveProviderWebhook,
    >,
) -> (
    Option<tokio::sync::broadcast::Sender<()>>,
    inbound_sync::adapter::WebhookHandlerState,
) {
    let shutdown_tx = if std::env::var(WEBHOOK_WORKER_ENABLED_ENV)
        .map(|value| env_flag_enabled(&value))
        .unwrap_or(false)
    {
        tracing::warn!(
            env = WEBHOOK_WORKER_ENABLED_ENV,
            "enabling webhook event worker"
        );
        let worker = WebhookEventWorker::new(process_webhook_event)
            .with_batch_size(10)
            .with_poll_interval(std::time::Duration::from_secs(5));
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
        tokio::spawn(async move {
            worker.run(shutdown_rx).await;
        });
        Some(shutdown_tx)
    } else {
        tracing::info!(
            env = WEBHOOK_WORKER_ENABLED_ENV,
            "webhook event worker disabled"
        );
        drop(process_webhook_event);
        None
    };

    let webhook_handler_state =
        inbound_sync::adapter::WebhookHandlerState {
            receive_webhook,
            receive_provider_webhook,
            base_url: std::env::var("LIBRARY_API_BASE_URL").ok(),
        };
    (shutdown_tx, webhook_handler_state)
}

#[cfg(test)]
mod tests {
    use super::env_flag_enabled;

    #[test]
    fn collaboration_ws_env_flag_requires_true() {
        assert!(env_flag_enabled("true"));
        assert!(env_flag_enabled("TRUE"));
        assert!(env_flag_enabled(" true "));

        assert!(!env_flag_enabled(""));
        assert!(!env_flag_enabled("false"));
        assert!(!env_flag_enabled("1"));
        assert!(!env_flag_enabled("yes"));
    }

    #[test]
    fn webhook_worker_env_flag_uses_same_boolean_parser() {
        assert!(env_flag_enabled("TrUe"));

        assert!(!env_flag_enabled("false"));
        assert!(!env_flag_enabled("enabled"));
    }
}
