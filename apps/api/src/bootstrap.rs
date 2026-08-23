use crate::oauth_bootstrap::OAuthBootstrap;
use crate::{config, domain::LIBRARY_TENANT, router, sdk_auth};
use inbound_sync::interface_adapter::gateway::HttpOAuthService;
use inbound_sync_domain::OAuthTokenRepository;
use std::net::SocketAddr;
use std::sync::Arc;

pub async fn run_api(
    config: config::Config,
) -> Result<(), Box<dyn std::error::Error>> {
    init_tracing(&config);

    // Hold the Sentry guard for the process lifetime so events
    // forwarded by sentry_tracing are flushed before exit.
    let _sentry_guard = config
        .sentry_dsn
        .as_deref()
        .map(|dsn| telemetry::init_sentry(dsn, sentry::release_name!()));

    tracing::debug!("start connect database...");
    let dsn = config.database_url.parse::<value_object::DatabaseUrl>()?;
    let database_layout =
        crate::database_layout::DatabaseLayout::from_runtime(dsn)?;
    let property_value_mode = config.property_value_storage_mode.parse()?;
    let property_definition_mode =
        config.property_definition_storage_mode.parse()?;
    tracing::info!(
        mode = ?property_value_mode,
        "configured PropertyValue storage rollout mode"
    );
    tracing::info!(
        mode = ?property_definition_mode,
        "configured PropertyDefinition storage rollout mode"
    );
    // One pool per physical database, shared by everything below.
    let pools = database_layout.open_pools();
    let database_app = Arc::new(database_manager::factory_client_with_db(
        pools.database_manager.clone(),
        property_value_mode,
        property_definition_mode,
    )?);

    let sdk = Arc::new(sdk_auth::SdkAuthApp::new(
        &config.tachyon_api_url,
        &LIBRARY_TENANT,
        &config.service_auth_token,
    ));
    tracing::info!(
        "SdkAuthApp configured with base_url={}",
        config.tachyon_api_url
    );

    let (
        oauth_service,
        github,
        oauth_token_repo,
        provider_secrets,
        oauth_bootstrap,
    ) = build_oauth_runtime(&sdk).await;

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let app = router::router(
        pools,
        sdk,
        database_app,
        github,
        oauth_service,
        oauth_token_repo,
        provider_secrets,
        oauth_bootstrap,
    )
    .await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

fn init_tracing(config: &config::Config) {
    let mut filter: Vec<&str> = telemetry::DEFAULT_FILTER.to_vec();
    filter.extend([
        "library_api=debug",
        "auth=debug",
        "database_manager=debug",
    ]);
    telemetry::init_tracing(telemetry::TracingConfig {
        environment: config.environment.as_str(),
        crate_name: "library-api",
        filter: Some(filter),
        otel_endpoint: config.otel_exporter_otlp_endpoint.clone(),
        ..Default::default()
    });
}

async fn build_oauth_runtime(
    sdk: &Arc<sdk_auth::SdkAuthApp>,
) -> (
    Arc<dyn inbound_sync_domain::OAuthService>,
    Arc<github_provider::GitHub>,
    Arc<dyn OAuthTokenRepository>,
    Arc<inbound_sync::WebhookSecretStore>,
    Arc<OAuthBootstrap>,
) {
    let oauth_token_repo: Arc<dyn OAuthTokenRepository> =
        Arc::new(sdk_auth::SdkOAuthTokenRepository::new(sdk.clone()));

    let oauth_bootstrap =
        Arc::new(OAuthBootstrap::new(sdk.clone(), LIBRARY_TENANT.clone()));

    // The client credentials resolve on first use, but the Linear
    // webhook secret has to be in the store before a webhook can be
    // verified against it, so this server resolves the configuration
    // now. It shares the cell with everything else, so nothing fetches
    // it a second time.
    let mut provider_secrets = inbound_sync::WebhookSecretStore::new();
    if let Some(bootstrap) = oauth_bootstrap.get().await {
        if bootstrap.github_credentials.is_some() {
            tracing::info!(
                "GitHub OAuth credentials configured via REST \
                 (redirect_uri must be registered in the GitHub \
                 App callback URLs)"
            );
        }
        if bootstrap.linear_credentials.is_some() {
            tracing::info!("Linear OAuth credentials configured via REST");
        }
        if let Some(secret) = &bootstrap.linear_webhook_secret {
            if !secret.trim().is_empty() {
                provider_secrets.insert(
                    inbound_sync_domain::Provider::Linear,
                    secret.clone(),
                );
            }
        }
    }

    let oauth_service = HttpOAuthService::new(oauth_token_repo.clone())
        .with_credential_source(oauth_bootstrap.clone()
            as Arc<dyn inbound_sync_domain::OAuthCredentialsSource>);

    (
        Arc::new(oauth_service),
        Arc::new(github_provider::GitHub::new(Some(
            oauth_bootstrap.clone()
                as Arc<dyn github_provider::OAuthConfigSource>,
        ))),
        oauth_token_repo,
        Arc::new(provider_secrets),
        oauth_bootstrap,
    )
}
