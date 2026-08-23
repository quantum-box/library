use clap::Parser;
use lambda_http::{run, Error};
use library_api::LIBRARY_TENANT;
use std::env::set_var;
use std::sync::Arc;

use library_api::sdk_auth::{SdkAuthApp, SdkOAuthTokenRepository};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = library_api::Config::parse();
    config.validate_for_server_startup()?;
    let property_value_mode = config.property_value_storage_mode.parse()?;
    let property_definition_mode =
        config.property_definition_storage_mode.parse()?;

    telemetry::init_production_tracing(telemetry::TracingConfig {
        environment: config.environment.as_str(),
        crate_name: "library-api",
        filter: Some(telemetry::DEFAULT_FILTER.to_vec()),
        otel_endpoint: config.otel_exporter_otlp_endpoint,
        insi: Some(false),
        // OTel/X-Ray: enabled via OTEL_ENABLED env var, uses ADOT Lambda layer
        otel_enabled: None, // reads from OTEL_ENABLED
        otel_sampling_rate: None, // reads from OTEL_TRACES_SAMPLER_ARG (default 10%)
    });
    tracing::info!(
        mode = ?property_value_mode,
        "configured PropertyValue storage rollout mode"
    );
    tracing::info!(
        mode = ?property_definition_mode,
        "configured PropertyDefinition storage rollout mode"
    );

    // Hold the Sentry guard for the Lambda runtime lifetime so events
    // forwarded by sentry_tracing are flushed before process exit.
    let _sentry_guard = config
        .sentry_dsn
        .as_deref()
        .map(|dsn| telemetry::init_sentry(dsn, sentry::release_name!()));

    let database_url =
        config.database_url.parse::<value_object::DatabaseUrl>()?;
    let database_layout =
        library_api::DatabaseLayout::from_runtime(database_url)?;
    // One pool per physical database, opened here and shared by
    // everything below. The pools connect on first query, so a cold
    // start no longer waits on the database before it can serve.
    let pools = database_layout.open_pools();

    let database_app = Arc::new(database_manager::factory_client_with_db(
        pools.database_manager.clone(),
        property_value_mode,
        property_definition_mode,
    )?);

    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

    // Create SdkAuthApp for REST-based auth operations
    let sdk = Arc::new(SdkAuthApp::new(
        &config.tachyon_api_url,
        &LIBRARY_TENANT,
        &config.service_auth_token,
    ));

    // OAuth credentials come from Tachyon's IaC configuration, which
    // means a round trip to tachyon-api. Resolving them on first use
    // keeps that round trip off the startup path.
    use inbound_sync::interface_adapter::gateway::HttpOAuthService;
    use inbound_sync_domain::{
        OAuthCredentialsSource, OAuthTokenRepository,
    };
    use library_api::oauth_bootstrap::OAuthBootstrap;

    let oauth_token_repo: Arc<dyn OAuthTokenRepository> =
        Arc::new(SdkOAuthTokenRepository::new(sdk.clone()));

    let oauth_bootstrap =
        Arc::new(OAuthBootstrap::new(sdk.clone(), LIBRARY_TENANT.clone()));
    // Warm the configuration in the background so the first OAuth
    // flow does not pay for it. Nothing waits on this, and nothing
    // depends on it finishing: a request that arrives first resolves
    // the configuration itself. On Lambda the task may also be frozen
    // mid-flight between invocations, which is equally harmless.
    let warm_oauth = oauth_bootstrap.clone();
    tokio::spawn(async move {
        warm_oauth.get().await;
    });

    let oauth_service = Arc::new(
        HttpOAuthService::new(oauth_token_repo.clone())
            .with_credential_source(
                oauth_bootstrap.clone() as Arc<dyn OAuthCredentialsSource>
            ),
    );

    let github = Arc::new(github_provider::GitHub::new(Some(
        oauth_bootstrap.clone()
            as Arc<dyn github_provider::OAuthConfigSource>,
    )));
    let provider_secrets =
        Arc::new(inbound_sync::WebhookSecretStore::new());

    let app = library_api::router(
        pools,
        sdk,
        database_app,
        github,
        oauth_service,
        oauth_token_repo,
        provider_secrets,
        oauth_bootstrap,
    )
    .await
    .expect("library api router error");

    tracing::info!("start lambda...");
    run(app).await
}
