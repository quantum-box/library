use clap::Parser;
use github_provider::OAuthConfig;
use lambda_http::{run, Error};
use library_api::LIBRARY_TENANT;
use std::env::set_var;
use std::sync::Arc;

use library_api::sdk_auth::{SdkAuthApp, SdkOAuthTokenRepository};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = library_api::Config::parse();

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

    if let Some(dsn) = config.sentry_dsn {
        telemetry::init_sentry(&dsn);
    }

    let database_url =
        config.database_url.parse::<value_object::DatabaseUrl>()?;
    tracing::debug!("start connect database...");

    let database_app = Arc::new(
        database_manager::factory_client(
            &database_url.use_database("tachyon_apps_database_manager"),
        )
        .await?,
    );

    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

    // Create SdkAuthApp for REST-based auth operations
    let sdk = Arc::new(SdkAuthApp::new(
        &config.tachyon_api_url,
        &LIBRARY_TENANT,
        &config.service_auth_token,
    ));

    // Fetch OAuth config from tachyon-api (replaces
    // auth::App + iac::App initialization)
    use inbound_sync::interface_adapter::gateway::HttpOAuthService;
    use inbound_sync_domain::{
        OAuthClientCredentials, OAuthProvider, OAuthService,
        OAuthTokenRepository,
    };

    let oauth_token_repo: Arc<dyn OAuthTokenRepository> =
        Arc::new(SdkOAuthTokenRepository::new(sdk.clone()));

    let mut oauth_service = HttpOAuthService::new(oauth_token_repo.clone());

    // Fetch OAuth config via REST endpoint
    match sdk.fetch_oauth_config(&LIBRARY_TENANT).await {
        Ok(bootstrap) => {
            if let Some(creds) = &bootstrap.github_credentials {
                // Allow env var override for redirect URI in Lambda
                let redirect_uri = std::env::var("GITHUB_REDIRECT_URI")
                    .unwrap_or_else(|_| creds.redirect_uri.clone());
                oauth_service = oauth_service.with_credentials(
                    OAuthProvider::Github,
                    OAuthClientCredentials {
                        client_id: creds.client_id.clone(),
                        client_secret: creds.client_secret.clone(),
                        redirect_uri,
                    },
                );
            }
            if let Some(creds) = &bootstrap.linear_credentials {
                oauth_service = oauth_service.with_credentials(
                    OAuthProvider::Linear,
                    OAuthClientCredentials {
                        client_id: creds.client_id.clone(),
                        client_secret: creds.client_secret.clone(),
                        redirect_uri: creds.redirect_uri.clone(),
                    },
                );
            }
        }
        Err(e) => {
            tracing::warn!("Failed to fetch OAuth config: {:?}", e);
        }
    }

    let oauth_service = Arc::new(oauth_service);

    // Legacy GitHub provider for backward compatibility
    let github_oauth_config = oauth_service
        .get_credentials(OAuthProvider::Github)
        .map(|credentials| {
            let redirect_uri = std::env::var("GITHUB_REDIRECT_URI")
                .unwrap_or_else(|_| credentials.redirect_uri.clone());
            OAuthConfig {
                client_id: credentials.client_id.clone(),
                client_secret: credentials.client_secret.clone(),
                redirect_uri,
            }
        });

    let github =
        Arc::new(github_provider::GitHub::new(github_oauth_config));
    let provider_secrets =
        Arc::new(inbound_sync::WebhookSecretStore::new());

    let app = library_api::router(
        database_url.use_database("library"),
        sdk,
        database_app,
        github,
        oauth_service,
        oauth_token_repo,
        provider_secrets,
    )
    .await
    .expect("library api router error");

    tracing::info!("start lambda...");
    run(app).await
}
