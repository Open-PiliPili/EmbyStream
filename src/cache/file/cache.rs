use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use dashmap::DashMap;
use lru::LruCache;
use tokio::{
    fs::{File as TokioFile, metadata as TokioMetadata},
    sync::RwLock as TokioRwLock,
};

use super::builder::CacheBuilder;
use crate::cache::file::{Cached as CachedFile, Metadata as FileMetadata};
use crate::{FILE_CACHE_LOGGER_DOMAIN, debug_log};

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

        if let Some(mut cached) = self.cache.get_mut(&path) {
            let file_handle = cached.get_available_handle(&path).await;
            let duration_total = start_total.elapsed().as_millis();
            debug_log!(
                FILE_CACHE_LOGGER_DOMAIN,
                "Cache hit: Returned handle for path={:?}, total_duration={}ms",
                path,
                duration_total
            );
            return file_handle;
        }

        let metadata = self
            .get_metadata(&path)
            .await
            .unwrap_or(FileMetadata::default());
        let mut new_cached = CachedFile::new(metadata);
        let file_handle = new_cached.get_available_handle(&path).await;

        self.cache.insert(path.clone(), new_cached);
        self.evict_if_needed(&mut lru).await;

        let duration_total = start_total.elapsed().as_millis();
        debug_log!(
            FILE_CACHE_LOGGER_DOMAIN,
            "Cache miss and new handle: Returned handle for path={:?}, total_duration={}ms",
            path,
            duration_total
        );

        file_handle
    }

    pub async fn release_file(&self, path: PathBuf) {
        if let Some(mut cached) = self.cache.get_mut(&path) {
            for entry in &mut cached.file_handles {
                if entry.in_use {
                    entry.in_use = false;
                    entry.last_accessed = SystemTime::now();
                    break;
                }
            }
            debug_log!(
                FILE_CACHE_LOGGER_DOMAIN,
                "Released handle for path={:?}, pool_size={}",
                path,
                cached.file_handles.len()
            );
            cached.evict_if_needed(&path);
        }

        let mut lru = self.lru.write().await;
        lru.put(path, ());
    }

    async fn evict_if_needed(&self, lru: &mut LruCache<PathBuf, ()>) {
        while lru.len() > self.capacity {
            if let Some((path, _)) = lru.pop_lru() {
                if let Some(cached) = self.cache.get(&path) {
                    if cached.file_handles.iter().all(|entry| !entry.in_use) {
                        self.cache.remove(&path);
                        debug_log!(
                            FILE_CACHE_LOGGER_DOMAIN,
                            "Evicted file entry from cache due to capacity: {:?}",
                            path
                        );
                    } else {
                        lru.put(path, ());
                    }
                }
            }
        }
    }

    pub async fn get_metadata(&self, path: &PathBuf) -> Option<FileMetadata> {
        let start_metadata = Instant::now();

        if let Some(cached) = self.cache.get(path) {
            let elapsed = SystemTime::now()
                .duration_since(cached.metadata.updated_at)
                .unwrap_or(Duration::from_secs(0));
            if elapsed < Duration::from_secs(self.metadata_expiry) && cached.metadata.is_valid() {
                let duration_metadata = start_metadata.elapsed().as_millis();
                debug_log!(
                    FILE_CACHE_LOGGER_DOMAIN,
                    "Metadata cache hit: path={:?}, duration={}ms (returned from cache)",
                    path,
                    duration_metadata
                );
                return Some(cached.metadata.clone());
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
            updated_at: SystemTime::now(),
        });

        let duration_metadata = start_metadata.elapsed().as_millis();
        debug_log!(
            FILE_CACHE_LOGGER_DOMAIN,
            "Metadata fetched: path={:?}, duration={}ms",
            path,
            duration_metadata
        );

        metadata
    }
}
