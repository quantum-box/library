use crate::{Storage, StorageAdminAccess};
use errors::{Error, Result};
use s3::BucketConfiguration;
use s3::{creds::Credentials, Bucket, Region};
use std::fmt::Debug;
use std::sync::Arc;
use url::Url;
use value_object::InMemoryFile;
#[derive(Debug, Clone)]
pub struct CloudflareR2Driver {
    region: Region,
    credentials: Credentials,
}

impl CloudflareR2Driver {
    pub fn new(
        account_id: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Arc<Self>> {
        let region = Region::R2 {
            account_id: account_id.to_owned(),
        };
        let credentials = Credentials::new(
            Some(access_key),
            Some(secret_key),
            None,
            None,
            None,
        )
        .map_err(|e| Error::internal_server_error(e.to_string()))?;
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
}

#[async_trait::async_trait]
impl Storage for CloudflareR2Driver {
    async fn put_object(
        &self,
        bucket_name: &str,
        object_path: &str,
        file: &InMemoryFile,
    ) -> Result<Url> {
        let content = file.content().to_vec();
        let bucket = self.get_bucket(bucket_name).await?;
        bucket
            .put_object(object_path, content.as_slice())
            .await
            .map_err(|e| Error::internal_server_error(e.to_string()))?;
        let response = bucket
            .presign_get(object_path, 300, None)
            .await
            .map_err(|e| Error::internal_server_error(e.to_string()))?;
        let url = Url::parse(&response)
            .map_err(|e| Error::internal_server_error(e.to_string()))?;
        Ok(url)
    }

    async fn presigned_get(
        &self,
        bucket_name: &str,
        object_path: &str,
        expires: u32,
    ) -> Result<Url> {
        let bucket = self.get_bucket(bucket_name).await?;
        let response = bucket
            .presign_get(object_path, expires, None)
            .await
            .map_err(|e| Error::internal_server_error(e.to_string()))?;
        let url = Url::parse(&response)
            .map_err(|e| Error::internal_server_error(e.to_string()))?;
        Ok(url)
    }
}

#[async_trait::async_trait]
impl StorageAdminAccess for CloudflareR2Driver {
    async fn create_bucket(&self, bucket_name: &str) -> Result<()> {
        Bucket::create(
            bucket_name,
            self.region.clone(),
            self.credentials.clone(),
            BucketConfiguration::default(),
        )
        .await
        .map_err(|e| Error::internal_server_error(e.to_string()))?;
        Ok(())
    }
}
