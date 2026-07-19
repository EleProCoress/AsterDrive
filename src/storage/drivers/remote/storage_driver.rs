use async_trait::async_trait;
use tokio::io::AsyncRead;

use crate::errors::Result;
use crate::storage::error::{StorageErrorKind, storage_driver_error};
use crate::storage::traits::driver::{BlobMetadata, StorageDriver};
use crate::storage::traits::extensions::StorageCapacityInfo;

use super::RemoteDriver;

#[async_trait]
impl StorageDriver for RemoteDriver {
    async fn put(&self, path: &str, data: &[u8]) -> Result<String> {
        self.client.put_bytes(&self.object_key(path), data).await?;
        Ok(path.to_string())
    }

    async fn get(&self, path: &str) -> Result<Vec<u8>> {
        self.client.get_bytes(&self.object_key(path)).await
    }

    async fn get_stream(&self, path: &str) -> Result<Box<dyn AsyncRead + Unpin + Send>> {
        self.client
            .get_stream(&self.object_key(path), None, None)
            .await
    }

    async fn get_range(
        &self,
        path: &str,
        offset: u64,
        length: Option<u64>,
    ) -> Result<Box<dyn AsyncRead + Unpin + Send>> {
        self.client
            .get_stream(&self.object_key(path), Some(offset), length)
            .await
    }

    fn supports_efficient_range(&self) -> bool {
        true
    }

    async fn delete(&self, path: &str) -> Result<()> {
        self.client.delete(&self.object_key(path)).await
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        self.client.exists(&self.object_key(path)).await
    }

    async fn metadata(&self, path: &str) -> Result<BlobMetadata> {
        self.client.metadata(&self.object_key(path)).await
    }

    async fn capacity_info(&self) -> Result<StorageCapacityInfo> {
        if !self.supports_capacity {
            return Err(storage_driver_error(
                StorageErrorKind::Unsupported,
                "remote storage node does not support capacity observability",
            ));
        }
        self.client.capacity_info().await
    }

    fn extensions(&self) -> crate::storage::traits::StorageDriverExtensions<'_> {
        crate::storage::traits::StorageDriverExtensions {
            list: Some(self),
            stream_upload: Some(self),
            presigned: Some(self),
            multipart: Some(self),
            ..Default::default()
        }
    }
}
