//! Migration script
//!
//! ```bash
//! cargo run -p library-api --bin library_api_migrate dev
//! ```
//!
//! ```bash
//! cargo run -p library-api --bin library_api_migrate prod
//! ```

use std::env;

use sqlx::{mysql::MySqlPoolOptions, Executor};
use value_object::DatabaseUrl;

#[tokio::main]
async fn main() -> errors::Result<()> {
    const DATABASE_NAME: &str = "library";
    telemetry::init_debug_tracing();
    let env = env::args().nth(1).unwrap_or_else(|| "dev".to_string());

    match env.as_str() {
        "dev" => {
            tracing::info!("Running migrations in dev environment");

            let database_url: DatabaseUrl = env::var("DEV_DATABASE_URL")
                .unwrap()
                .parse()
                .expect("Invalid database URL");
            ensure_database_exists(&database_url, DATABASE_NAME).await?;
            let db = persistence::Db::new(
                database_url.use_database(DATABASE_NAME),
            )
            .await;
            sqlx::migrate!("./migrations")
                .run(db.pool().as_ref())
                .await
                .expect("Failed to run migrations");
            tracing::info!("Migrations ran successfully");
        }
        "prod" => {
            tracing::info!("Running migrations in prod environment");

            let database_url: DatabaseUrl = env::var("PROD_DATABASE_URL")
                .unwrap()
                .parse()
                .expect("Invalid database URL");
            ensure_database_exists(&database_url, DATABASE_NAME).await?;
            let db = persistence::Db::new(
                database_url.use_database(DATABASE_NAME),
            )
            .await;
            sqlx::migrate!("./migrations")
                .run(db.pool().as_ref())
                .await
                .expect("Failed to run migrations");
            tracing::info!("Migrations ran successfully");
        }
        "tidb-playground" => {
            tracing::info!(
                "Running migrations in tidb-playground environment"
            );

            let database_url: DatabaseUrl = "mysql://root@127.0.0.1:4000"
                .parse()
                .expect("Invalid database URL");
            ensure_database_exists(&database_url, DATABASE_NAME).await?;
            let db = persistence::Db::new(
                database_url.use_database(DATABASE_NAME),
            )
            .await;
            sqlx::migrate!("./migrations")
                .run(db.pool().as_ref())
                .await
                .expect("Failed to run migrations");
            tracing::info!("Migrations ran successfully");
        }
        _ => {
            panic!("Invalid environment: {env}");
        }
    }

    Ok(())
}

async fn ensure_database_exists(
    database_url: &DatabaseUrl,
    database_name: &str,
) -> errors::Result<()> {
    let admin_dsn = build_admin_dsn(database_url);
    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&admin_dsn)
        .await
        .map_err(|error| {
            errors::Error::internal_server_error(format!(
                "Failed to connect to database server ({admin_dsn}) to ensure `{database_name}`: {error}"
            ))
        })?;
    let query = format!("CREATE DATABASE IF NOT EXISTS `{database_name}`");
    pool.execute(query.as_str()).await.map_err(|error| {
        errors::Error::internal_server_error(format!(
            "Failed to create database `{database_name}`: {error}"
        ))
    })?;
    pool.close().await;
    Ok(())
}

fn build_admin_dsn(database_url: &DatabaseUrl) -> String {
    let mut credentials = String::new();
    if !database_url.username().is_empty() {
        credentials.push_str(database_url.username());
        if !database_url.password().is_empty() {
            credentials.push(':');
            credentials.push_str(database_url.password());
        }
        credentials.push('@');
    }
    format!(
        "{}://{}{}:{}",
        database_url.scheme(),
        credentials,
        database_url.host(),
        database_url.port()
    )
}
