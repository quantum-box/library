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
        let pool = MySqlPoolOptions::new()
            .max_connections(8)
            .acquire_timeout(std::time::Duration::from_secs(60))
            .connect(&dsn.to_string())
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Cannot connect to the database. Please check your configuration. :{:?}, {}",
                    e,
                    dsn.to_string(),
                )
            });
        Arc::new(Self(Arc::new(pool)))
    }

    pub fn pool(&self) -> Arc<Pool<MySql>> {
        self.0.clone()
    }
}
