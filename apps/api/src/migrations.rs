//! Production database migrations for the library-api service.

use sqlx::{mysql::MySqlPoolOptions, Executor};
use value_object::DatabaseUrl;

const DATABASE_NAME: &str = "library";

/// Resolves the production admin database URL from the runtime environment.
///
/// `PROD_DATABASE_URL` is used by the CLI migrate binary. Lambda and txcloud
/// deployments typically expose the same value as `DATABASE_URL`.
pub fn resolve_prod_database_url() -> errors::Result<DatabaseUrl> {
    let raw = std::env::var("PROD_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| {
            errors::Error::bad_request(
                "PROD_DATABASE_URL or DATABASE_URL must be set to run migrations",
            )
        })?;
    raw.parse::<DatabaseUrl>().map_err(|_| {
        errors::Error::bad_request("Invalid production database URL")
    })
}

/// Ensures the `library` schema exists and applies pending sqlx migrations.
pub async fn run_library_migrations(
    admin_database_url: &DatabaseUrl,
) -> errors::Result<()> {
    ensure_database_exists(admin_database_url, DATABASE_NAME).await?;
    let db = persistence::Db::new(
        admin_database_url.use_database(DATABASE_NAME),
    )
    .await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await
        .map_err(|error| {
            errors::Error::internal_server_error(format!(
                "Failed to run library migrations: {error}"
            ))
        })?;
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
                "Failed to connect to database server to ensure `{database_name}` exists: {error}"
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
