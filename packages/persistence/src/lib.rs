pub mod tests;

pub mod minio;
pub use minio::*;

mod s3;
pub use s3::*;

mod cloudflare_r2;
pub use cloudflare_r2::*;

pub mod test_helper;

use errors::Result;
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use url::Url;
use value_object::InMemoryFile;

#[async_trait::async_trait]
pub trait Storage: Debug + Send + Sync {
    async fn put_object(
        &self,
        bucket_name: &str,
        object_path: &str,
        file: &InMemoryFile,
    ) -> Result<Url>;

    async fn presigned_get(
        &self,
        bucket_name: &str,
        object_path: &str,
        expires: u32,
    ) -> Result<Url>;
}

#[async_trait::async_trait]
pub trait StorageAdminAccess: Debug + Send + Sync {
    async fn create_bucket(&self, bucket_name: &str) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct Db(pub(crate) Arc<Pool<MySql>>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbPoolConfig {
    max_connections: u32,
    acquire_timeout: Duration,
}

impl Default for DbPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 8,
            acquire_timeout: Duration::from_secs(60),
        }
    }
}

impl DbPoolConfig {
    pub const MAX_CONNECTIONS_ENV: &'static str = "DB_POOL_MAX_CONNECTIONS";
    pub const ACQUIRE_TIMEOUT_SECS_ENV: &'static str =
        "DB_POOL_ACQUIRE_TIMEOUT_SECS";

    pub fn from_env() -> Self {
        Self::from_lookup(Self::default(), |key| std::env::var(key).ok())
    }

    fn from_lookup(
        default: Self,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Self {
        let max_connections = lookup(Self::MAX_CONNECTIONS_ENV)
            .and_then(|value| {
                parse_non_zero_u32(Self::MAX_CONNECTIONS_ENV, &value)
            })
            .unwrap_or(default.max_connections);
        let acquire_timeout = lookup(Self::ACQUIRE_TIMEOUT_SECS_ENV)
            .and_then(|value| {
                parse_non_zero_u64(Self::ACQUIRE_TIMEOUT_SECS_ENV, &value)
            })
            .map(Duration::from_secs)
            .unwrap_or(default.acquire_timeout);

        Self {
            max_connections,
            acquire_timeout,
        }
    }

    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }

    pub fn acquire_timeout(&self) -> Duration {
        self.acquire_timeout
    }
}

fn parse_non_zero_u32(env_name: &str, value: &str) -> Option<u32> {
    match value.parse::<u32>() {
        Ok(parsed) if parsed > 0 => Some(parsed),
        _ => {
            tracing::warn!(
                env_name,
                value,
                "invalid DB pool max connections; falling back to default"
            );
            None
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
                "invalid DB pool acquire timeout; falling back to default"
            );
            None
        }
    }
}

impl Db {
    // TODO: add English comment
    pub async fn from_env() -> Arc<Self> {
        let dsn = if std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "dev".to_string())
            .to_lowercase()
            == "production"
        {
            std::env::var("DATABASE_URL").expect("DATABASE_URL is not set")
        } else {
            std::env::var("DEV_DATABASE_URL")
                .expect("DEV_DATABASE_URL is not set")
        };
        Self::new(dsn).await
    }

    pub async fn new(dsn: impl ToString) -> Arc<Self> {
        Self::new_with_pool_config(dsn, DbPoolConfig::from_env()).await
    }

    pub async fn new_with_pool_config(
        dsn: impl ToString,
        pool_config: DbPoolConfig,
    ) -> Arc<Self> {
        tracing::info!(
            max_connections = pool_config.max_connections(),
            acquire_timeout_secs = pool_config.acquire_timeout().as_secs(),
            "creating MySQL connection pool"
        );
        let dsn = dsn.to_string();
        let pool = MySqlPoolOptions::new()
            .max_connections(pool_config.max_connections())
            .acquire_timeout(pool_config.acquire_timeout())
            .connect(&dsn)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Cannot connect to the database. Please check your configuration: {e:?}"
                )
            });
        Arc::new(Self(Arc::new(pool)))
    }

    /// Build a pool that opens its first connection when a query needs
    /// one.
    ///
    /// [`Self::new`] connects before it returns. On Lambda that puts a
    /// TLS handshake to the database in front of every cold start, for
    /// each pool the process builds — and the request that follows may
    /// not touch that database at all. Connecting lazily moves the cost
    /// to the query that actually needs the connection.
    ///
    /// A DSN this cannot even parse is still a configuration error
    /// worth failing on immediately; that check costs no I/O. What
    /// moves is only the reachability of the server, which now surfaces
    /// as a failed query rather than a process that refuses to start.
    ///
    /// Must be called from inside a Tokio runtime: the pool starts its
    /// own maintenance task even when it holds no connection yet.
    pub fn new_lazy(dsn: impl ToString) -> Arc<Self> {
        Self::new_lazy_with_pool_config(dsn, DbPoolConfig::from_env())
    }

    pub fn new_lazy_with_pool_config(
        dsn: impl ToString,
        pool_config: DbPoolConfig,
    ) -> Arc<Self> {
        tracing::info!(
            max_connections = pool_config.max_connections(),
            acquire_timeout_secs = pool_config.acquire_timeout().as_secs(),
            "creating MySQL connection pool (lazy)"
        );
        let dsn = dsn.to_string();
        let pool = MySqlPoolOptions::new()
            .max_connections(pool_config.max_connections())
            .acquire_timeout(pool_config.acquire_timeout())
            .connect_lazy(&dsn)
            .unwrap_or_else(|e| {
                panic!(
                    "Cannot use the database URL. Please check your configuration: {e:?}"
                )
            });
        Arc::new(Self(Arc::new(pool)))
    }

    pub fn pool(&self) -> Arc<Pool<MySql>> {
        self.0.clone()
    }
}

#[cfg(test)]
mod db_pool_config_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn uses_default_pool_config_when_env_is_absent() {
        let config =
            DbPoolConfig::from_lookup(DbPoolConfig::default(), |_| None);

        assert_eq!(config.max_connections(), 8);
        assert_eq!(config.acquire_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn reads_pool_config_from_env_values() {
        let vars = HashMap::from([
            (DbPoolConfig::MAX_CONNECTIONS_ENV, "16".to_string()),
            (DbPoolConfig::ACQUIRE_TIMEOUT_SECS_ENV, "10".to_string()),
        ]);
        let config =
            DbPoolConfig::from_lookup(DbPoolConfig::default(), |key| {
                vars.get(key).cloned()
            });

        assert_eq!(config.max_connections(), 16);
        assert_eq!(config.acquire_timeout(), Duration::from_secs(10));
    }

    #[test]
    fn ignores_invalid_pool_config_values() {
        let vars = HashMap::from([
            (DbPoolConfig::MAX_CONNECTIONS_ENV, "0".to_string()),
            (
                DbPoolConfig::ACQUIRE_TIMEOUT_SECS_ENV,
                "invalid".to_string(),
            ),
        ]);
        let config =
            DbPoolConfig::from_lookup(DbPoolConfig::default(), |key| {
                vars.get(key).cloned()
            });

        assert_eq!(config, DbPoolConfig::default());
    }
}
