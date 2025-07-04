use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration
};

use moka::future::Cache;
use tokio::fs::metadata as TokioMetadata;

use crate::cache::file::{pool::FileEntryPool, Entry, Error, Metadata};

#[derive(Clone)]
pub struct FileCache {
    entry_pools: Cache<PathBuf, Arc<FileEntryPool>>,
    metadata: Cache<PathBuf, Metadata>,
}

impl FileCache {
    pub fn new(ttl_seconds: u64) -> Self {
        let entry_pools = Cache::builder()
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();

        let metadata = Cache::builder()
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();

        Self {
            entry_pools,
            metadata,
        }
    }

    pub async fn fetch_entry(&self, path: &Path) -> Result<Entry, Error> {
        let pool = self
            .entry_pools
            .try_get_with(path.to_path_buf(), async {
                Ok(Arc::new(FileEntryPool::new(path.to_path_buf())))
            })
            .await
            .map_err(|e: Arc<Error>| e.as_ref().clone())?;

        pool.get_or_create_entry().await
    }

    pub async fn fetch_metadata(&self, path: &Path) -> Result<Metadata, Error> {
        self.metadata
            .try_get_with(path.to_path_buf(), async move {
                let meta = TokioMetadata(path)
                    .await
                    .map_err(|e| Error::IoError(Arc::new(e)))?;

                let metadata = Metadata {
                    file_size: meta.len(),
                    format: path
                        .extension()
                        .and_then(|s| s.to_str())
                        .map_or_else(|| "unknown".to_string(), |s| s.to_string()),
                    last_modified: meta.modified().ok(),
                    updated_at: std::time::SystemTime::now(),
                };

                Ok(metadata)
            })
            .await
            .map_err(|e: Arc<Error>| e.as_ref().clone())
    }

    pub fn get_pool_count(&self) -> u64 {
        self.entry_pools.entry_count()
    }

    pub fn get_metadata_count(&self) -> u64 {
        self.metadata.entry_count()
    }
}