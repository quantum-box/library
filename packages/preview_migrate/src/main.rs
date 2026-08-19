use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::process::ExitCode;
use std::sync::OnceLock;

use lambda_runtime::{service_fn, Error as LambdaError, LambdaEvent};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::migrate::{Migration, Migrator};
use sqlx::mysql::MySqlPoolOptions;
use url::Url;

// The per-PR preview database is PrivateLink-only, so this binary runs as
// the `lambda-library-api-preview-migrate` deploy hook inside the
// enterprise-library network (field PLT-3561 pattern). A Lambda has no
// workspace checkout, so both migration sets are embedded at compile time;
// build.rs makes newly added migration files trigger a rebuild.
static DATABASE_MANAGER_MIGRATOR: Migrator =
    sqlx::migrate!("../database-manager/migrations");
static LIBRARY_MIGRATOR: Migrator =
    sqlx::migrate!("../../apps/api/migrations");

const DATABASE_MANAGER_SOURCE: &str =
    "packages/database-manager/migrations";
const LIBRARY_SOURCE: &str = "apps/api/migrations";
const MIGRATION_SOURCES: &[&str] =
    &[DATABASE_MANAGER_SOURCE, LIBRARY_SOURCE];
const LOGICAL_DATABASE_QUALIFIERS: &[&str] =
    &["library", "tachyon_apps_database_manager"];
// These identifiers are quoted only when removing a logical database
// qualifier would otherwise turn valid qualified SQL into invalid MySQL.
// Keep this list restricted to MySQL reserved words actually present in the
// immutable migration sources.
const RESERVED_IDENTIFIERS_AFTER_DEQUALIFICATION: &[&str] = &["databases"];

#[tokio::main]
async fn main() -> ExitCode {
    if env::var("AWS_LAMBDA_RUNTIME_API").is_ok() {
        return match lambda_runtime::run(service_fn(lambda_handler)).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        };
    }

    match run_cli().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Invoke payload for the `provisioned-database-migrate` deploy hook. The
/// platform injects the resolved per-PR connection URL as `databaseUrl` and
/// the manifest's env var name as `databaseUrlEnv`.
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

async fn lambda_handler(
    event: LambdaEvent<MigrationPayload>,
) -> Result<MigrationResponse, LambdaError> {
    let resolved =
        resolve_database_url(event.payload, |name| env::var(name))
            .map_err(LambdaError::from)?;
    run_preview_migrations(&resolved.value, &resolved.source)
        .await
        .map_err(LambdaError::from)?;
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
        "databaseUrl or databaseUrlEnv is required for preview migrations"
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

async fn run_cli() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1))?;
    let raw_database_url =
        env::var(&args.database_url_env).map_err(|_| {
            format!(
                "{} is required for preview migrations",
                args.database_url_env
            )
        })?;
    run_preview_migrations(&raw_database_url, &args.database_url_env).await
}

async fn run_preview_migrations(
    raw_database_url: &str,
    database_url_source: &str,
) -> Result<(), String> {
    if raw_database_url.trim().is_empty() {
        return Err(format!(
            "{database_url_source} must not be empty for preview migrations"
        ));
    }
    // Connect with the raw URL (via url::Url): the provisioned URL carries
    // `?ssl-mode=REQUIRED`, which TiDB enforces, and dropping the query
    // string would silently disable TLS.
    let database_url = parse_database_url(raw_database_url.trim())?;
    let database_name = require_preview_target(
        env::var("TACHYON_ENV").ok().as_deref(),
        &database_url,
    )?
    .to_string();

    let migrator = load_combined_migrator(embedded_migration_sources())?;
    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(database_url.as_str())
        .await
        .map_err(|error| {
            format!(
                "failed to connect to preview database `{database_name}`: {error}"
            )
        })?;
    migration_preflight::ensure_check_constraints_enforced(&pool)
        .await
        .map_err(|error| error.to_string())?;
    migrator.run(&pool).await.map_err(|error| {
        format!(
            "failed to apply {} to preview database `{database_name}`: {error}",
            MIGRATION_SOURCES.join(", ")
        )
    })?;
    pool.close().await;

    println!(
        "preview migrations applied: {} -> {}",
        MIGRATION_SOURCES.join(", "),
        database_name
    );
    Ok(())
}

fn embedded_migration_sources() -> Vec<(String, &'static Migrator)> {
    vec![
        (
            DATABASE_MANAGER_SOURCE.to_string(),
            &DATABASE_MANAGER_MIGRATOR,
        ),
        (LIBRARY_SOURCE.to_string(), &LIBRARY_MIGRATOR),
    ]
}

fn parse_database_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw)
        .map_err(|_| "preview DATABASE_URL is invalid".to_string())?;
    if url.scheme() != "mysql" || url.host_str().is_none() {
        return Err("preview DATABASE_URL is invalid".to_string());
    }
    Ok(url)
}

fn database_name(database_url: &Url) -> Option<&str> {
    let name = database_url.path().trim_start_matches('/');
    (!name.is_empty()).then_some(name)
}

fn require_preview_target<'a>(
    tachyon_environment: Option<&str>,
    database_url: &'a Url,
) -> Result<&'a str, String> {
    if tachyon_environment != Some("preview") {
        return Err(
            "preview migrations require TACHYON_ENV=preview".to_string()
        );
    }

    let database_name = database_name(database_url).ok_or_else(|| {
        "preview DATABASE_URL must select a database".to_string()
    })?;
    if !database_name.starts_with("pr_") {
        return Err(
            "preview DATABASE_URL must select a PR-scoped database"
                .to_string(),
        );
    }
    Ok(database_name)
}

fn load_combined_migrator(
    source_migrators: Vec<(String, &Migrator)>,
) -> Result<Migrator, String> {
    ensure_table_names_do_not_collide(&source_migrators)?;
    combine_migrators(source_migrators)
}

fn combine_migrators(
    source_migrators: Vec<(String, &Migrator)>,
) -> Result<Migrator, String> {
    let mut migrations = BTreeMap::<i64, (Migration, String)>::new();

    // ADR-0049 defines one physical database and therefore one SQLx history
    // per app/PR. Source namespaces are persisted in `_sqlx_migrations`; never
    // renumber an existing source when another migration set is added.
    for (source, migrator) in source_migrators {
        let namespace = migration_source_namespace(&source)?;
        for migration in migrator
            .iter()
            .filter(|migration| migration.migration_type.is_up_migration())
        {
            let mut migration = migration.clone();
            migration.sql = Cow::Owned(strip_logical_database_qualifiers(
                &migration.sql,
            ));
            migration.version = migration
                .version
                .checked_mul(100)
                .and_then(|version| version.checked_add(namespace))
                .ok_or_else(|| {
                    format!(
                        "preview migration version overflow: {}",
                        migration.version
                    )
                })?;

            if let Some((_, existing_source)) = migrations
                .insert(migration.version, (migration, source.clone()))
            {
                return Err(format!(
                    "conflicting preview migration version in {existing_source} and {source}"
                ));
            }
        }
    }

    Ok(Migrator {
        migrations: Cow::Owned(
            migrations
                .into_values()
                .map(|(migration, _)| migration)
                .collect(),
        ),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    })
}

fn migration_source_namespace(source: &str) -> Result<i64, String> {
    match source {
        DATABASE_MANAGER_SOURCE => Ok(1),
        LIBRARY_SOURCE => Ok(2),
        _ => Err(format!(
            "preview migration source `{source}` has no stable namespace"
        )),
    }
}

// PLT-3328 intentionally limits preview/production SQL divergence to removal
// of the two logical database qualifiers. A dequalified MySQL reserved word is
// backtick-quoted so the same identifier remains syntactically valid; no other
// SQL rewrite belongs here.
fn strip_logical_database_qualifiers(sql: &str) -> String {
    static QUALIFIED_IDENTIFIER: OnceLock<Regex> = OnceLock::new();
    let pattern = QUALIFIED_IDENTIFIER.get_or_init(|| {
        let qualifiers = LOGICAL_DATABASE_QUALIFIERS.join("|");
        Regex::new(&format!(
            r"(?i)(?:`?(?:{qualifiers})`?)\.(?P<identifier>`?[A-Za-z_][A-Za-z0-9_]*`?)"
        ))
        .expect("qualified identifier regex must compile")
    });

    pattern
        .replace_all(sql, |captures: &regex::Captures<'_>| {
            let identifier = captures
                .name("identifier")
                .expect("qualified identifier must be captured")
                .as_str();
            let unquoted = identifier.trim_matches('`');
            if RESERVED_IDENTIFIERS_AFTER_DEQUALIFICATION
                .iter()
                .any(|reserved| unquoted.eq_ignore_ascii_case(reserved))
            {
                format!("`{unquoted}`")
            } else {
                identifier.to_string()
            }
        })
        .into_owned()
}

fn ensure_table_names_do_not_collide(
    source_migrators: &[(String, &Migrator)],
) -> Result<(), String> {
    let mut owners = BTreeMap::<String, String>::new();
    for (source, migrator) in source_migrators {
        for table_name in created_table_names(migrator) {
            if let Some(existing_source) =
                owners.insert(table_name.clone(), source.clone())
            {
                if existing_source != *source {
                    return Err(format!(
                        "preview table `{table_name}` is created by both {existing_source} and {source}"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn created_table_names(migrator: &Migrator) -> BTreeSet<String> {
    migrator
        .iter()
        .filter(|migration| migration.migration_type.is_up_migration())
        .flat_map(|migration| created_table_names_in_sql(&migration.sql))
        .collect()
}

fn created_table_names_in_sql(sql: &str) -> BTreeSet<String> {
    static CREATE_TABLE: OnceLock<Regex> = OnceLock::new();
    let pattern = CREATE_TABLE.get_or_init(|| {
        Regex::new(
            r"(?is)\bCREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?((?:`?[A-Za-z0-9_]+`?\.)?`?[A-Za-z0-9_]+`?)",
        )
        .expect("CREATE TABLE regex must compile")
    });

    pattern
        .captures_iter(sql)
        .filter_map(|capture| capture.get(1))
        .filter_map(|identifier| identifier.as_str().rsplit('.').next())
        .map(|identifier| identifier.trim_matches('`').to_ascii_lowercase())
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
struct Args {
    database_url_env: String,
}

impl Args {
    fn parse(
        args: impl IntoIterator<Item = String>,
    ) -> Result<Self, String> {
        let mut database_url_env = "DATABASE_URL".to_string();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--database-url-env" => {
                    database_url_env = iter.next().ok_or_else(|| {
                        "--database-url-env requires a value".to_string()
                    })?;
                }
                other => {
                    return Err(format!("unexpected argument: {other}"))
                }
            }
        }
        Ok(Self { database_url_env })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::migrate::MigrationType;
    use std::sync::atomic::{AtomicU64, Ordering};

    static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn test_migrator(
        version: i64,
        description: &'static str,
        sql: &'static str,
    ) -> Migrator {
        Migrator {
            migrations: Cow::Owned(vec![Migration::new(
                version,
                Cow::Borrowed(description),
                MigrationType::Simple,
                Cow::Borrowed(sql),
                false,
            )]),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        }
    }

    #[test]
    fn accepts_only_pr_scoped_tachyon_preview_target() {
        let preview = parse_database_url(
            "mysql://app:password@host:4000/pr_3328_library",
        )
        .unwrap();
        let shared = parse_database_url(
            "mysql://app:password@host:4000/shared_preview",
        )
        .unwrap();

        assert_eq!(
            require_preview_target(Some("preview"), &preview).unwrap(),
            "pr_3328_library"
        );
        assert!(
            require_preview_target(Some("production"), &preview).is_err()
        );
        assert!(require_preview_target(Some("preview"), &shared).is_err());
    }

    #[test]
    fn preview_database_url_keeps_tls_query_parameters() {
        // The provisioned URL carries `?ssl-mode=REQUIRED`; the parsed URL
        // handed to sqlx must keep it, or TiDB rejects the connection.
        let url = parse_database_url(
            "mysql://app:password@host:4000/pr_3328_library?ssl-mode=REQUIRED",
        )
        .unwrap();

        assert_eq!(
            require_preview_target(Some("preview"), &url).unwrap(),
            "pr_3328_library"
        );
        assert!(url.as_str().ends_with("?ssl-mode=REQUIRED"));
    }

    #[test]
    fn deserializes_platform_payload_contract() {
        let payload: MigrationPayload = serde_json::from_value(json!({
            "databaseUrl": "direct-database-url",
            "databaseUrlEnv": "DATABASE_URL"
        }))
        .unwrap();

        assert_eq!(
            payload.database_url.as_deref(),
            Some("direct-database-url")
        );
        assert_eq!(
            payload.database_url_env.as_deref(),
            Some("DATABASE_URL")
        );
    }

    #[test]
    fn direct_database_url_takes_priority_without_reading_environment() {
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
    fn database_url_env_fallback_reads_named_environment_variable() {
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
            "databaseUrl or databaseUrlEnv is required for preview migrations"
        );
    }

    #[test]
    fn rewrites_only_logical_database_qualifiers() {
        let sql = "INSERT INTO library.repos SELECT * FROM \
                   `tachyon_apps_database_manager`.objects; \
                   ALTER TABLE library.databases ADD COLUMN enabled BOOL; \
                   SELECT library_database FROM metadata;";
        let transformed = strip_logical_database_qualifiers(sql);

        assert_eq!(
            transformed,
            "INSERT INTO repos SELECT * FROM objects; ALTER TABLE \
             `databases` ADD COLUMN enabled BOOL; SELECT \
             library_database FROM metadata;"
        );

        // Removing the one syntax-preserving reserved-word quote leaves
        // exactly the result of qualifier removal and no other rewrite.
        assert_eq!(
            transformed.replace("`databases`", "databases"),
            sql.replace("library.", "")
                .replace("`tachyon_apps_database_manager`.", "")
        );
    }

    #[test]
    fn create_table_parser_normalizes_qualifiers_and_backticks() {
        let tables = created_table_names_in_sql(
            "CREATE TABLE library.repos (id INT);\n\
             CREATE TABLE IF NOT EXISTS `tachyon_apps_database_manager`.`objects` (id INT);",
        );

        assert_eq!(
            tables,
            BTreeSet::from(["objects".to_string(), "repos".to_string()])
        );
    }

    #[test]
    fn rejects_table_names_owned_by_both_migration_sets() {
        let database_manager = test_migrator(
            1,
            "database manager",
            "CREATE TABLE duplicate_table (id INT)",
        );
        let library = test_migrator(
            2,
            "library",
            "CREATE TABLE library.duplicate_table (id INT)",
        );
        let result = ensure_table_names_do_not_collide(&[
            (DATABASE_MANAGER_SOURCE.to_string(), &database_manager),
            (LIBRARY_SOURCE.to_string(), &library),
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn embedded_migration_sets_have_disjoint_table_names() {
        ensure_table_names_do_not_collide(&[
            (
                DATABASE_MANAGER_SOURCE.to_string(),
                &DATABASE_MANAGER_MIGRATOR,
            ),
            (LIBRARY_SOURCE.to_string(), &LIBRARY_MIGRATOR),
        ])
        .unwrap();
    }

    #[test]
    fn embedded_migration_sets_are_not_empty() {
        // The Lambda ships these migrations compiled in; an empty embed
        // would "succeed" while applying nothing.
        assert!(DATABASE_MANAGER_MIGRATOR.iter().next().is_some());
        assert!(LIBRARY_MIGRATOR.iter().next().is_some());
    }

    #[test]
    fn migration_versions_use_stable_source_namespaces() {
        let version = 20260809000000;
        let library = test_migrator(version, "library", "SELECT 1");
        let database_manager =
            test_migrator(version, "database manager", "SELECT 2");
        let migrator = combine_migrators(vec![
            (LIBRARY_SOURCE.to_string(), &library),
            (DATABASE_MANAGER_SOURCE.to_string(), &database_manager),
        ])
        .unwrap();
        let versions = migrator
            .iter()
            .map(|migration| migration.version)
            .collect::<Vec<_>>();

        assert_eq!(versions, vec![version * 100 + 1, version * 100 + 2]);
    }

    #[tokio::test]
    #[ignore = "requires MySQL configured by DEV_DATABASE_URL"]
    async fn preview_migrations_replay_on_empty_mysql() {
        let base_url = parse_database_url(
            env::var("DEV_DATABASE_URL")
                .expect("DEV_DATABASE_URL must be set")
                .trim(),
        )
        .expect("DEV_DATABASE_URL must be valid");
        let database_name = format!(
            "pr_plt3328_replay_{}_{}",
            std::process::id(),
            DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let mut admin_url = base_url.clone();
        admin_url.set_path("/mysql");
        let admin_pool = MySqlPoolOptions::new()
            .max_connections(1)
            .connect(admin_url.as_str())
            .await
            .expect("MySQL must be available");

        // The identifier is generated only from a literal prefix, process ID,
        // and an atomic counter; MySQL cannot bind database identifiers.
        sqlx::query(&format!("CREATE DATABASE `{database_name}`"))
            .execute(&admin_pool)
            .await
            .expect("throwaway preview database must be created");

        let replay_result = async {
            let migrator =
                load_combined_migrator(embedded_migration_sources())?;
            let mut replay_url = base_url.clone();
            replay_url.set_path(&format!("/{database_name}"));
            let pool = MySqlPoolOptions::new()
                .max_connections(1)
                .connect(replay_url.as_str())
                .await
                .map_err(|error| error.to_string())?;
            migration_preflight::ensure_check_constraints_enforced(&pool)
                .await
                .map_err(|error| error.to_string())?;
            let result = match migrator.run(&pool).await {
                Ok(()) => exercise_reserved_database_runtime_queries(&pool)
                    .await
                    .map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            pool.close().await;
            result
        }
        .await;

        let cleanup_result =
            sqlx::query(&format!("DROP DATABASE `{database_name}`"))
                .execute(&admin_pool)
                .await;
        admin_pool.close().await;
        replay_result.expect("transformed preview migrations must replay");
        cleanup_result.expect("throwaway preview database must be removed");
    }

    async fn exercise_reserved_database_runtime_queries(
        pool: &sqlx::MySqlPool,
    ) -> Result<(), sqlx::Error> {
        const PLATFORM_ID: &str = "tn_01j91h09tpj5ehwbwfwfxpak2b";
        const REPO_ID: &str = "rp_01j91h09tpj5ehwbwfwfxpak2b";
        const DATABASE_ID: &str = "db_01j91h09tpj5ehwbwfwfxpak2b";

        // This database is throwaway and single-connection. Disabling FK
        // checks lets the test exercise the exact runtime INSERT without
        // manufacturing unrelated application fixtures.
        sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM `databases` WHERE repo_id = ?")
            .bind(REPO_ID)
            .execute(pool)
            .await?;
        sqlx::query(
            "INSERT INTO `databases` (database_id, repo_id, platform_id) VALUES (?, ?, ?)",
        )
        .bind(DATABASE_ID)
        .bind(REPO_ID)
        .bind(PLATFORM_ID)
        .execute(pool)
        .await?;
        sqlx::query(
            "SELECT id, database_id FROM `databases` WHERE platform_id = ? AND repo_id = ?",
        )
        .bind(PLATFORM_ID)
        .bind(REPO_ID)
        .fetch_all(pool)
        .await?;
        sqlx::query(
            "SELECT id, database_id, repo_id FROM `databases` WHERE platform_id = ? AND repo_id = ?",
        )
        .bind(PLATFORM_ID)
        .bind(REPO_ID)
        .fetch_all(pool)
        .await?;
        sqlx::query(
            "SELECT id, database_id FROM `databases` WHERE platform_id = ? AND repo_id = ?",
        )
        .bind(PLATFORM_ID)
        .bind(REPO_ID)
        .fetch_all(pool)
        .await?;
        sqlx::query(
            "SELECT database_id, repo_id FROM `databases` WHERE platform_id = ? AND repo_id = ?",
        )
        .bind(PLATFORM_ID)
        .bind(REPO_ID)
        .fetch_all(pool)
        .await?;
        sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
            .execute(pool)
            .await?;
        Ok(())
    }
}
