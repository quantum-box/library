use crate::{Storage, StorageAdminAccess};
use errors::{Error, Result};
use s3::{creds::Credentials, Bucket, Region};
use std::fmt::Debug;
use std::sync::Arc;
use url::Url;
use value_object::InMemoryFile;

#[derive(Debug, Clone)]
pub struct S3Driver {
    region: Region,
    credentials: Credentials,
}

impl S3Driver {
    /// TODO: add English documentation
    ///
    /// Environment Variables:
    /// TODO: add English documentation
    /// TODO: add English documentation
    /// TODO: add English documentation
    pub fn new() -> Result<Arc<Self>> {
        let region = Region::ApNortheast1;
        let credentials = Credentials::default().map_err(|e| {
            Error::internal_server_error(format!(
                "failed to get credentials: {e}"
            ))
        })?;
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
        .map_err(|e| Error::internal_server_error(e.to_string()))?;
        Ok(bucket)
    }
}

#[async_trait::async_trait]
impl Storage for S3Driver {
    async fn put_object(
        &self,
        bucket_name: &str,
        object_path: &str,
        file: &InMemoryFile,
    ) -> Result<Url> {
        let content = file.content().to_vec();
        let bucket = self.get_bucket(bucket_name).await?;
        bucket
            .put_object_with_content_type(
                object_path,
                content.as_slice(),
                file.content_type().as_str(),
            )
            .await
            .map_err(|e| Error::internal_server_error(e.to_string()))?;
        let url = format!(
            "https://{}/{}/{}",
            self.region.endpoint(),
            bucket_name,
            object_path
        );
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
impl StorageAdminAccess for S3Driver {
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
