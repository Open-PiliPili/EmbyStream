use std::{
    path::PathBuf,
    sync::Arc,
    time::{
        Duration,
        SystemTime,
        Instant
    }
};

use dashmap::DashMap;
use lru::LruCache;
use tokio::{
    fs::{
        metadata as TokioMetadata,
        File as TokioFile
    },
    sync::RwLock as TokioRwLock
};

use crate::cache::file::{
    Cached as CachedFile,
    Metadata as FileMetadata
};
use crate::{debug_log, error_log, FILE_CACHE_LOGGER_DOMAIN};
use super::builder::CacheBuilder;

pub struct Cache {
    pub(crate) cache: Arc<DashMap<PathBuf, CachedFile>>,
    pub(crate) lru: Arc<TokioRwLock<LruCache<PathBuf, ()>>>,
    pub(crate) capacity: usize,
    pub(crate) metadata_expiry: u64,
}

impl Cache {

    pub fn builder() -> CacheBuilder {
        CacheBuilder::new()
    }

    pub async fn get_file(&self, path: PathBuf) -> Option<Arc<TokioRwLock<TokioFile>>> {
        let start_total = Instant::now();

        let mut lru = self.lru.write().await;
        lru.put(path.clone(), ());

        // Check if the file is already in cache
        if let Some(mut cached_file) = self.cache.get_mut(&path) {
            // Increment reference count since a new user is accessing this file
            cached_file.reference_count += 1;
            // Update the last accessed time for LRU tracking
            cached_file.last_accessed = SystemTime::now();
            // Clone the existing file handle (Arc) to return to the user
            let file_handle = Arc::clone(&cached_file.file_handle);

            let duration_total = start_total.elapsed().as_millis();
            debug_log!(
                FILE_CACHE_LOGGER_DOMAIN,
                "Cache hit: Returned cached file handle for path={:?}, duration={}ms",
                path,
                duration_total
            );

            return Some(file_handle);
        }

        // If file is not in cache, open it asynchronously using tokio::fs
        let start_open = Instant::now();
        match TokioFile::open(&path).await {
            Ok(file) => {
                let duration_open = start_open.elapsed().as_millis();
                debug_log!(
                    FILE_CACHE_LOGGER_DOMAIN,
                    "File opened: path={:?}, duration={}ms",
                    path,
                    duration_open
                );

                let metadata = self.get_metadata(&path).await
                    .unwrap_or(FileMetadata::default());

                // Create a new file handle wrapped in Arc and TokioRwLock for thread-safe access
                let file_handle = Arc::new(TokioRwLock::new(file));
                let cached_file = CachedFile {
                    file_handle: Arc::clone(&file_handle),
                    metadata,
                    reference_count: 1,
                    last_accessed: SystemTime::now(),
                };

                // Insert the new file entry into the cache
                self.cache.insert(path.clone(), cached_file);

                // Check if eviction is needed due to capacity limits
                self.evict_if_needed(&mut lru).await;

                let duration_total = start_total.elapsed().as_millis();
                debug_log!(
                    FILE_CACHE_LOGGER_DOMAIN,
                    "Cache miss: Returned new file handle for path={:?}, total_duration={}ms",
                    path, duration_total
                );

                Some(file_handle)
            }
            Err(e) => {
                error_log!(
                    FILE_CACHE_LOGGER_DOMAIN,
                    "Failed to open file {:?}: {}",
                    path,
                    e
                );
                None
            }
        }
    }

    pub async fn release_file(&self, path: PathBuf) {
        // Check if the file exists in cache and decrement reference count
        if let Some(mut cached_file) = self.cache.get_mut(&path) {
            cached_file.reference_count = cached_file.reference_count.saturating_sub(1);
            cached_file.last_accessed = SystemTime::now();
            // Update LRU cache to mark this path as recently accessed
            let mut lru = self.lru.write().await;
            lru.put(path, ());
        }
    }

    async fn evict_if_needed(&self, lru: &mut LruCache<PathBuf, ()>) {
        // Evict entries if cache exceeds capacity
        while lru.len() > self.capacity {
            if let Some((path, _)) = lru.pop_lru() {
                // Only evict if reference count is 0 (no active users)
                if let Some(cached_file) = self.cache.get(&path) {
                    if cached_file.reference_count == 0 {
                        self.cache.remove(&path);
                        debug_log!(
                            FILE_CACHE_LOGGER_DOMAIN,
                            "Evicted file from cache due to capacity: {:?}",
                            path
                        );
                    } else {
                        // If still in use, reinsert into LRU cache
                        lru.put(path, ());
                    }
                }
            }
        }
    }

    pub async fn get_metadata(&self, path: &PathBuf) -> Option<FileMetadata> {
        let start_metadata = Instant::now();

        if let Some(cached_file) = self.cache.get(path) {
            let elapsed = SystemTime::now()
                .duration_since(cached_file.metadata.updated_at)
                .unwrap_or(Duration::from_secs(0));
            if elapsed < Duration::from_secs(self.metadata_expiry) && cached_file.metadata.is_valid() {
                let duration_metadata = start_metadata.elapsed().as_millis();
                debug_log!(
                    FILE_CACHE_LOGGER_DOMAIN,
                    "Metadata cache hit: path={:?}, duration={}ms (returned from cache)",
                    path, duration_metadata
                );
                return Some(cached_file.metadata.clone());
            }
        }

        let file = TokioMetadata(path).await.ok()?;
        let metadata = Some(FileMetadata {
            file_size: file.len(),
            format: path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("unknown")
                .to_string(),
            last_modified: file.modified().ok(),
            updated_at: SystemTime::now()
        });

        let duration_metadata = start_metadata.elapsed().as_millis();
        debug_log!(
            FILE_CACHE_LOGGER_DOMAIN,
            "Metadata fetched: path={:?}, duration={}ms",
            path, duration_metadata
        );

        metadata
    }
}