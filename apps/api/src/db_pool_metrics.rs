use std::sync::Arc;
use std::time::{Duration, Instant};

use sqlx::{MySql, Pool};
use tokio::task::JoinHandle;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbPoolAcquireMetricsConfig {
    enabled: bool,
    interval: Duration,
    warn_threshold: Duration,
}

impl Default for DbPoolAcquireMetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(60),
            warn_threshold: Duration::from_millis(250),
        }
    }
}

impl DbPoolAcquireMetricsConfig {
    pub const ENABLED_ENV: &'static str = "DB_POOL_ACQUIRE_METRICS_ENABLED";
    pub const INTERVAL_SECS_ENV: &'static str =
        "DB_POOL_ACQUIRE_METRICS_INTERVAL_SECS";
    pub const WARN_MS_ENV: &'static str = "DB_POOL_ACQUIRE_WARN_MS";

    pub fn from_env() -> Self {
        Self::from_lookup(Self::default(), |key| std::env::var(key).ok())
    }

    fn from_lookup(
        default: Self,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Self {
        let enabled = lookup(Self::ENABLED_ENV)
            .map(|value| parse_bool(Self::ENABLED_ENV, &value))
            .unwrap_or(default.enabled);
        let interval = lookup(Self::INTERVAL_SECS_ENV)
            .and_then(|value| {
                parse_non_zero_u64(Self::INTERVAL_SECS_ENV, &value)
            })
            .map(Duration::from_secs)
            .unwrap_or(default.interval);
        let warn_threshold = lookup(Self::WARN_MS_ENV)
            .and_then(|value| parse_non_zero_u64(Self::WARN_MS_ENV, &value))
            .map(Duration::from_millis)
            .unwrap_or(default.warn_threshold);

        Self {
            enabled,
            interval,
            warn_threshold,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn warn_threshold(&self) -> Duration {
        self.warn_threshold
    }
}

pub fn start_default_pool_acquire_metrics(
    pools: impl IntoIterator<Item = (&'static str, Arc<Pool<MySql>>)>,
) -> Vec<JoinHandle<()>> {
    let config = DbPoolAcquireMetricsConfig::from_env();
    pools
        .into_iter()
        .filter_map(|(pool_name, pool)| {
            start_pool_acquire_metrics(pool_name, pool, config.clone())
        })
        .collect()
}

fn start_pool_acquire_metrics(
    pool_name: &'static str,
    pool: Arc<Pool<MySql>>,
    config: DbPoolAcquireMetricsConfig,
) -> Option<JoinHandle<()>> {
    if !config.enabled() {
        tracing::info!(pool_name, "DB pool acquire metrics disabled");
        return None;
    }

    Some(tokio::spawn(async move {
        loop {
            tokio::time::sleep(config.interval()).await;
            probe_pool(pool_name, &pool, &config).await;
        }
    }))
}

async fn probe_pool(
    pool_name: &'static str,
    pool: &Pool<MySql>,
    config: &DbPoolAcquireMetricsConfig,
) {
    let started_at = Instant::now();
    match pool.acquire().await {
        Ok(conn) => {
            drop(conn);
            let wait = started_at.elapsed();
            log_pool_acquire_wait(
                pool_name,
                wait,
                pool.size(),
                pool.num_idle(),
                config.warn_threshold(),
            );
        }
        Err(error) if is_pool_timeout(&error) => {
            let wait = started_at.elapsed();
            tracing::error!(
                metric = "library_api_db_pool_acquire_timeout",
                pool_name,
                wait_ms = duration_ms(wait),
                pool_size = pool.size(),
                pool_idle = pool.num_idle(),
                error = %error,
                "library-api DB pool acquire timeout"
            );
        }
        Err(error) => {
            let wait = started_at.elapsed();
            tracing::warn!(
                metric = "library_api_db_pool_acquire_error",
                pool_name,
                wait_ms = duration_ms(wait),
                pool_size = pool.size(),
                pool_idle = pool.num_idle(),
                error = %error,
                "library-api DB pool acquire probe failed"
            );
        }
    }
}

fn log_pool_acquire_wait(
    pool_name: &'static str,
    wait: Duration,
    pool_size: u32,
    pool_idle: usize,
    warn_threshold: Duration,
) {
    if wait >= warn_threshold {
        tracing::warn!(
            metric = "library_api_db_pool_acquire_wait_ms",
            pool_name,
            wait_ms = duration_ms(wait),
            pool_size,
            pool_idle,
            "library-api DB pool acquire probe"
        );
    } else {
        tracing::info!(
            metric = "library_api_db_pool_acquire_wait_ms",
            pool_name,
            wait_ms = duration_ms(wait),
            pool_size,
            pool_idle,
            "library-api DB pool acquire probe"
        );
    }
}

fn is_pool_timeout(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::PoolTimedOut)
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn parse_bool(env_name: &str, value: &str) -> bool {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => {
            tracing::warn!(
                env_name,
                value,
                "invalid DB pool acquire metrics bool; falling back to enabled"
            );
            true
        }
    }
}

fn parse_non_zero_u64(env_name: &str, value: &str) -> Option<u64> {
    match value.parse::<u64>() {
        Ok(parsed) if parsed > 0 => Some(parsed),
        _ => {
            tracing::warn!(
                env_name,
                value,
                "invalid DB pool acquire metrics value; falling back to default"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn config_uses_defaults_when_env_absent() {
        let config = DbPoolAcquireMetricsConfig::from_lookup(
            DbPoolAcquireMetricsConfig::default(),
            |_| None,
        );

        assert!(config.enabled());
        assert_eq!(config.interval(), Duration::from_secs(60));
        assert_eq!(config.warn_threshold(), Duration::from_millis(250));
    }

    #[test]
    fn config_reads_env_values() {
        let vars = HashMap::from([
            (DbPoolAcquireMetricsConfig::ENABLED_ENV, "false".to_string()),
            (
                DbPoolAcquireMetricsConfig::INTERVAL_SECS_ENV,
                "15".to_string(),
            ),
            (DbPoolAcquireMetricsConfig::WARN_MS_ENV, "500".to_string()),
        ]);
        let config = DbPoolAcquireMetricsConfig::from_lookup(
            DbPoolAcquireMetricsConfig::default(),
            |key| vars.get(key).cloned(),
        );

        assert!(!config.enabled());
        assert_eq!(config.interval(), Duration::from_secs(15));
        assert_eq!(config.warn_threshold(), Duration::from_millis(500));
    }

    #[test]
    fn config_falls_back_for_invalid_values() {
        let vars = HashMap::from([
            (DbPoolAcquireMetricsConfig::ENABLED_ENV, "maybe".to_string()),
            (
                DbPoolAcquireMetricsConfig::INTERVAL_SECS_ENV,
                "0".to_string(),
            ),
            (
                DbPoolAcquireMetricsConfig::WARN_MS_ENV,
                "invalid".to_string(),
            ),
        ]);
        let config = DbPoolAcquireMetricsConfig::from_lookup(
            DbPoolAcquireMetricsConfig::default(),
            |key| vars.get(key).cloned(),
        );

        assert_eq!(config, DbPoolAcquireMetricsConfig::default());
    }

    #[test]
    fn detects_sqlx_pool_timeout() {
        assert!(is_pool_timeout(&sqlx::Error::PoolTimedOut));
        assert!(!is_pool_timeout(&sqlx::Error::PoolClosed));
    }
}
