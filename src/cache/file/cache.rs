use std::{
    collections::HashSet,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use dashmap::DashMap;
use tokio::{
    fs::{File as TokioFile, metadata as TokioMetadata},
    sync::RwLock as TokioRwLock,
};

use super::builder::CacheBuilder;
use crate::cache::file::{
    CacheInner as FileCacheInner, Entry as FileEntry, Error as FileCacheError,
    Metadata as FileMetadata, Metadata,
};
use crate::{FILE_CACHE_LOGGER_DOMAIN, debug_log};

#[derive(Clone, Debug)]
pub struct Cache {
    pub(crate) cache: Arc<DashMap<PathBuf, FileCacheInner>>,
    pub(crate) default_ttl: Duration,
    pub(crate) clean_interval: Duration,
    pub(crate) last_cleaned: Arc<TokioRwLock<Instant>>,
}

impl Cache {
    pub fn builder() -> CacheBuilder {
        CacheBuilder::new()
    }

    /// Atomically gets a file handle for the given path.
    pub async fn fetch_entry(&self, path: PathBuf) -> Result<FileEntry, FileCacheError> {
        self.check_and_clean_expired().await;
        if let Some(file) = self.retrieve_entry(path.clone()).await {
            return Ok(file);
        }
        self.fetch_new_entry(path.clone()).await
    }

    /// Atomically releases a file handle, marking it as available.
    pub async fn release_entry(&self, entry: &FileEntry) {
        let mut state = entry.state.write().await;
        state.in_use = false;
        state.last_accessed = SystemTime::now();
    }

    /// Atomically gets metadata for the given path.
    pub async fn fetch_metadata(&self, path: &PathBuf) -> Result<FileMetadata, FileCacheError> {
        if let Some(metadata) = self.retrieve_metadata(path).await {
            return Ok(metadata);
        }

        let fetch_result = self.fetch_new_metadata(path).await;
        match fetch_result {
            Ok(metadata) => {
                self.update_metadata(path.clone(), metadata.clone()).await;
                Ok(metadata)
            }
            Err(error) => Err(error),
        }
    }

    async fn update_metadata(&self, path: PathBuf, metadata: Metadata) {
        let cache_inner = self
            .cache
            .entry(path.clone())
            .or_insert(FileCacheInner::new(None));
        let mut meta_lock = cache_inner.metadata.write().await;
        *meta_lock = Some(metadata.clone());

        debug_log!(
            FILE_CACHE_LOGGER_DOMAIN,
            "Update metatdata to cache：path={:?}，fileSize={}，format={}",
            path,
            metadata.file_size,
            metadata.format
        );
    }

    /// Atomically retrieve a file.
    async fn retrieve_entry(&self, path: PathBuf) -> Option<FileEntry> {
        let system_now = SystemTime::now();
        let retrieve_start = Instant::now();

        if let Some(cache_inner) = self.cache.get(&path) {
            let mut entries = cache_inner.entries.write().await;

            for entry in entries.iter_mut() {
                let mut state = entry.state.write().await;
                if !state.in_use && !state.is_expired(system_now, self.default_ttl) {
                    state.in_use = true;
                    state.last_accessed = system_now;
                    debug_log!(
                        FILE_CACHE_LOGGER_DOMAIN,
                        "Reusing cached file entry: path={:?}, last_accessed={:?}, elapsed={:?}",
                        path,
                        system_now,
                        Instant::now().duration_since(retrieve_start)
                    );
                    return Some(entry.clone());
                }
            }
        }

        debug_log!(
            FILE_CACHE_LOGGER_DOMAIN,
            "No available cached file: path={:?}, elapsed={:?}",
            path,
            Instant::now().duration_since(retrieve_start)
        );
        None
    }

    /// Atomically fetch a file entry.
    async fn fetch_new_entry(&self, path: PathBuf) -> Result<FileEntry, FileCacheError> {
        let fetch_start = Instant::now();

        let file = TokioFile::open(&path)
            .await
            .map_err(FileCacheError::IoError)?;

        let last_accessed = SystemTime::now();
        let new_entry = FileEntry::new(file, path.clone());

        let cache_inner = self
            .cache
            .entry(path.clone())
            .or_insert_with(|| FileCacheInner::new(None));

        {
            let mut entries = cache_inner.entries.write().await;
            let mut order = cache_inner.order.write().await;
            entries.push(new_entry.clone());
            order.push_back(path.clone());
        }

        debug_log!(
            FILE_CACHE_LOGGER_DOMAIN,
            "Cached new file entry: path={:?}, last_accessed={:?}, elapsed={:?}",
            path,
            last_accessed,
            Instant::now().duration_since(fetch_start)
        );

        Ok(new_entry)
    }

    /// Atomically fetches or retrieves metadata.
    async fn retrieve_metadata(&self, path: &PathBuf) -> Option<FileMetadata> {
        let system_now = SystemTime::now();
        let retrieve_start = Instant::now();

        if let Some(cache_inner) = self.cache.get(path) {
            let metadata = cache_inner.metadata.read().await;
            if let Some(meta) = metadata.as_ref() {
                let elapsed = system_now
                    .duration_since(meta.updated_at)
                    .unwrap_or(Duration::from_secs(0));
                if meta.is_valid() && elapsed <= self.default_ttl {
                    debug_log!(
                        FILE_CACHE_LOGGER_DOMAIN,
                        "Found metadata in cache：path={:?}, elapsed={:?}",
                        path,
                        Instant::now().duration_since(retrieve_start)
                    );
                    return Some(meta.clone());
                }
            }
        }

        debug_log!(
            FILE_CACHE_LOGGER_DOMAIN,
            "Unable to find valid metadata in cache, path={:?}, elapsed={:?}",
            path,
            Instant::now().duration_since(retrieve_start)
        );
        None
    }

    async fn fetch_new_metadata(&self, path: &PathBuf) -> Result<FileMetadata, FileCacheError> {
        let system_now = SystemTime::now();
        let retrieve_start = Instant::now();

        let meta = TokioMetadata(path).await.map_err(FileCacheError::IoError)?;

        let metadata = FileMetadata {
            file_size: meta.len(),
            format: path
                .extension()
                .map(|ext| ext.to_string_lossy().into_owned())
                .unwrap_or("unknown".to_string()),
            last_modified: meta.modified().ok(),
            updated_at: system_now,
        };

        debug_log!(
            FILE_CACHE_LOGGER_DOMAIN,
            "Caching metadata：path={:?}，fileSize={}，format={}, elapsed={:?}",
            path,
            metadata.file_size,
            metadata.format,
            Instant::now().duration_since(retrieve_start)
        );

        Ok(metadata)
    }

    pub async fn len(&self) -> usize {
        let mut total = 0;
        for entry in self.cache.iter() {
            let cache_inner = entry.value();
            let entries = cache_inner.entries.read().await;
            total += entries.len();
        }
        total
    }

    /// Checks if cleanup is needed and performs it atomically.
    pub async fn check_and_clean_expired(&self) {
        let check_start = Instant::now();

        {
            let last_cleaned = self.last_cleaned.read().await;
            if check_start.duration_since(*last_cleaned) <= self.clean_interval {
                return;
            }
        }

        let mut last_cleaned_guard = self.last_cleaned.write().await;

        if check_start.duration_since(*last_cleaned_guard) > self.clean_interval {
            let clean_start = Instant::now();
            self.clean_expired().await;
            *last_cleaned_guard = clean_start;
            debug_log!(
                FILE_CACHE_LOGGER_DOMAIN,
                "Performed cache cleanup at time={:?}, elapsed={:?}",
                clean_start,
                Instant::now().duration_since(clean_start)
            );
        }
    }

    /// Atomically cleans expired entries, respecting in_use entries.
    async fn clean_expired(&self) {
        let mut expired_paths = Vec::new();
        let system_now = SystemTime::now();
        let clean_start = Instant::now();

        debug_log!(
            FILE_CACHE_LOGGER_DOMAIN,
            "Ready for clean expired cache entry at time={:?}",
            clean_start
        );

        for entry in self.cache.iter() {
            let path = entry.key();
            let cache_inner = entry.value();

            let (all_expired, need_clean) = {
                let entries = cache_inner.entries.read().await;
                let mut all_expired = !entries.is_empty();
                let mut need_clean = false;

                for e in entries.iter() {
                    let state = e.state.read().await;
                    let is_expired = state.is_expired(system_now, self.default_ttl);
                    debug_log!(
                        FILE_CACHE_LOGGER_DOMAIN,
                        "Entry last accessed time: {:?}, now: {:?}",
                        state.last_accessed,
                        system_now,
                    );
                    if state.in_use || !is_expired {
                        all_expired = false;
                    }
                    if !state.in_use && is_expired {
                        need_clean = true;
                    }
                }

                (all_expired, need_clean)
            };

            if need_clean {
                let mut entries = cache_inner.entries.write().await;
                let mut order = cache_inner.order.write().await;

                let mut valid_entries = Vec::new();
                let mut valid_paths = HashSet::new();

                for e in entries.drain(..) {
                    let state = e.state.read().await;
                    if state.in_use || !state.is_expired(system_now, self.default_ttl) {
                        valid_paths.insert(e.path.clone());
                        valid_entries.push(e.clone());
                    }
                }
                *entries = valid_entries;
                order.retain(|k| valid_paths.contains(k));

                if entries.is_empty() || all_expired {
                    expired_paths.push(path.clone());
                }
            }
        }

        for path in expired_paths {
            self.cache.remove(&path);
            debug_log!(
                FILE_CACHE_LOGGER_DOMAIN,
                "Removed expired cache entry: path={:?}",
                path
            );
        }

        debug_log!(
            FILE_CACHE_LOGGER_DOMAIN,
            "Removed expired cache entries, elapsed={:?}",
            Instant::now().duration_since(clean_start)
        );
    }
}

unsafe impl Send for Cache {}
unsafe impl Sync for Cache {}
