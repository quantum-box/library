//! Migration script (CLI and Lambda).
//!
//! ```bash
//! cargo run -p library-api --bin library_api_migrate dev
//! ```
//!
//! ```bash
//! cargo run -p library-api --bin library_api_migrate prod
//! ```

use std::env;

use lambda_runtime::{service_fn, Error as LambdaError, LambdaEvent};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if env::var("AWS_LAMBDA_RUNTIME_API").is_ok() {
        // Initialize tracing once per execution environment. Doing this
        // inside the handler panics on warm invocations because the global
        // default subscriber can only be set once per process.
        telemetry::init_debug_tracing();
        lambda_runtime::run(service_fn(lambda_handler)).await?;
        return Ok(());
    }

    run_cli().await?;
    Ok(())
}

async fn lambda_handler(
    _event: LambdaEvent<Value>,
) -> Result<Value, LambdaError> {
    tracing::info!("Starting library-api production migration");

    let database_url = library_api::migrations::resolve_prod_database_url()
        .map_err(|error| LambdaError::from(error.to_string()))?;
    library_api::migrations::run_library_migrations(&database_url)
        .await
        .map_err(|error| LambdaError::from(error.to_string()))?;

    tracing::info!(
        "library-api production migration completed successfully"
    );
    Ok(json!({ "status": "ok" }))
}

async fn run_cli() -> errors::Result<()> {
    telemetry::init_debug_tracing();
    let env = env::args().nth(1).unwrap_or_else(|| "dev".to_string());

    match env.as_str() {
        "dev" => {
            tracing::info!("Running migrations in dev environment");

            let database_url: value_object::DatabaseUrl =
                env::var("DEV_DATABASE_URL")
                    .unwrap()
                    .parse()
                    .expect("Invalid database URL");
            library_api::migrations::run_library_migrations(&database_url)
                .await?;
            tracing::info!("Migrations ran successfully");
        }
        "prod" => {
            tracing::info!("Running migrations in prod environment");

            let database_url =
                library_api::migrations::resolve_prod_database_url()?;
            library_api::migrations::run_library_migrations(&database_url)
                .await?;
            tracing::info!("Migrations ran successfully");
        }
        "tidb-playground" => {
            tracing::info!(
                "Running migrations in tidb-playground environment"
            );

            let database_url: value_object::DatabaseUrl =
                "mysql://root@127.0.0.1:4000"
                    .parse()
                    .expect("Invalid database URL");
            library_api::migrations::run_library_migrations(&database_url)
                .await?;
            tracing::info!("Migrations ran successfully");
        }
        _ => {
            panic!("Invalid environment: {env}");
        }
    }

    Ok(())
}
