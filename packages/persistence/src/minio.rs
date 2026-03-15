use crate::{Storage, StorageAdminAccess};
use errors::{Error, Result};
use s3::BucketConfiguration;
use s3::{creds::Credentials, Bucket, Region};
use std::fmt::Debug;
use std::sync::Arc;
use url::Url;
use value_object::InMemoryFile;

#[derive(Debug, Clone, Default)]
pub struct MinioConfiguration {
    pub storage_url: String,
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone)]
pub struct MinioDriver {
    region: Region,
    credentials: Credentials,
}

impl MinioDriver {
    /// TODO: add English documentation
    ///
    /// Environment Variables:
    /// TODO: add English documentation
    /// TODO: add English documentation
    /// TODO: add English documentation
    ///
    /// default: http://localhost:9000
    pub fn new(config: &MinioConfiguration) -> Result<Arc<Self>> {
        let region = Region::Custom {
            region: String::from("eu-central-1"),
            endpoint: config.storage_url.clone(),
        };
        let credentials = Credentials {
            access_key: Some(config.access_key.clone()),
            secret_key: Some(config.secret_key.clone()),
            security_token: None,
            session_token: None,
            expiration: None,
        };
        Ok(Arc::new(Self {
            region,
            credentials,
        }))
    }

    async fn get_bucket(&self, bucket_name: &str) -> Result<Bucket> {
        let bucket = Bucket::new(
            bucket_name,
            self.region.clone(),
            self.credentials.clone(),
        )
        .map_err(|e| Error::internal_server_error(e.to_string()))?
        .with_path_style();
        Ok(bucket)
    }

    pub async fn create_bucket(&self, bucket_name: &str) -> Result<()> {
        let bucket = Bucket::new(
            bucket_name,
            self.region.clone(),
            self.credentials.clone(),
        )
        .map_err(|e| Error::internal_server_error(e.to_string()))?
        .with_path_style();
        if !bucket
            .exists()
            .await
            .map_err(|e| Error::internal_server_error(e.to_string()))?
        {
            Bucket::create_with_path_style(
                bucket_name,
                self.region.clone(),
                self.credentials.clone(),
                BucketConfiguration::default(),
            )
            .await
            .map_err(|e| Error::internal_server_error(e.to_string()))?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Storage for MinioDriver {
    async fn put_object(
        &self,
        bucket_name: &str,
        object_path: &str,
        file: &InMemoryFile,
    ) -> Result<Url> {
        let content = file.content().to_vec();
        let bucket = self.get_bucket(bucket_name).await?;
        let response = bucket
            .put_object_with_content_type(
                object_path,
                content.as_slice(),
                file.content_type().as_str(),
            )
            .await
            .map_err(|e| Error::internal_server_error(e.to_string()))?;
        let url = format!(
            "{}/{}/{}",
            self.region.endpoint(),
            bucket_name,
            object_path
        );
        dbg!(response);
        Ok(Url::parse(&url)
            .map_err(|e| Error::internal_server_error(e.to_string()))?)
    }

    async fn presigned_get(
        &self,
        bucket_name: &str,
        object_name: &str,
        expires: u32,
    ) -> Result<Url> {
        let bucket = self.get_bucket(bucket_name).await?;
        let response = bucket
            .presign_get(object_name, expires, None)
            .await
            .map_err(|e| Error::internal_server_error(e.to_string()))?;
        let url = Url::parse(&response)
            .map_err(|e| Error::internal_server_error(e.to_string()))?;
        Ok(url)
    }
}

#[async_trait::async_trait]
impl StorageAdminAccess for MinioDriver {
    async fn create_bucket(&self, bucket_name: &str) -> Result<()> {
        Bucket::create(
            bucket_name,
            self.region.clone(),
            self.credentials.clone(),
            s3::BucketConfiguration::default(),
        )
        .await
        .map_err(|e| Error::internal_server_error(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use s3::BucketConfiguration;
    use std::env;
    use value_object::Ulid;

    use super::*;

    // TODO: add English comment
    #[tokio::test]
    #[ignore]
    async fn test_create_bucket() -> anyhow::Result<()> {
        dotenvy::dotenv().ok();

        let user = env::var("MINIO_ROOT_USER")
            .expect("MINIO_ROOT_USER is not set");
        let secret = env::var("MINIO_ROOT_PASSWORD")
            .expect("MINIO_ROOT_PASSWORD is not set");

        let bucket_name = "test-rust-s3";
        let region = Region::Custom {
            region: "eu-central-1".to_owned(),
            endpoint: "http://localhost:9000".to_owned(),
        };
        let credentials = Credentials {
            access_key: Some(user),
            secret_key: Some(secret),
            security_token: None,
            session_token: None,
            expiration: None,
        };

        let bucket =
            Bucket::new(bucket_name, region.clone(), credentials.clone())?
                .with_path_style();

        if !bucket.exists().await? {
            Bucket::create_with_path_style(
                bucket_name,
                region,
                credentials,
                BucketConfiguration::default(),
            )
            .await?;
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_put_object() -> anyhow::Result<()> {
        dotenvy::dotenv().ok();

        let user = env::var("MINIO_ROOT_USER")
            .expect("MINIO_ROOT_USER is not set");
        let secret = env::var("MINIO_ROOT_PASSWORD")
            .expect("MINIO_ROOT_PASSWORD is not set");

        let config = MinioConfiguration {
            storage_url: "http://localhost:9000".to_string(),
            access_key: user,
            secret_key: secret,
        };
        let storage = MinioDriver::new(&config)?;

        let bucket_name = Ulid::new().to_string().to_lowercase();

        let bucket = storage.get_bucket(&bucket_name).await?;

        if !bucket.exists().await? {
            storage.create_bucket(&bucket_name).await?;
        }
        Ok(())
    }
}
