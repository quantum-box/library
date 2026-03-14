use dotenvy::dotenv;
use std::net::SocketAddr;
use std::sync::Arc;

pub mod app;
pub mod collaboration;
pub mod config;
mod domain;
pub mod error;
pub mod handler;
mod interface_adapter;
mod router;
pub mod sdk_auth;
mod usecase;

pub use crate::domain::LIBRARY_TENANT;
use clap::Parser;
use github_provider::OAuthConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let config = config::Config::parse();
    if config.environment == "development" {
        println!("{config:#?}");
    }

    // Extend DEFAULT_FILTER with library_api tracing targets
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
        otel_endpoint: config.otel_exporter_otlp_endpoint,
        ..Default::default()
    });

    if let Some(dsn) = config.sentry_dsn {
        telemetry::init_sentry(&dsn);
    }

    tracing::debug!("start connect database...");

    let dsn = config.database_url.parse::<value_object::DatabaseUrl>()?;
    let database_app = Arc::new(
        database_manager::factory_client(
            &dsn.use_database("tachyon_apps_database_manager"),
        )
        .await?,
    );

    // Create SdkAuthApp for REST-based auth operations
    let sdk = Arc::new(sdk_auth::SdkAuthApp::new(
        &config.tachyon_api_url,
        &LIBRARY_TENANT,
        &config.service_auth_token,
    ));
    tracing::info!(
        "SdkAuthApp configured with base_url={}",
        config.tachyon_api_url
    );

    // Fetch OAuth config from tachyon-api (replaces
    // tachyon_sdk::auth::AuthApp + iac::App initialization)
    use inbound_sync::interface_adapter::gateway::HttpOAuthService;
    use inbound_sync_domain::{
        OAuthClientCredentials, OAuthProvider, OAuthTokenRepository,
    };

    let oauth_token_repo: Arc<dyn OAuthTokenRepository> =
        Arc::new(sdk_auth::SdkOAuthTokenRepository::new(sdk.clone()));

    let mut oauth_service = HttpOAuthService::new(oauth_token_repo.clone());
    let mut provider_secrets = inbound_sync::WebhookSecretStore::new();
    let mut github_oauth_config: Option<OAuthConfig> = None;

    // Fetch OAuth config via REST endpoint (non-fatal on failure)
    match sdk.fetch_oauth_config(&LIBRARY_TENANT).await {
        Ok(bootstrap) => {
            if let Some(creds) = &bootstrap.github_credentials {
                // Allow env var override for redirect URI
                let redirect_uri = std::env::var("GITHUB_REDIRECT_URI")
                    .unwrap_or_else(|_| creds.redirect_uri.clone());
                oauth_service = oauth_service.with_credentials(
                    OAuthProvider::Github,
                    OAuthClientCredentials {
                        client_id: creds.client_id.clone(),
                        client_secret: creds.client_secret.clone(),
                        redirect_uri: redirect_uri.clone(),
                    },
                );
                tracing::info!(
                    %redirect_uri,
                    "GitHub OAuth credentials configured via REST \
                     (redirect_uri must be registered in the GitHub \
                     App callback URLs)"
                );
                github_oauth_config = Some(OAuthConfig {
                    client_id: creds.client_id.clone(),
                    client_secret: creds.client_secret.clone(),
                    redirect_uri,
                });
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
                if let Some(secret) = &bootstrap.linear_webhook_secret {
                    if !secret.trim().is_empty() {
                        provider_secrets.insert(
                            inbound_sync_domain::Provider::Linear,
                            secret.clone(),
                        );
                    }
                }
                tracing::info!(
                    "Linear OAuth credentials configured via REST"
                );
            }
        }
        Err(e) => {
            tracing::warn!("Failed to fetch OAuth config: {:?}", e);
        }
    }

    let oauth_service = Arc::new(oauth_service);
    let github =
        Arc::new(github_provider::GitHub::new(github_oauth_config));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let app = router::router(
        config.database_url,
        sdk,
        database_app,
        github,
        oauth_service,
        oauth_token_repo,
        Arc::new(provider_secrets),
    )
    .await?;
    axum::serve(listener, app).await?;
    Ok(())
}
