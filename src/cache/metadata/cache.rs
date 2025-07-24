use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use moka::future::Cache;
use tokio::fs::metadata as TokioMetadata;

use crate::cache::metadata::{Error, Metadata};

#[derive(Clone)]
pub struct MetadataCache {
    metadata: Cache<PathBuf, Metadata>,
}

impl MetadataCache {
    pub fn new(max_capacity: u64, time_to_live: u64) -> Self {
        let metadata = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(time_to_live))
            .build();

        Self { metadata }
    }

    pub async fn fetch_metadata(&self, path: &Path) -> Result<Metadata, Error> {
        self.metadata
            .try_get_with(path.to_path_buf(), async move {
                let meta = TokioMetadata(path)
                    .await
                    .map_err(|e| Error::IoError(Arc::new(e)))?;

                let metadata = Metadata {
                    file_size: meta.len(),
                    file_name: path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map_or_else(
                            || "unknown".to_string(),
                            |s| s.to_string(),
                        ),
                    format: path
                        .extension()
                        .and_then(|s| s.to_str())
                        .map_or_else(
                            || "unknown".to_string(),
                            |s| s.to_string(),
                        ),
                    last_modified: meta.modified().ok(),
                    updated_at: std::time::SystemTime::now(),
                };

                Ok(metadata)
            })
            .await
            .map_err(|e: Arc<Error>| e.as_ref().clone())
    }

    pub fn get_metadata_count(&self) -> u64 {
        self.metadata.entry_count()
    }
}
