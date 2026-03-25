use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use moka::future::Cache;
use tokio::fs::metadata as TokioMetadata;

use crate::cache::metadata::{Error, Metadata};
use crate::{METADATA_CACHE_LOGGER_DOMAIN, debug_log, info_log};

/// FUSE and network filesystems often take 100-500ms for metadata queries.
const SLOW_METADATA_FETCH_THRESHOLD_MS: u128 = 100;

/// If total time exceeds this, multiple requests may have waited for same fetch.
const CONCURRENT_WAIT_THRESHOLD_MS: u128 = 50;

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
        let start = Instant::now();
        let path_buf = path.to_path_buf();

        let result = self
            .metadata
            .try_get_with(path_buf.clone(), async move {
                let fetch_start = Instant::now();
                let meta = TokioMetadata(path)
                    .await
                    .map_err(|e| Error::IoError(Arc::new(e)))?;

                let fetch_ms = fetch_start.elapsed().as_millis();
                if fetch_ms > SLOW_METADATA_FETCH_THRESHOLD_MS {
                    info_log!(
                        METADATA_CACHE_LOGGER_DOMAIN,
                        "Slow metadata fetch: path={:?} fetch_ms={} \
                         hint=FUSE_or_network_filesystem",
                        path,
                        fetch_ms
                    );
                }

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
            .map_err(|e: Arc<Error>| e.as_ref().clone());

        let total_ms = start.elapsed().as_millis();
        if total_ms > CONCURRENT_WAIT_THRESHOLD_MS {
            debug_log!(
                METADATA_CACHE_LOGGER_DOMAIN,
                "Metadata fetch completed: path={:?} total_ms={} \
                 hint=may_include_concurrent_wait",
                path_buf,
                total_ms
            );
        }

        result
    }

    pub fn get_metadata_count(&self) -> u64 {
        self.metadata.entry_count()
    }
}
