use crate::app::LibraryApp;
use crate::collaboration::handler::CollaborationState;
use crate::collaboration::manager::DocumentManager;
use crate::collaboration::persistence::SqlxDocumentPersistence;
use crate::handler;
use crate::handler::graphql;
use crate::interface_adapter::gateway::LibraryDataRepositoryImpl;
use crate::sdk_auth::SdkAuthApp;
use async_graphql::EmptySubscription;
use async_graphql::Schema;
use axum::http::method::Method;
use axum::routing::get;
use axum::routing::post;
use axum::Extension;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;
use telemetry::http::{
    create_propagate_request_id_layer, create_request_id_layer,
    create_trace_layer,
};
use tower_http::cors::{Any, CorsLayer};
use value_object::DatabaseUrl;

use persistence::{MinioConfiguration, MinioDriver, S3Driver, Storage};

use crate::handler::data::ParquetStorage;

use inbound_sync::interface_adapter::{
    BuiltinIntegrationRegistry, HttpApiKeyValidator, NoOpGitHubClient,
    NoOpGitHubDataHandler, NoOpHubSpotClient, NoOpHubSpotDataHandler,
    NoOpNotionClient, NoOpNotionDataHandler, NoOpSquareClient,
    NoOpStripeClient, NoOpStripeDataHandler, SqlxConnectionRepository,
    SqlxSyncStateRepository,
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

#[derive(Serialize)]
struct VersionResponse {
    version: &'static str,
}

pub async fn router(
    dsn: impl ToString,
    sdk: Arc<SdkAuthApp>,
    database_app: Arc<database_manager::App>,
    github: Arc<github_provider::GitHub>,
    oauth_service: Arc<dyn inbound_sync_domain::OAuthService>,
    oauth_token_repo: Arc<dyn inbound_sync_domain::OAuthTokenRepository>,
    provider_secrets: Arc<WebhookSecretStore>,
) -> Result<axum::Router, Box<dyn std::error::Error>> {
    let dsn = dsn.to_string().parse::<DatabaseUrl>()?;

    // Database connection for library
    let library_db =
        persistence::Db::new(&dsn.use_database("library")).await;
    let database_manager_db = persistence::Db::new(
        &dsn.use_database("tachyon_apps_database_manager"),
    )
    .await;

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

    // API Key Validator for non-OAuth integrations (e.g., Stripe)
    let api_key_validator: Arc<dyn ApiKeyValidator> =
        Arc::new(HttpApiKeyValidator::new());

    // Default clients and data handlers
    // Linear uses real implementations; others remain NoOp for now
    let github_client: Arc<
        dyn inbound_sync::providers::github::GitHubClient,
    > = Arc::new(NoOpGitHubClient);
    let github_data_handler: Arc<
        dyn inbound_sync::providers::github::GitHubDataHandler,
    > = Arc::new(NoOpGitHubDataHandler);
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

    // Start background worker for processing webhook events
    let worker = WebhookEventWorker::new(process_webhook_event.clone())
        .with_batch_size(10)
        .with_poll_interval(std::time::Duration::from_secs(5));

    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    tokio::spawn(async move {
        worker.run(shutdown_rx).await;
    });

    // Webhook handler state
    let webhook_handler_state =
        inbound_sync::adapter::WebhookHandlerState {
            receive_webhook: receive_webhook.clone(),
            receive_provider_webhook: receive_provider_webhook.clone(),
            base_url: std::env::var("LIBRARY_API_BASE_URL").ok(),
        };

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

    // Enhance database_app with sync usecases
    let database_app_with_sync = Arc::new(
        (*database_app)
            .clone()
            .with_inbound_sync(
                list_integrations.clone(),
                list_connections.clone(),
                register_endpoint.clone()
                    as Arc<
                        dyn inbound_sync::usecase::RegisterWebhookEndpointInputPort,
                    >,
                update_endpoint.clone()
                    as Arc<
                        dyn inbound_sync::usecase::UpdateWebhookEndpointInputPort,
                    >,
                delete_endpoint.clone()
                    as Arc<
                        dyn inbound_sync::usecase::DeleteWebhookEndpointInputPort,
                    >,
            )
            .with_outbound_sync(sync_data.clone()),
    );

    // Library application with enhanced database_app
    let library_app: Arc<LibraryApp> = Arc::new(
        LibraryApp::new(
            &dsn,
            database_app_with_sync.clone(),
            sdk.clone(),
            sync_data.clone(),
        )
        .await,
    );

    let parquet_bucket = std::env::var("LIBRARY_PARQUET_BUCKET")
        .unwrap_or_else(|_| "library-parquet".to_string());
    let environment =
        std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string());
    let environment_lower = environment.to_lowercase();
    let is_production =
        environment_lower == "prod" || environment_lower == "production";
    let skip_minio_setup = environment_lower == "test"
        || std::env::var("SKIP_MINIO_SETUP")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE"))
            .unwrap_or(false);
    let (storage, presign_storage): (Arc<dyn Storage>, Arc<dyn Storage>) =
        if is_production {
            let s3 = S3Driver::new()? as Arc<dyn Storage>;
            (s3.clone(), s3)
        } else {
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
                minio.create_bucket(&parquet_bucket).await?;
            }
            let public_minio = MinioDriver::new(&MinioConfiguration {
                storage_url: public_storage_url,
                access_key,
                secret_key,
            })?;
            (minio as Arc<dyn Storage>, public_minio as Arc<dyn Storage>)
        };
    let parquet_storage =
        ParquetStorage::new(storage, presign_storage, parquet_bucket);

    let schema: graphql::AppSchema = Schema::build(
        graphql::Query::default(),
        graphql::Mutation::default(),
        EmptySubscription,
    )
    .data(sdk.clone())
    .data(auth_app_trait.clone())
    .data(library_app.clone())
    .data(database_app_with_sync.clone())
    .data(github.clone())
    .data(sync_data.clone())
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

    // Collaboration (real-time editing) setup
    let collab_persistence =
        Arc::new(SqlxDocumentPersistence::new(library_db.pool()));
    let doc_manager = Arc::new(DocumentManager::new(collab_persistence));
    doc_manager.start_background_persistence();
    let collab_state = CollaborationState {
        manager: doc_manager,
    };

    let collab_router = axum::Router::new()
        .route(
            "/ws/collab/:document_key",
            get(crate::collaboration::ws_handler),
        )
        .with_state(collab_state);

    // Webhook router
    let webhook_router =
        inbound_sync::adapter::create_webhook_router(webhook_handler_state);

    // Public docs routes (no authentication required)
    let docs_router = axum::Router::new()
        .route("/docs/:org/:repo", get(handler::docs::list_docs))
        .route(
            "/docs/:org/:repo/:data_id",
            get(handler::docs::view_doc),
        )
        .route(
            "/docs/:org/:repo/:data_id/md",
            get(handler::docs::view_doc_markdown),
        );

    let app = axum::Router::new()
        .route("/", axum::routing::get(health_check))
        .route("/version", get(version))
        .route("/v1/graphql", get(graphql::graphiql))
        .route("/v1/graphql", post(graphql::graphql_handler))
        .route(
            "/v1/graphql/introspection",
            get(graphql::graphql_introspection),
        )
        .merge(handler::create_router())
        .merge(docs_router)
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
        .layer(create_request_id_layer())
        .layer(Extension(sdk))
        .layer(Extension(library_app))
        .layer(Extension(database_app))
        .layer(Extension(parquet_storage))
        // Keep webhook worker alive by retaining the shutdown sender.
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
