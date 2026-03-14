//! Scenario test runner for library-api.
//!
//! Spawns tachyon-api (auth backend) and library-api, then executes
//! YAML-based scenario tests against the library-api HTTP endpoints.

extern crate muon;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use muon::{
    api_client::TachyonOpsClient, DefaultTestRunner, StepResult,
    TestConfigManager, TestResult, TestRunReport, TestRunner, TestScenario,
};
use serde::Deserialize;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::OnceCell;
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

const CONFIG_CANDIDATES: &[&str] = &[
    "tests/config/runtime.yaml",
    "tests/config/ci.yaml",
    "tests/config/default.yaml",
];

static DATABASE_SEEDED: OnceCell<()> = OnceCell::const_new();

fn init_tracing() {
    if tracing::dispatcher::has_been_set() {
        return;
    }

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }

    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .try_init();
}

#[tokio::test]
async fn run_library_api_scenarios() -> Result<()> {
    init_tracing();

    let runtime_config = RuntimeConfig::load()?;
    let base_url = runtime_config.resolve_base_url();

    let server = spawn_test_servers(&runtime_config, &base_url).await;
    let mut server_guard = match server {
        Ok(guard) => guard,
        Err(err) => return Err(err),
    };

    let total_start = Instant::now();
    let (results, run_result) =
        execute_scenarios(&runtime_config.scenario, &base_url).await;

    // Submit report to Ops API if enabled
    if runtime_config.report.enabled && !results.is_empty() {
        submit_report(
            &runtime_config.report,
            &base_url,
            results,
            total_start.elapsed().as_millis() as u64,
        )
        .await;
    }

    server_guard.shutdown().await;

    run_result
}

async fn execute_scenarios(
    scenario_config: &ScenarioConfig,
    base_url: &str,
) -> (Vec<TestResult>, Result<()>) {
    let mut config_manager = TestConfigManager::new();
    config_manager.add_path("tests/scenarios");

    let mut scenarios = match config_manager.load_all_scenarios() {
        Ok(s) => s,
        Err(e) => return (vec![], Err(e)),
    };
    if scenarios.is_empty() {
        info!("テストシナリオが見つかりません。");
        return (vec![], Ok(()));
    }

    if !scenario_config.include.is_empty() {
        let filters: Vec<_> =
            scenario_config.include.iter().map(String::as_str).collect();
        scenarios.retain(|scenario| {
            filters
                .iter()
                .any(|pattern| scenario.name.contains(pattern))
        });

        if scenarios.is_empty() {
            info!(
                "フィルタに一致するシナリオがありません。filters={:?}",
                scenario_config.include
            );
            return (vec![], Ok(()));
        }
    }

    info!("{}個のテストシナリオを実行します...", scenarios.len());

    let runner = DefaultTestRunner::new();
    let mut collected: Vec<TestResult> = Vec::new();

    for mut scenario in scenarios.drain(..) {
        prepare_scenario(&mut scenario, base_url);

        info!("=== シナリオ: {} ===", scenario.name);
        match runner.run(&scenario).await {
            Ok(result) => {
                for (i, step) in result.steps.iter().enumerate() {
                    print_step_result(i, step);
                }

                let failed = !result.success;
                let err_msg = result.error.clone();
                let scenario_name = scenario.name.clone();

                if result.success {
                    info!(
                        "--- ✅ シナリオ成功 ({}ms) ---",
                        result.duration_ms
                    );
                } else if let Some(err) = &err_msg {
                    error!("--- ❌ シナリオ失敗: {} ---", err);
                } else {
                    error!("--- ❌ シナリオ失敗 ---");
                }

                collected.push(result);

                if failed {
                    let msg = match err_msg {
                        Some(err) => format!(
                            "シナリオ '{}' が失敗しました: {}",
                            scenario_name, err
                        ),
                        None => format!(
                            "シナリオ '{}' が失敗しました",
                            scenario_name
                        ),
                    };
                    return (collected, Err(anyhow!(msg)));
                }
            }
            Err(e) => {
                error!("--- ❌ シナリオ実行エラー: {} ---", e);
                return (
                    collected,
                    Err(anyhow!(
                        "シナリオ '{}' の実行に失敗しました: {}",
                        scenario.name,
                        e
                    )),
                );
            }
        }
    }

    info!("=== ✅ すべてのシナリオが成功しました ===");
    (collected, Ok(()))
}

fn prepare_scenario(scenario: &mut TestScenario, base_url: &str) {
    scenario.config.base_url = Some(base_url.to_string());

    scenario.vars.insert(
        "base_url".to_string(),
        Value::String(base_url.to_string()),
    );

    let timestamp = Utc::now().timestamp();
    scenario.vars.insert(
        "timestamp".to_string(),
        Value::String(timestamp.to_string()),
    );
}

/// Spawn tachyon-api (auth backend) and library-api, returning a guard
/// that shuts both down on drop.
async fn spawn_test_servers(
    runtime_config: &RuntimeConfig,
    base_url: &str,
) -> Result<TestServerGuard> {
    ensure_database_seeded(runtime_config).await?;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow!("Failed to resolve workspace root"))?;

    // 1. Spawn tachyon-api if configured
    let mut tachyon_api_child: Option<Child> = None;
    let mut tachyon_api_log_handles: Vec<JoinHandle<()>> = Vec::new();
    let tachyon_api_url: String;

    if let Some(tachyon_config) = &runtime_config.tachyon_api {
        let tachyon_binary = workspace_root
            .join("target/debug")
            .join(&tachyon_config.binary);
        info!("tachyon-api を起動します: {}", tachyon_binary.display());

        let mut cmd = Command::new(&tachyon_binary);
        cmd.current_dir(workspace_root);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::inherit());
        cmd.env("RUST_LOG", "error");

        for (key, value) in &tachyon_config.env {
            cmd.env(key, value);
        }

        let mut child =
            cmd.spawn().context("tachyon-apiの起動に失敗しました")?;

        // Wait for tachyon-api health
        let health_url = format!(
            "http://{}{}",
            tachyon_config.address, tachyon_config.health_path
        );
        let timeout_dur =
            Duration::from_secs(tachyon_config.health_timeout_seconds);
        let interval =
            Duration::from_secs(tachyon_config.health_interval_seconds);

        if let Err(err) = wait_for_health_generic(
            &mut child,
            &health_url,
            timeout_dur,
            interval,
            "tachyon-api",
        )
        .await
        {
            error!("tachyon-api ヘルスチェック失敗: {err}");
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(err);
        }

        tachyon_api_url = format!("http://{}", tachyon_config.address);
        tachyon_api_child = Some(child);
    } else {
        // No tachyon_api config; use TACHYON_API_URL env or default
        tachyon_api_url = std::env::var("TACHYON_API_URL")
            .unwrap_or_else(|_| "http://localhost:50054".to_string());
        info!(
            "tachyon_api config not found, using TACHYON_API_URL={}",
            tachyon_api_url
        );
    }

    // 2. Spawn library-api
    let binary_path = runtime_config.server.binary_path();
    info!("library-api を起動します: {}", binary_path.display());

    let mut command = Command::new(binary_path);
    command.current_dir(&manifest_dir);

    if !runtime_config.logging.forward_stdout
        && !runtime_config.logging.forward_stderr
    {
        command.env("RUST_LOG", "error");
    }

    if runtime_config.logging.forward_stdout {
        command.stdout(Stdio::piped());
    } else {
        command.stdout(Stdio::null());
    }

    if runtime_config.logging.forward_stderr {
        command.stderr(Stdio::piped());
    } else {
        command.stderr(Stdio::inherit());
    }

    for (key, value) in runtime_config.dependencies.env_vars() {
        command.env(key, value);
    }

    // Point library-api at tachyon-api
    command.env("TACHYON_API_URL", &tachyon_api_url);

    let mut child =
        command.spawn().context("library-apiの起動に失敗しました")?;

    let mut log_handles = Vec::new();
    if runtime_config.logging.forward_stdout {
        if let Some(stdout) = child.stdout.take() {
            log_handles.push(spawn_log_forwarder(stdout, "STDOUT"));
        }
    }
    if runtime_config.logging.forward_stderr {
        if let Some(stderr) = child.stderr.take() {
            log_handles.push(spawn_log_forwarder(stderr, "STDERR"));
        }
    }

    let health_url = runtime_config.server.health_url(base_url);
    let timeout_dur = Duration::from_secs(
        runtime_config.server.health_check.timeout_seconds,
    );
    let interval = Duration::from_secs(
        runtime_config.server.health_check.interval_seconds,
    );

    if let Err(err) = wait_for_health_generic(
        &mut child,
        &health_url,
        timeout_dur,
        interval,
        "library-api",
    )
    .await
    {
        error!("ヘルスチェックに失敗しました: {err}");
        for handle in log_handles.drain(..) {
            handle.abort();
        }
        if let Err(kill_err) = child.start_kill() {
            error!("library-apiの停止に失敗しました: {kill_err}");
        }
        if let Err(wait_err) = child.wait().await {
            error!("library-apiの終了待ちでエラー: {wait_err}");
        }
        // Also clean up tachyon-api
        if let Some(mut ta_child) = tachyon_api_child {
            let _ = ta_child.start_kill();
            let _ = ta_child.wait().await;
        }
        return Err(err);
    }

    // Merge log handles
    log_handles.extend(tachyon_api_log_handles.drain(..));

    Ok(TestServerGuard::new(child, tachyon_api_child, log_handles))
}

async fn ensure_database_seeded(
    runtime_config: &RuntimeConfig,
) -> Result<()> {
    // When running in CI, the rust_action already handles migrations
    // and seeding. Skip the duplicate DB setup to save time.
    if std::env::var("SCENARIO_TEST_SKIP_DB_SETUP").is_ok() {
        info!(
            "SCENARIO_TEST_SKIP_DB_SETUP is set, skipping migrations and seeding"
        );
        return Ok(());
    }

    DATABASE_SEEDED
        .get_or_try_init(|| async {
            run_yaml_seeder(runtime_config).await?;
            Ok(())
        })
        .await
        .map(|_| ())
}

async fn run_yaml_seeder(runtime_config: &RuntimeConfig) -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root =
        manifest_dir.parent().and_then(|p| p.parent()).ok_or_else(
            || anyhow!("Failed to resolve workspace root for seeding"),
        )?;

    let base_database_url = runtime_config
        .dependencies
        .database_root_url
        .as_deref()
        .or_else(|| {
            runtime_config
                .dependencies
                .database_url
                .as_deref()
                .and_then(|url| url.rsplit_once('/').map(|(base, _)| base))
        })
        .ok_or_else(|| {
            anyhow!(
                "database_root_url or database_url must be provided in test config"
            )
        })?;

    info!(
        "yaml-seeder を実行してデータベースを初期化します (DATABASE_URL={})",
        base_database_url
    );

    let trimmed_base = base_database_url.trim_end_matches('/');

    // Migrate all databases needed by library-api and tachyon-api.
    // This list matches docker-sqlx-migrate / sqlx-migrate-all.
    let migrations: &[(&str, &str)] = &[
        ("packages/auth/migrations", "tachyon_apps_auth"),
        ("packages/payment/migrations", "tachyon_apps_payment"),
        (
            "packages/database/migrations",
            "tachyon_apps_database_manager",
        ),
        ("packages/llms/migrations", "tachyon_apps_llms"),
        ("packages/iac/migrations", "tachyon_apps_iac"),
        (
            "packages/source_explore/migrations",
            "tachyon_apps_source_explore",
        ),
        (
            "packages/feature_flag/migrations",
            "tachyon_apps_feature_flag",
        ),
        ("packages/order/migrations", "tachyon_apps_order"),
        ("packages/delivery/migrations", "tachyon_apps_delivery"),
        ("packages/crm/migrations", "tachyon_apps_crm"),
        (
            "packages/procurement/migrations",
            "tachyon_apps_procurement",
        ),
        ("packages/pricing/migrations", "tachyon_apps_pricing"),
        ("packages/taskflow/migrations", "tachyon_apps_taskflow"),
        (
            "packages/scenario_report/migrations",
            "tachyon_apps_scenario_report",
        ),
        ("apps/library-api/migrations", "library"),
    ];

    // Run migrations concurrently - each targets a different database
    // so there are no conflicts.
    let migrate_futures: Vec<_> = migrations
        .iter()
        .map(|(dir, db_name)| {
            let url = format!("{}/{}", trimmed_base, db_name);
            let ws = workspace_root.to_path_buf();
            let d = dir.to_string();
            async move { run_sqlx_migrate(&ws, &d, &url).await }
        })
        .collect();
    let results = futures::future::join_all(migrate_futures).await;
    for result in results {
        result?;
    }

    let seeds_path = workspace_root.join("scripts/seeds/n1-seed");
    if !seeds_path.exists() {
        return Err(anyhow!(
            "シードディレクトリが見つかりません: {}",
            seeds_path.display()
        ));
    }

    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("yaml-seeder")
        .arg("--")
        .arg("apply")
        .arg(seeds_path.as_os_str())
        .current_dir(workspace_root)
        .env("RUST_LOG", "error")
        .env("DATABASE_URL", base_database_url)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .await
        .context("yaml-seeder apply の実行に失敗しました")?;

    if !status.success() {
        return Err(anyhow!(
            "yaml-seeder apply が異常終了しました: status={}",
            status
        ));
    }

    Ok(())
}

async fn run_sqlx_migrate(
    workspace_root: &Path,
    migration_dir: &str,
    database_url: &str,
) -> Result<()> {
    let _ = Command::new("sqlx")
        .arg("database")
        .arg("create")
        .arg("--database-url")
        .arg(database_url)
        .current_dir(workspace_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    let status = Command::new("sqlx")
        .arg("migrate")
        .arg("run")
        .arg("--source")
        .arg(migration_dir)
        .arg("--database-url")
        .arg(database_url)
        .current_dir(workspace_root)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .await
        .with_context(|| {
            format!(
                "failed to run sqlx migrate for {} (database_url={})",
                migration_dir, database_url
            )
        })?;

    if !status.success() {
        warn!(
            "sqlx migrate run exited with non-zero status {} for {} (database_url={}), continue",
            status,
            migration_dir,
            database_url
        );
    }

    Ok(())
}

fn spawn_log_forwarder<R>(reader: R, label: &'static str) -> JoinHandle<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            info!("[library-api::{label}] {line}");
        }
    })
}

fn print_step_result(index: usize, step: &StepResult) {
    let icon = if step.success { "✅" } else { "❌" };
    info!(
        "  {} Step {} - {} ({}ms)",
        icon,
        index + 1,
        step.name,
        step.duration_ms
    );

    if step.success {
        return;
    }

    if let Some(error) = &step.error {
        error!("      Error: {}", error);
    }

    info!(
        "      Request: {} {}",
        step.request.method, step.request.url
    );
    if let Some(body) = &step.request.body {
        info!("      Request Body: {}", truncate_for_display(body, 400));
    }

    if let Some(response) = &step.response {
        info!("      Response Status: {}", response.status);
        if let Some(body) = &response.body {
            info!(
                "      Response Body: {}",
                truncate_for_display(body, 400)
            );
        }
    }
}

fn truncate_for_display(text: &str, limit: usize) -> String {
    let sanitized = text.replace('\n', "\\n");
    let mut chars = sanitized.chars();
    let truncated: String = chars.by_ref().take(limit).collect();

    if chars.next().is_some() {
        format!("{}...(省略)", truncated)
    } else {
        truncated
    }
}

/// Generic health check for any spawned server process.
async fn wait_for_health_generic(
    child: &mut Child,
    health_url: &str,
    timeout_dur: Duration,
    interval: Duration,
    label: &str,
) -> Result<()> {
    let client = reqwest::Client::new();

    let health_future = async {
        loop {
            if let Some(status) = child.try_wait().context(format!(
                "{}プロセスの状態確認に失敗しました",
                label
            ))? {
                return Err(anyhow!(
                    "{}がヘルスチェック完了前に終了しました (status: {status})",
                    label
                ));
            }

            match client.get(health_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("ヘルスチェック成功 [{}]: {}", label, health_url);
                    break;
                }
                Ok(resp) => {
                    warn!(
                        "ヘルスチェック失敗 [{}] (status: {}) 再試行",
                        label,
                        resp.status()
                    );
                }
                Err(err) => {
                    warn!(
                        "ヘルスチェック要求に失敗しました [{}]: {err}",
                        label
                    );
                }
            }

            sleep(interval).await;
        }
        Ok(())
    };

    timeout(timeout_dur, health_future).await.context(format!(
        "ヘルスチェックがタイムアウトしました [{}]",
        label
    ))??;

    Ok(())
}

struct TestServerGuard {
    library_child: Option<Child>,
    tachyon_child: Option<Child>,
    log_handles: Vec<JoinHandle<()>>,
}

impl TestServerGuard {
    fn new(
        library_child: Child,
        tachyon_child: Option<Child>,
        log_handles: Vec<JoinHandle<()>>,
    ) -> Self {
        Self {
            library_child: Some(library_child),
            tachyon_child,
            log_handles,
        }
    }

    async fn shutdown(&mut self) {
        for handle in self.log_handles.drain(..) {
            handle.abort();
        }

        // Shut down library-api first
        if let Some(mut child) = self.library_child.take() {
            if let Err(e) = child.start_kill() {
                warn!("library-apiの停止に失敗しました: {e}");
            }
            let _ = child.wait().await;
        }

        // Then tachyon-api
        if let Some(mut child) = self.tachyon_child.take() {
            if let Err(e) = child.start_kill() {
                warn!("tachyon-apiの停止に失敗しました: {e}");
            }
            let _ = child.wait().await;
        }
    }
}

#[derive(Debug, Deserialize)]
struct RuntimeConfig {
    server: ServerConfig,
    dependencies: DependencyConfig,
    logging: LoggingConfig,
    scenario: ScenarioConfig,
    #[serde(default)]
    report: ReportConfig,
    /// Optional tachyon-api configuration.
    /// When present, the test runner spawns tachyon-api before library-api
    /// so that SdkAuthApp REST calls work.
    tachyon_api: Option<TachyonApiConfig>,
}

impl RuntimeConfig {
    fn load() -> Result<Self> {
        // Check TEST_CONFIG_PATH environment variable first
        if let Ok(env_path) = std::env::var("TEST_CONFIG_PATH") {
            let path = Path::new(&env_path);
            if path.exists() {
                let content =
                    std::fs::read_to_string(path).with_context(|| {
                        format!(
                            "Failed to read config file: {}",
                            path.display()
                        )
                    })?;
                let mut config: RuntimeConfig =
                    serde_yaml::from_str(&content).with_context(|| {
                        format!(
                            "Failed to parse config file: {}",
                            path.display()
                        )
                    })?;
                config.report = config.report.with_env_overrides();
                info!(
                    "設定ファイルを読み込みました (TEST_CONFIG_PATH): {}",
                    path.display()
                );
                return Ok(config);
            } else {
                warn!(
                    "TEST_CONFIG_PATH で指定されたファイルが見つかりません: {}",
                    env_path
                );
            }
        }

        for candidate in CONFIG_CANDIDATES {
            let path = Path::new(candidate);
            if path.exists() {
                let content =
                    std::fs::read_to_string(path).with_context(|| {
                        format!(
                            "Failed to read config file: {}",
                            path.display()
                        )
                    })?;
                let mut config: RuntimeConfig =
                    serde_yaml::from_str(&content).with_context(|| {
                        format!(
                            "Failed to parse config file: {}",
                            path.display()
                        )
                    })?;
                config.report = config.report.with_env_overrides();
                info!("設定ファイルを読み込みました: {}", path.display());
                return Ok(config);
            }
        }

        Err(anyhow!(
            "設定ファイルが見つかりません。以下のいずれかを配置してください: {:?}",
            CONFIG_CANDIDATES
        ))
    }

    fn resolve_base_url(&self) -> String {
        self.scenario
            .default_base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}", self.server.address))
    }
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    binary: String,
    address: String,
    health_check: HealthCheckConfig,
}

impl ServerConfig {
    fn binary_path(&self) -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // workspace root = apps/library-api/.. (tachyon-apps)
        manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|ws| ws.join("target/debug").join(&self.binary))
            .unwrap_or_else(|| PathBuf::from(&self.binary))
    }

    fn health_url(&self, base_url: &str) -> String {
        format!("{}{}", base_url, self.health_check.path)
    }
}

#[derive(Debug, Deserialize)]
struct HealthCheckConfig {
    path: String,
    timeout_seconds: u64,
    interval_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct DependencyConfig {
    database_url: Option<String>,
    database_root_url: Option<String>,
    auth_database_url: Option<String>,
    database_manager_url: Option<String>,
    root_id: Option<String>,
    #[serde(default)]
    additional_env: HashMap<String, String>,
}

impl DependencyConfig {
    fn env_vars(&self) -> HashMap<String, String> {
        let mut envs = self.additional_env.clone();

        if let Some(db) = &self.database_url {
            envs.insert("LIBRARY_DATABASE_URL".to_string(), db.clone());
            envs.insert("DATABASE_URL".to_string(), db.clone());
        }
        if let Some(auth_db) = &self.auth_database_url {
            envs.insert("AUTH_DATABASE_URL".to_string(), auth_db.clone());
        }
        if let Some(dm_db) = &self.database_manager_url {
            envs.insert(
                "DB_MANAGER_DATABASE_URL".to_string(),
                dm_db.clone(),
            );
        }

        if let Some(root_id) = &self.root_id {
            envs.insert("LIBRARY_TENANT_ID".to_string(), root_id.clone());
            // Library API expects ROOT_ID for auth bootstrap. Align both to the same value.
            envs.insert("ROOT_ID".to_string(), root_id.clone());
        }

        envs
    }
}

#[derive(Debug, Deserialize)]
struct LoggingConfig {
    forward_stdout: bool,
    forward_stderr: bool,
}

#[derive(Debug, Deserialize)]
struct ScenarioConfig {
    default_base_url: Option<String>,
    #[serde(rename = "allow_cli_override")]
    _allow_cli_override: bool,
    #[serde(default)]
    include: Vec<String>,
}

/// Configuration for the tachyon-api dependency.
/// When specified, the test runner spawns tachyon-api before library-api.
#[derive(Debug, Deserialize)]
struct TachyonApiConfig {
    binary: String,
    address: String,
    health_path: String,
    health_timeout_seconds: u64,
    health_interval_seconds: u64,
    #[serde(default)]
    env: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReportConfig {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_api_key")]
    api_key: String,
    #[serde(default)]
    api_url: Option<String>,
    #[serde(default)]
    operator_id: Option<String>,
}

impl ReportConfig {
    fn with_env_overrides(mut self) -> Self {
        if let Ok(url) = std::env::var("TACHYON_OPS_API_URL") {
            if !url.is_empty() {
                self.api_url = Some(url);
            }
        }
        if let Ok(key) = std::env::var("TACHYON_OPS_API_KEY") {
            if !key.is_empty() {
                self.api_key = key;
            }
        }
        if let Ok(oid) = std::env::var("TACHYON_OPS_OPERATOR_ID") {
            if !oid.is_empty() {
                self.operator_id = Some(oid);
            }
        }
        self
    }
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_key: default_api_key(),
            api_url: None,
            operator_id: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_api_key() -> String {
    "dummy-token".to_string()
}

async fn submit_report(
    report_config: &ReportConfig,
    base_url: &str,
    results: Vec<TestResult>,
    total_duration_ms: u64,
) {
    let api_url = report_config
        .api_url
        .clone()
        .unwrap_or_else(|| base_url.to_string());
    let api_key = report_config.api_key.clone();
    let operator_id = report_config.operator_id.clone();

    let report = TestRunReport {
        scenarios: results,
        total_duration_ms,
        timestamp: Utc::now().to_rfc3339(),
        ci: detect_ci_metadata(),
    };

    info!("テスト結果を Ops API に送信します ({})...", api_url);

    let mut client = TachyonOpsClient::new(api_url, api_key);
    if let Some(oid) = operator_id {
        client = client.with_operator_id(oid);
    }

    match client.submit_report(&report).await {
        Ok(resp) => {
            info!("レポート送信完了 (run_id: {})", resp.run_id);
        }
        Err(e) => {
            warn!("レポート送信に失敗しました: {}", e);
        }
    }
}

fn detect_ci_metadata() -> Option<muon::CiMetadata> {
    if std::env::var("GITHUB_ACTIONS").is_ok() {
        let repository =
            std::env::var("GITHUB_REPOSITORY").unwrap_or_default();
        let branch = std::env::var("GITHUB_REF_NAME")
            .or_else(|_| std::env::var("GITHUB_HEAD_REF"))
            .unwrap_or_default();
        let commit_sha = std::env::var("GITHUB_SHA").unwrap_or_default();
        let pr_number = std::env::var("PR_NUMBER")
            .or_else(|_| {
                std::env::var("GITHUB_REF")
                    .map(|r| r.split('/').nth(2).unwrap_or("").to_string())
            })
            .ok()
            .and_then(|n| n.parse::<u64>().ok());
        let run_id = std::env::var("GITHUB_RUN_ID").ok();
        let run_url = run_id.as_ref().map(|id| {
            format!("https://github.com/{}/actions/runs/{}", repository, id)
        });

        return Some(muon::CiMetadata {
            provider: "github".to_string(),
            repository,
            branch,
            commit_sha,
            pr_number,
            run_id,
            run_url,
        });
    }

    // Local development
    let branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let commit_sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if branch.is_empty() && commit_sha.is_empty() {
        return None;
    }

    Some(muon::CiMetadata {
        provider: "local".to_string(),
        repository: "quantum-box/tachyon-apps".to_string(),
        branch,
        commit_sha,
        pr_number: None,
        run_id: None,
        run_url: None,
    })
}
