//! Preview migration Lambda (PLT-3561).
//!
//! The per-PR TiDB is PrivateLink-only, so the deploy-hook runner has no
//! route to it. The platform therefore invokes this function as a
//! `provisionedDatabase.migration.lambdaInvoke` hook from inside the
//! `enterprise-library` network and passes the resolved connection URL in
//! the payload (`databaseUrl`, falling back to `databaseUrlEnv`).

use std::env;
use std::io;

use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use library_api_preview_migrate::run_preview_migrations;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MigrationPayload {
    database_url: Option<String>,
    database_url_env: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MigrationResponse {
    migration_applied: bool,
}

struct ResolvedDatabaseUrl {
    value: String,
    source: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(handle)).await
}

async fn handle(
    event: LambdaEvent<MigrationPayload>,
) -> Result<MigrationResponse, Error> {
    let resolved =
        resolve_database_url(event.payload, |name| env::var(name))
            .map_err(runtime_error)?;

    // This function exists only to migrate preview databases, so it
    // asserts its own environment rather than trusting the payload.
    run_preview_migrations(
        env::var("ENVIRONMENT").ok().as_deref(),
        &resolved.value,
        &resolved.source,
    )
    .await
    .map_err(runtime_error)?;

    Ok(MigrationResponse {
        migration_applied: true,
    })
}

fn resolve_database_url<F, E>(
    payload: MigrationPayload,
    lookup_env: F,
) -> Result<ResolvedDatabaseUrl, String>
where
    F: FnOnce(&str) -> Result<String, E>,
{
    if let Some(database_url) = payload.database_url {
        return Ok(ResolvedDatabaseUrl {
            value: database_url,
            source: "databaseUrl".to_string(),
        });
    }

    let database_url_env = payload.database_url_env.ok_or_else(|| {
        "databaseUrl or databaseUrlEnv is required for preview \
         migrations"
            .to_string()
    })?;
    if database_url_env.trim().is_empty() {
        return Err(
            "databaseUrlEnv must not be empty for preview migrations"
                .to_string(),
        );
    }
    let database_url = lookup_env(&database_url_env).map_err(|_| {
        format!("{database_url_env} is required for preview migrations")
    })?;

    Ok(ResolvedDatabaseUrl {
        value: database_url,
        source: database_url_env,
    })
}

fn runtime_error(message: String) -> Error {
    // The message otherwise reaches only the invoker's response payload,
    // which the deploy hook reduces to "lambdaInvoke returned
    // FunctionError: Unhandled" -- leaving CloudWatch with a START and an
    // END and no reason. Put it in the log too, so the function's own
    // logs explain a failed migration.
    eprintln!("preview migration failed: {message}");
    io::Error::other(message).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deserializes_platform_payload_contract() {
        let payload: MigrationPayload = serde_json::from_value(json!({
            "databaseUrl": "mysql://app:password@host:4000/pr_1_x",
            "databaseUrlEnv": "DATABASE_URL"
        }))
        .unwrap();

        assert_eq!(
            payload.database_url.as_deref(),
            Some("mysql://app:password@host:4000/pr_1_x")
        );
        assert_eq!(
            payload.database_url_env.as_deref(),
            Some("DATABASE_URL")
        );
    }

    #[test]
    fn direct_database_url_takes_priority_over_the_environment() {
        let resolved = resolve_database_url(
            MigrationPayload {
                database_url: Some("direct-database-url".to_string()),
                database_url_env: Some("DATABASE_URL".to_string()),
            },
            |_| -> Result<String, ()> {
                panic!("environment fallback must not be read")
            },
        )
        .unwrap();

        assert_eq!(resolved.value, "direct-database-url");
        assert_eq!(resolved.source, "databaseUrl");
    }

    #[test]
    fn database_url_env_fallback_reads_the_named_variable() {
        let resolved = resolve_database_url(
            MigrationPayload {
                database_url: None,
                database_url_env: Some("PREVIEW_DATABASE_URL".to_string()),
            },
            |name| {
                assert_eq!(name, "PREVIEW_DATABASE_URL");
                Ok::<_, ()>("fallback-database-url".to_string())
            },
        )
        .unwrap();

        assert_eq!(resolved.value, "fallback-database-url");
        assert_eq!(resolved.source, "PREVIEW_DATABASE_URL");
    }

    #[test]
    fn missing_database_url_contract_fails_closed() {
        let error = resolve_database_url(
            MigrationPayload {
                database_url: None,
                database_url_env: None,
            },
            |_| Ok::<_, ()>(String::new()),
        )
        .err()
        .unwrap();

        assert_eq!(
            error,
            "databaseUrl or databaseUrlEnv is required for preview \
             migrations"
        );
    }

    #[test]
    fn missing_named_environment_variable_does_not_expose_a_value() {
        let error = resolve_database_url(
            MigrationPayload {
                database_url: None,
                database_url_env: Some("PREVIEW_DATABASE_URL".to_string()),
            },
            |_| Err::<String, _>(()),
        )
        .err()
        .unwrap();

        assert_eq!(
            error,
            "PREVIEW_DATABASE_URL is required for preview migrations"
        );
    }
}
