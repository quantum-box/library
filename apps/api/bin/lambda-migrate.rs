//! One-shot Lambda entrypoint for production library schema migrations.

use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(service_fn(handler)).await
}

async fn handler(_event: LambdaEvent<Value>) -> Result<Value, Error> {
    telemetry::init_debug_tracing();
    tracing::info!("Starting library-api production migration");

    let database_url = library_api::migrations::resolve_prod_database_url()
        .map_err(|error| Error::from(error.to_string()))?;
    library_api::migrations::run_library_migrations(&database_url)
        .await
        .map_err(|error| Error::from(error.to_string()))?;

    tracing::info!("library-api production migration completed successfully");
    Ok(json!({ "status": "ok" }))
}
