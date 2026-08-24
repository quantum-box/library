use clap::Parser;
use lambda_http::{
    lambda_runtime, request::LambdaRequest, Adapter, Error, LambdaEvent,
    Service,
};
use library_api::LIBRARY_TENANT;
use std::env::set_var;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use library_api::sdk_auth::{SdkAuthApp, SdkOAuthTokenRepository};

/// Marks the synthetic event Tachyon sends to the candidate alias before it
/// promotes stable traffic. All three must match, so nothing that can reach
/// the public Function URL is able to start a migration.
const MIGRATION_GATE_ROUTE_KEY: &str = "LIBRARY_MIGRATION_GATE";
const MIGRATION_GATE_PATH: &str = "/";
const MIGRATION_GATE_DOMAIN: &str = "library-api-migration-gate.internal";

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

    // Kept as the raw string the operator configured: the migration gate
    // reconnects with it, and `DatabaseUrl`'s Display masks the password.
    let raw_database_url: Arc<str> = Arc::from(config.database_url.clone());
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
    // Tachyon invokes the candidate alias with a synthetic API Gateway event
    // before promoting it to stable traffic. Migrations run only for that
    // event; an ordinary cold start just builds serving dependencies, so a
    // poisoned history stops the deployment instead of the live API.
    lambda_runtime::run(DeployMigrationGate::new(
        Adapter::from(app),
        raw_database_url,
    ))
    .await
}

struct DeployMigrationGate<S> {
    inner: S,
    database_url: Arc<str>,
}

impl<S> DeployMigrationGate<S> {
    fn new(inner: S, database_url: Arc<str>) -> Self {
        Self {
            inner,
            database_url,
        }
    }
}

impl<S> Service<LambdaEvent<LambdaRequest>> for DeployMigrationGate<S>
where
    S: Service<LambdaEvent<LambdaRequest>>,
    S::Future: Send + 'static,
    S::Response: 'static,
    S::Error: Into<Error>,
{
    type Response = S::Response;
    type Error = Error;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<Self::Response, Self::Error>> + Send,
        >,
    >;

    fn poll_ready(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(context).map_err(Into::into)
    }

    fn call(&mut self, event: LambdaEvent<LambdaRequest>) -> Self::Future {
        let is_migration_gate = is_migration_gate_event(&event.payload);
        let database_url = Arc::clone(&self.database_url);
        let response = self.inner.call(event);

        Box::pin(async move {
            if is_migration_gate {
                library_api::migrations::run_migration_gate(&database_url)
                    .await
                    .map_err(|error| Error::from(error.to_string()))?;
            }
            response.await.map_err(Into::into)
        })
    }
}

fn is_migration_gate_event(event: &LambdaRequest) -> bool {
    matches!(
        event,
        LambdaRequest::ApiGatewayV2(request)
            if request.route_key.as_deref() == Some(MIGRATION_GATE_ROUTE_KEY)
                && request.raw_path.as_deref() == Some(MIGRATION_GATE_PATH)
                && request.request_context.domain_name.as_deref()
                    == Some(MIGRATION_GATE_DOMAIN)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(payload: serde_json::Value) -> LambdaRequest {
        serde_json::from_value(payload)
            .expect("deserialize Lambda HTTP event")
    }

    fn gate_payload(route_key: &str, domain: &str) -> serde_json::Value {
        serde_json::json!({
            "version": "2.0",
            "routeKey": route_key,
            "rawPath": MIGRATION_GATE_PATH,
            "rawQueryString": "",
            "headers": { "host": domain },
            "requestContext": {
                "domainName": domain,
                "http": { "method": "GET", "path": MIGRATION_GATE_PATH }
            },
            "isBase64Encoded": false
        })
    }

    #[test]
    fn only_the_internal_candidate_hook_selects_the_migration_gate() {
        // The manifest is the other half of the contract: rename either side
        // and deploys would stop migrating without ever failing.
        let manifest = include_str!("../../../tachyon.yaml");
        assert!(manifest.lines().any(|line| line.trim()
            == format!("routeKey: {MIGRATION_GATE_ROUTE_KEY}")));
        assert!(manifest
            .lines()
            .any(|line| line.trim()
                == format!("rawPath: {MIGRATION_GATE_PATH}")));
        assert!(manifest.lines().any(|line| line.trim()
            == format!("domainName: {MIGRATION_GATE_DOMAIN}")));
        assert!(
            manifest.contains("name: migration-gate-before-activation"),
            "the migration gate hook must stay wired"
        );

        assert!(is_migration_gate_event(&request(gate_payload(
            MIGRATION_GATE_ROUTE_KEY,
            MIGRATION_GATE_DOMAIN
        ))));

        // A public request that merely hits the same path must not migrate,
        // and neither may one that spoofs the route key from outside.
        assert!(!is_migration_gate_event(&request(gate_payload(
            "$default",
            "library-api.txcloud.app"
        ))));
        assert!(!is_migration_gate_event(&request(gate_payload(
            MIGRATION_GATE_ROUTE_KEY,
            "library-api.txcloud.app"
        ))));
    }
}
