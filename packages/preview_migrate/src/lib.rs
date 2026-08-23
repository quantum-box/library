//! Preview-database migrations for library-api (ADR-0049 / PLT-3328).
//!
//! Two entry points share this logic: the `library_api_preview_migrate`
//! CLI used by CI replays, and the `lambda-library-api-preview-migrate`
//! Lambda the platform invokes as the preview deploy hook. The Lambda
//! has no workspace checkout, so the migration SQL is embedded at
//! compile time rather than read from disk.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;
use sqlx::migrate::{Migration, Migrator};
use sqlx::mysql::{MySqlConnection, MySqlPool, MySqlPoolOptions};
use sqlx::Connection;
use tokio::net::{lookup_host, TcpStream};
use tokio::time::timeout;
use url::Url;

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

static DATABASE_MANAGER_MIGRATOR: Migrator =
    sqlx::migrate!("../database-manager/migrations");
static LIBRARY_MIGRATOR: Migrator =
    sqlx::migrate!("../../apps/api/migrations");

/// How long a single TCP connect attempt may hang before the database is
/// reported as unreachable.
const SOCKET_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// Pool acquire timeout. Kept short because `ensure_socket_reachable` has
/// already proven the socket answers by the time the pool is built.
const POOL_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(15);

/// Apply every migration source to one PR-scoped preview database.
///
/// `environment` is the caller's own assertion that it is a preview
/// runtime (`TACHYON_ENV` for the CLI, `ENVIRONMENT` for the Lambda);
/// `database_url_source` names where the URL came from so an empty value
/// can be reported without printing the URL itself.
pub async fn run_preview_migrations(
    environment: Option<&str>,
    raw_database_url: &str,
    database_url_source: &str,
) -> Result<(), String> {
    require_preview_runtime(environment)?;
    if raw_database_url.trim().is_empty() {
        return Err(format!(
            "{database_url_source} must not be empty for preview \
             migrations"
        ));
    }
    let database_url = parse_database_url(raw_database_url.trim())?;
    let database_name =
        require_pr_scoped_database(&database_url)?.to_string();

    let migrator = load_combined_migrator()?;
    let pool = connect(&database_url, &database_name).await?;
    migration_preflight::ensure_check_constraints_enforced(&pool)
        .await
        .map_err(|error| error.to_string())?;
    clear_failed_sqlx_migrations(&pool).await;
    migrator.run(&pool).await.map_err(|error| {
        format!(
            "failed to apply {} to preview database `{database_name}`: \
             {error}",
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

/// Drop half-applied rows so a retried deploy starts from a clean history.
///
/// A migration that fails mid-DDL leaves `success = FALSE` in
/// `_sqlx_migrations`, and sqlx refuses every later run with "partially
/// applied" until that row is gone. The per-PR database is not recreated
/// between deploys, so without this the first failure would strand the PR
/// until it is closed and reopened. Mirrors the production migrator
/// (`library_api::migrations::clear_failed_sqlx_migrations`). The table may
/// not exist yet on a fresh database, which is why the error is dropped.
async fn clear_failed_sqlx_migrations(pool: &MySqlPool) {
    sqlx::query("DELETE FROM _sqlx_migrations WHERE success = FALSE")
        .execute(pool)
        .await
        .ok();
}

fn require_preview_runtime(
    environment: Option<&str>,
) -> Result<(), String> {
    if environment != Some("preview") {
        return Err(
            "preview migrations require a preview runtime".to_string()
        );
    }
    Ok(())
}

pub fn parse_database_url(raw: &str) -> Result<Url, String> {
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

fn require_pr_scoped_database(database_url: &Url) -> Result<&str, String> {
    let database_name = database_name(database_url).ok_or_else(|| {
        "preview DATABASE_URL must select a database".to_string()
    })?;
    if !is_pr_scoped_database_name(database_name) {
        return Err(
            "preview DATABASE_URL must select a PR-scoped database"
                .to_string(),
        );
    }
    Ok(database_name)
}

/// Recognize the platform's per-PR database names, both shapes.
///
/// tachyon-apps `preview_database_name` produced `pr_{pr}_{app id tail}`
/// until PLT-3851 put the app slug in front: `{slug}_pr{pr}_{app id tail}`.
/// Open PRs keep whichever name they were provisioned with, so both must
/// pass. Everything else -- above all the shared `library` database -- is
/// still refused, because this migrator drops and rewrites schema.
///
/// `library_api::DatabaseLayout` resolves the same question when it decides
/// whether the runtime sees one unified preview database or the two
/// production ones. Both must agree, so both call this.
pub fn is_pr_scoped_database_name(name: &str) -> bool {
    static PR_SCOPED: OnceLock<[Regex; 2]> = OnceLock::new();
    let patterns = PR_SCOPED.get_or_init(|| {
        [
            Regex::new(r"^pr_[0-9]+_[a-z0-9]+$")
                .expect("legacy preview database regex must compile"),
            Regex::new(r"^[a-z0-9][a-z0-9_]*_pr[0-9]+_[a-z0-9]+$")
                .expect("preview database regex must compile"),
        ]
    });

    patterns.iter().any(|pattern| pattern.is_match(name))
}

async fn connect(
    database_url: &Url,
    database_name: &str,
) -> Result<MySqlPool, String> {
    ensure_socket_reachable(database_url).await?;
    match MySqlPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(POOL_ACQUIRE_TIMEOUT)
        .connect(database_url.as_str())
        .await
    {
        Ok(pool) => Ok(pool),
        Err(error) => Err(format!(
            "failed to connect to preview database `{database_name}`: {}",
            describe_connect_error(database_url, error).await
        )),
    }
}

/// PLT-3561: the per-PR TiDB lives behind a PrivateLink endpoint that only
/// resolves to a route from inside the app's VPC. A caller without that
/// route sees its SYN disappear, and `sqlx` reports the resulting silence
/// as `PoolTimedOut` — the same message it uses for genuine pool
/// exhaustion. Probe the socket first so "cannot get there from here" is
/// never mistaken for "the pool is busy".
async fn ensure_socket_reachable(database_url: &Url) -> Result<(), String> {
    let host = database_url.host_str().unwrap_or_default();
    let port = database_url.port().unwrap_or(3306);
    let addresses = match timeout(
        SOCKET_PROBE_TIMEOUT,
        lookup_host((host, port)),
    )
    .await
    {
        Ok(Ok(addresses)) => addresses.collect::<Vec<_>>(),
        Ok(Err(error)) => {
            return Err(format!(
                "preview database host `{host}` does not resolve: {error}"
            ))
        }
        Err(_) => {
            return Err(format!(
                "preview database host `{host}` did not resolve within \
                 {}s",
                SOCKET_PROBE_TIMEOUT.as_secs()
            ))
        }
    };
    if addresses.is_empty() {
        return Err(format!(
            "preview database host `{host}` resolved to no addresses"
        ));
    }

    let mut rejection = None;
    for address in addresses {
        match timeout(SOCKET_PROBE_TIMEOUT, TcpStream::connect(address))
            .await
        {
            Ok(Ok(_)) => return Ok(()),
            Ok(Err(error)) => rejection = Some(error.to_string()),
            // A silent drop, which is what a missing network route looks
            // like. Keep probing the remaining addresses.
            Err(_) => {}
        }
    }

    Err(match rejection {
        Some(error) => format!(
            "preview database `{host}:{port}` rejected the connection: \
             {error}"
        ),
        None => format!(
            "preview database `{host}:{port}` is unreachable: TCP \
             connect got no response within {}s. The per-PR database is \
             reachable only from inside the app's VPC, so the migration \
             must run there (tachyon.yaml \
             provisionedDatabase.migration.lambdaInvoke), not on the \
             deploy-hook runner",
            SOCKET_PROBE_TIMEOUT.as_secs()
        ),
    })
}

/// Recover the error `sqlx` discarded when it rounded a failed connect up
/// to `PoolTimedOut`. Only runs on the failure path.
async fn describe_connect_error(
    database_url: &Url,
    error: sqlx::Error,
) -> String {
    let sqlx::Error::PoolTimedOut = error else {
        return error.to_string();
    };
    match MySqlConnection::connect(database_url.as_str()).await {
        Ok(connection) => {
            let _ = connection.close().await;
            "the pool timed out even though a direct connection \
             succeeded"
                .to_string()
        }
        Err(direct) => direct.to_string(),
    }
}

fn embedded_sources() -> [(&'static str, &'static Migrator); 2] {
    [
        (DATABASE_MANAGER_SOURCE, &DATABASE_MANAGER_MIGRATOR),
        (LIBRARY_SOURCE, &LIBRARY_MIGRATOR),
    ]
}

fn load_combined_migrator() -> Result<Migrator, String> {
    let sources = embedded_sources();
    ensure_table_names_do_not_collide(&sources)?;
    combine_migrators(&sources)
}

fn combine_migrators(
    source_migrators: &[(&str, &Migrator)],
) -> Result<Migrator, String> {
    let mut migrations = BTreeMap::<i64, (Migration, String)>::new();

    // ADR-0049 defines one physical database and therefore one SQLx history
    // per app/PR. Source namespaces are persisted in `_sqlx_migrations`; never
    // renumber an existing source when another migration set is added.
    for (source, migrator) in source_migrators {
        let namespace = migration_source_namespace(source)?;
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

            if let Some((_, existing_source)) = migrations.insert(
                migration.version,
                (migration, (*source).to_string()),
            ) {
                return Err(format!(
                    "conflicting preview migration version in \
                     {existing_source} and {source}"
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
    source_migrators: &[(&str, &Migrator)],
) -> Result<(), String> {
    let mut owners = BTreeMap::<String, String>::new();
    for (source, migrator) in source_migrators {
        for table_name in created_table_names(migrator) {
            if let Some(existing_source) =
                owners.insert(table_name.clone(), (*source).to_string())
            {
                if existing_source != *source {
                    return Err(format!(
                        "preview table `{table_name}` is created by both \
                         {existing_source} and {source}"
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::migrate::MigrationType;
    use std::env;
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

        assert!(require_preview_runtime(Some("preview")).is_ok());
        assert!(require_preview_runtime(Some("production")).is_err());
        assert!(require_preview_runtime(None).is_err());
        assert_eq!(
            require_pr_scoped_database(&preview).unwrap(),
            "pr_3328_library"
        );
        assert!(require_pr_scoped_database(&shared).is_err());
    }

    #[test]
    fn accepts_both_platform_preview_database_names() {
        // Legacy shape, still carried by PRs provisioned before PLT-3851.
        assert!(is_pr_scoped_database_name("pr_221_k6cypbws3q3j"));
        // Current shape: the app slug precedes the PR scope.
        assert!(is_pr_scoped_database_name(
            "library_api_pr224_k6cypbws3q3j"
        ));

        // The shared production databases must never be migrated here,
        // and neither may a name that only mentions a PR in passing.
        assert!(!is_pr_scoped_database_name("library"));
        assert!(!is_pr_scoped_database_name(
            "tachyon_apps_database_manager"
        ));
        assert!(!is_pr_scoped_database_name("library_api_preview"));
        assert!(!is_pr_scoped_database_name("pr_221"));
        assert!(!is_pr_scoped_database_name("_pr224_k6cypbws3q3j"));
    }

    #[tokio::test]
    async fn refused_socket_is_not_reported_as_a_pool_timeout() {
        // Port 1 on the loopback interface has no listener, so the
        // kernel answers with RST. The probe must name that answer
        // instead of letting sqlx flatten it into `PoolTimedOut`.
        let url =
            parse_database_url("mysql://app:password@127.0.0.1:1/pr_1_x")
                .unwrap();

        let error = ensure_socket_reachable(&url).await.unwrap_err();

        assert!(
            error.contains("rejected the connection"),
            "unexpected error: {error}"
        );
        assert!(!error.contains("pool timed out"));
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
            (DATABASE_MANAGER_SOURCE, &database_manager),
            (LIBRARY_SOURCE, &library),
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn repository_migration_sets_have_disjoint_table_names() {
        ensure_table_names_do_not_collide(&embedded_sources()).unwrap();
    }

    #[test]
    fn embedded_migration_sets_are_not_empty() {
        // The Lambda has no workspace to fall back on: an empty embedded
        // set would silently "succeed" against an empty preview database.
        for (source, migrator) in embedded_sources() {
            assert!(
                migrator.iter().next().is_some(),
                "{source} embedded no migrations"
            );
        }
    }

    #[test]
    fn migration_versions_use_stable_source_namespaces() {
        let version = 20260809000000;
        let library = test_migrator(version, "library", "SELECT 1");
        let database_manager =
            test_migrator(version, "database manager", "SELECT 2");
        let migrator = combine_migrators(&[
            (LIBRARY_SOURCE, &library),
            (DATABASE_MANAGER_SOURCE, &database_manager),
        ])
        .unwrap();
        let versions = migrator
            .iter()
            .map(|migration| migration.version)
            .collect::<Vec<_>>();

        assert_eq!(versions, vec![version * 100 + 1, version * 100 + 2]);
    }

    fn with_database(base_url: &Url, database: &str) -> Url {
        let mut url = base_url.clone();
        url.set_path(&format!("/{database}"));
        url
    }

    #[tokio::test]
    #[ignore = "requires MySQL configured by DEV_DATABASE_URL"]
    async fn preview_migrations_replay_on_empty_mysql() {
        let base_url = parse_database_url(
            &env::var("DEV_DATABASE_URL")
                .expect("DEV_DATABASE_URL must be set"),
        )
        .expect("DEV_DATABASE_URL must be valid");
        // Shaped like a real per-PR database so the replay also proves the
        // name the platform hands us now clears `require_pr_scoped_database`.
        let database_name = format!(
            "library_api_pr{}_replay{}",
            std::process::id(),
            DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let admin_pool = MySqlPoolOptions::new()
            .max_connections(1)
            .connect(with_database(&base_url, "mysql").as_str())
            .await
            .expect("MySQL must be available");

        // The identifier is generated only from a literal prefix, process ID,
        // and an atomic counter; MySQL cannot bind database identifiers.
        sqlx::query(&format!("CREATE DATABASE `{database_name}`"))
            .execute(&admin_pool)
            .await
            .expect("throwaway preview database must be created");

        let replay_result = async {
            let preview_url = with_database(&base_url, &database_name);
            run_preview_migrations(
                Some("preview"),
                preview_url.as_str(),
                "DEV_DATABASE_URL",
            )
            .await?;
            let pool = MySqlPoolOptions::new()
                .max_connections(1)
                .connect(preview_url.as_str())
                .await
                .map_err(|error| error.to_string())?;
            let result = exercise_reserved_database_runtime_queries(&pool)
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
