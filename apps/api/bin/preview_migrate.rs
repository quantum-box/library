use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::Path;
use std::process::ExitCode;
use std::sync::OnceLock;

use regex::Regex;
use sqlx::migrate::{Migration, Migrator};
use sqlx::mysql::MySqlPoolOptions;
use value_object::DatabaseUrl;

const DATABASE_MANAGER_SOURCE: &str =
    "packages/database-manager/migrations";
const LIBRARY_SOURCE: &str = "apps/api/migrations";
const MIGRATION_SOURCES: &[&str] =
    &[DATABASE_MANAGER_SOURCE, LIBRARY_SOURCE];
const LOGICAL_DATABASE_QUALIFIERS: &[&str] =
    &["library", "tachyon_apps_database_manager"];

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1))?;
    let raw_database_url =
        env::var(&args.database_url_env).map_err(|_| {
            format!(
                "{} is required for preview migrations",
                args.database_url_env
            )
        })?;
    if raw_database_url.trim().is_empty() {
        return Err(format!(
            "{} must not be empty for preview migrations",
            args.database_url_env
        ));
    }
    let database_url: DatabaseUrl = raw_database_url
        .parse()
        .map_err(|_| "preview DATABASE_URL is invalid".to_string())?;
    let database_name = require_preview_target(
        env::var("TACHYON_ENV").ok().as_deref(),
        &database_url,
    )?;

    let migrator =
        load_combined_migrator(workspace_root(), MIGRATION_SOURCES).await?;
    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&database_url.to_string())
        .await
        .map_err(|error| {
            format!(
                "failed to connect to preview database `{database_name}`: {error}"
            )
        })?;
    database_manager::migration_preflight::ensure_check_constraints_enforced(
        &pool,
    )
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

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("apps/api must be two levels below the workspace root")
}

fn require_preview_target<'a>(
    tachyon_environment: Option<&str>,
    database_url: &'a DatabaseUrl,
) -> Result<&'a str, String> {
    if tachyon_environment != Some("preview") {
        return Err(
            "preview migrations require TACHYON_ENV=preview".to_string()
        );
    }

    let database_name =
        database_url.database().as_deref().ok_or_else(|| {
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

async fn load_combined_migrator(
    workspace_root: &Path,
    sources: &[&str],
) -> Result<Migrator, String> {
    let mut source_migrators = Vec::with_capacity(sources.len());
    for source in sources {
        let migrator = Migrator::new(workspace_root.join(source).as_path())
            .await
            .map_err(|error| {
                format!(
                    "failed to load preview migrations from {source}: {error}"
                )
            })?;
        source_migrators.push(((*source).to_string(), migrator));
    }
    ensure_table_names_do_not_collide(&source_migrators)?;
    combine_migrators(source_migrators)
}

fn combine_migrators(
    source_migrators: Vec<(String, Migrator)>,
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
// of the two logical database qualifiers. No other SQL rewrite belongs here.
fn strip_logical_database_qualifiers(sql: &str) -> String {
    LOGICAL_DATABASE_QUALIFIERS.iter().fold(
        sql.to_string(),
        |sql, qualifier| {
            sql.replace(&format!("{qualifier}."), "")
                .replace(&format!("`{qualifier}`."), "")
        },
    )
}

fn ensure_table_names_do_not_collide(
    source_migrators: &[(String, Migrator)],
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
        let preview: DatabaseUrl =
            "mysql://app:password@host:4000/pr_3328_library"
                .parse()
                .unwrap();
        let shared: DatabaseUrl =
            "mysql://app:password@host:4000/shared_preview"
                .parse()
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
    fn rewrites_only_logical_database_qualifiers() {
        let sql = "INSERT INTO library.repos SELECT * FROM \
                   `tachyon_apps_database_manager`.objects;";

        assert_eq!(
            strip_logical_database_qualifiers(sql),
            "INSERT INTO repos SELECT * FROM objects;"
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
        let result = ensure_table_names_do_not_collide(&[
            (
                DATABASE_MANAGER_SOURCE.to_string(),
                test_migrator(
                    1,
                    "database manager",
                    "CREATE TABLE duplicate_table (id INT)",
                ),
            ),
            (
                LIBRARY_SOURCE.to_string(),
                test_migrator(
                    2,
                    "library",
                    "CREATE TABLE library.duplicate_table (id INT)",
                ),
            ),
        ]);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn repository_migration_sets_have_disjoint_table_names() {
        let mut source_migrators = Vec::new();
        for source in MIGRATION_SOURCES {
            let migrator =
                Migrator::new(workspace_root().join(source).as_path())
                    .await
                    .unwrap();
            source_migrators.push(((*source).to_string(), migrator));
        }

        ensure_table_names_do_not_collide(&source_migrators).unwrap();
    }

    #[test]
    fn migration_versions_use_stable_source_namespaces() {
        let version = 20260809000000;
        let migrator = combine_migrators(vec![
            (
                LIBRARY_SOURCE.to_string(),
                test_migrator(version, "library", "SELECT 1"),
            ),
            (
                DATABASE_MANAGER_SOURCE.to_string(),
                test_migrator(version, "database manager", "SELECT 2"),
            ),
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
        let base_url: DatabaseUrl = env::var("DEV_DATABASE_URL")
            .expect("DEV_DATABASE_URL must be set")
            .parse()
            .expect("DEV_DATABASE_URL must be valid");
        let database_name = format!(
            "pr_plt3328_replay_{}_{}",
            std::process::id(),
            DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let admin_pool = MySqlPoolOptions::new()
            .max_connections(1)
            .connect(&base_url.use_database("mysql").to_string())
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
                load_combined_migrator(workspace_root(), MIGRATION_SOURCES)
                    .await?;
            let pool = MySqlPoolOptions::new()
                .max_connections(1)
                .connect(
                    &base_url.use_database(&database_name).to_string(),
                )
                .await
                .map_err(|error| error.to_string())?;
            database_manager::migration_preflight::ensure_check_constraints_enforced(
                &pool,
            )
            .await
            .map_err(|error| error.to_string())?;
            let result = migrator
                .run(&pool)
                .await
                .map_err(|error| error.to_string());
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
}
