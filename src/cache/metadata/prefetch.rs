use std::{path::PathBuf, sync::Arc, time::Duration};

use dashmap::DashMap;
use tokio::time::{Instant, interval};

use super::cache::MetadataCache;
use crate::{METADATA_CACHE_LOGGER_DOMAIN, debug_log, info_log};

/// Tracks recently accessed files and prefetches their metadata.
/// Reduces cold start delays for hot files by keeping metadata fresh.
pub struct MetadataPrefetcher {
    cache: Arc<MetadataCache>,
    hot_files: Arc<DashMap<PathBuf, Instant>>,
    prefetch_interval_secs: u64,
    hot_file_ttl_secs: u64,
}

const DEFAULT_PREFETCH_INTERVAL_SECS: u64 = 30;
const DEFAULT_HOT_FILE_TTL_SECS: u64 = 300;
const MAX_HOT_FILES: usize = 1000;

impl MetadataPrefetcher {
    pub fn new(cache: Arc<MetadataCache>) -> Self {
        Self {
            cache,
            hot_files: Arc::new(DashMap::new()),
            prefetch_interval_secs: DEFAULT_PREFETCH_INTERVAL_SECS,
            hot_file_ttl_secs: DEFAULT_HOT_FILE_TTL_SECS,
        }
    }

    /// Tracks a file access for future prefetching.
    pub fn track_access(&self, path: PathBuf) {
        // Evict old entries if we're at capacity
        if self.hot_files.len() >= MAX_HOT_FILES {
            self.evict_old_entries();

            // If still at capacity after eviction, remove oldest entry
            if self.hot_files.len() >= MAX_HOT_FILES {
                if let Some(oldest) = self.find_oldest_entry() {
                    self.hot_files.remove(&oldest);
                }
            }
        }

        self.hot_files.insert(path, Instant::now());
    }

    /// Finds the path with the oldest access time.
    fn find_oldest_entry(&self) -> Option<PathBuf> {
        self.hot_files
            .iter()
            .min_by_key(|entry| *entry.value())
            .map(|entry| entry.key().clone())
    }

    /// Removes entries older than hot_file_ttl_secs.
    fn evict_old_entries(&self) {
        let now = Instant::now();
        let ttl = Duration::from_secs(self.hot_file_ttl_secs);

        self.hot_files
            .retain(|_, last_access| now.duration_since(*last_access) < ttl);
    }

    /// Starts background task to periodically prefetch hot file metadata.
    pub fn start_prefetch_task(self: Arc<Self>) {
        let interval_secs = self.prefetch_interval_secs;
        let ttl_secs = self.hot_file_ttl_secs;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));

            loop {
                ticker.tick().await;
                self.prefetch_hot_files().await;
            }
        });

        info_log!(
            METADATA_CACHE_LOGGER_DOMAIN,
            "Metadata prefetcher started: interval={}s ttl={}s",
            interval_secs,
            ttl_secs
        );
    }

    async fn prefetch_hot_files(&self) {
        self.evict_old_entries();

        let hot_count = self.hot_files.len();
        if hot_count == 0 {
            return;
        }

        debug_log!(
            METADATA_CACHE_LOGGER_DOMAIN,
            "Prefetching metadata for {} hot files",
            hot_count
        );

        let mut prefetched = 0;
        for entry in self.hot_files.iter() {
            if self.cache.fetch_metadata(entry.key()).await.is_ok() {
                prefetched += 1;
            }
        }

        debug_log!(
            METADATA_CACHE_LOGGER_DOMAIN,
            "Prefetch completed: {}/{} files refreshed",
            prefetched,
            hot_count
        );
    }

    pub fn hot_file_count(&self) -> usize {
        self.hot_files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_access_adds_file() {
        let cache = Arc::new(MetadataCache::new(100, 60));
        let prefetcher = MetadataPrefetcher::new(cache);

        let path = PathBuf::from("/test/file.mp4");
        prefetcher.track_access(path.clone());

        assert_eq!(prefetcher.hot_file_count(), 1);
        assert!(prefetcher.hot_files.contains_key(&path));
    }

    #[test]
    fn evict_old_entries_removes_stale_files() {
        let cache = Arc::new(MetadataCache::new(100, 60));
        let mut prefetcher = MetadataPrefetcher::new(cache);
        prefetcher.hot_file_ttl_secs = 0;

        let path = PathBuf::from("/test/file.mp4");
        prefetcher.track_access(path.clone());
        assert_eq!(prefetcher.hot_file_count(), 1);

        std::thread::sleep(Duration::from_millis(10));
        prefetcher.evict_old_entries();

        assert_eq!(prefetcher.hot_file_count(), 0);
    }

    #[test]
    fn max_hot_files_enforces_limit() {
        let cache = Arc::new(MetadataCache::new(100, 60));
        let prefetcher = MetadataPrefetcher::new(cache);

        // Add MAX_HOT_FILES + 10 files
        for i in 0..MAX_HOT_FILES + 10 {
            let path = PathBuf::from(format!("/test/file{}.mp4", i));
            prefetcher.track_access(path);
        }

        // Should never exceed MAX_HOT_FILES
        assert_eq!(prefetcher.hot_file_count(), MAX_HOT_FILES);
    }
}
