//! Production database migrations for the library-api service.

use sqlx::{mysql::MySqlPoolOptions, Executor};
use value_object::DatabaseUrl;

use crate::database_layout::{
    DATABASE_MANAGER_DATABASE_NAME, LIBRARY_DATABASE_NAME,
};

/// SHA-384 of the current `20250305080000_update_unique_constraints.up.sql`.
///
/// Production applied an earlier revision of that file, so its recorded
/// checksum is realigned to this value before sqlx verifies the history.
/// `pinned_checksum_matches_the_migration_file` fails if the file is edited
/// again without refreshing this constant.
const UPDATE_UNIQUE_CONSTRAINTS_CHECKSUM: &str = "ad8c3fffc2061424884916a14a1e11b5b8191af96291eab12ef0bfda077692cbb641b3224fc47bf167c744fbd7f65ae1";

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

/// Ensures schemas required by library-api exist and applies pending migrations.
///
/// PLT-3328: this production-only path deliberately keeps two physical
/// databases and two SQLx histories. Only the separate preview migrator may
/// combine them into the one app/PR database required by ADR-0049.
pub async fn run_library_migrations(
    admin_database_url: &DatabaseUrl,
) -> errors::Result<()> {
    run_database_manager_migrations(admin_database_url).await?;
    run_app_migrations(admin_database_url).await?;
    Ok(())
}

async fn run_database_manager_migrations(
    admin_database_url: &DatabaseUrl,
) -> errors::Result<()> {
    ensure_database_exists(
        admin_database_url,
        DATABASE_MANAGER_DATABASE_NAME,
    )
    .await?;
    let db = persistence::Db::new(
        admin_database_url.use_database(DATABASE_MANAGER_DATABASE_NAME),
    )
    .await;
    database_manager::migration_preflight::ensure_check_constraints_enforced(
        db.pool().as_ref(),
    )
    .await?;
    clear_failed_sqlx_migrations(&db).await;
    sqlx::migrate!("../../packages/database-manager/migrations")
        .run(db.pool().as_ref())
        .await
        .map_err(|error| {
            errors::Error::internal_server_error(format!(
                "Failed to run database-manager migrations: {error}"
            ))
        })?;
    Ok(())
}

async fn run_app_migrations(
    admin_database_url: &DatabaseUrl,
) -> errors::Result<()> {
    ensure_database_exists(admin_database_url, LIBRARY_DATABASE_NAME)
        .await?;
    let db = persistence::Db::new(
        admin_database_url.use_database(LIBRARY_DATABASE_NAME),
    )
    .await;
    clear_failed_sqlx_migrations(&db).await;
    // PLT-1808: 20241116030727 was applied on production before 1d2b766 added
    // IF NOT EXISTS (checksum drift). Restore the e769b7f SHA-384 checksum so
    // sqlx accepts the reverted migration file without re-running DDL.
    sqlx::query(
        "UPDATE _sqlx_migrations \
         SET checksum = UNHEX(?) \
         WHERE version = 20241116030727 AND success = TRUE",
    )
    .bind("16b8c3a465b012e1aa9fa3438e5a91cf7f8fd6691d0032ad87869d4d59741022348d7d89ee062c7b04a862b4ee8a8d31")
    .execute(db.pool().as_ref())
    .await
    .ok();
    // 20250305080000 dropped a UNIQUE key with `DROP CONSTRAINT`, which TiDB
    // resolves to CHECK constraints only and rejects with ERROR 3940 once
    // tidb_enable_check_constraint is ON. The statement is now `DROP INDEX`,
    // which has the same effect on the schema production already carries, so
    // realign the recorded checksum rather than re-run the DDL.
    sqlx::query(
        "UPDATE _sqlx_migrations \
         SET checksum = UNHEX(?) \
         WHERE version = 20250305080000 AND success = TRUE",
    )
    .bind(UPDATE_UNIQUE_CONSTRAINTS_CHECKSUM)
    .execute(db.pool().as_ref())
    .await
    .ok();
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

async fn clear_failed_sqlx_migrations(db: &persistence::Db) {
    // Remove any partially applied migrations so they can be re-run cleanly.
    sqlx::query("DELETE FROM _sqlx_migrations WHERE success = FALSE")
        .execute(db.pool().as_ref())
        .await
        .ok();
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The pin only works while it names the file sqlx actually embeds.
    #[test]
    fn pinned_checksum_matches_the_migration_file() {
        let migrator = sqlx::migrate!("./migrations");
        let migration = migrator
            .iter()
            .find(|migration| {
                migration.version == 20250305080000
                    && migration.migration_type.is_up_migration()
            })
            .expect("20250305080000 must stay in the library migrations");

        let checksum = migration
            .checksum
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        assert_eq!(
            checksum, UPDATE_UNIQUE_CONSTRAINTS_CHECKSUM,
            "20250305080000 changed; refresh \
             UPDATE_UNIQUE_CONSTRAINTS_CHECKSUM so production keeps \
             accepting its already-applied history"
        );
    }
}
