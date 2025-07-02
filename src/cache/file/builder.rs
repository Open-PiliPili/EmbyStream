use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use tokio::sync::RwLock as TokioRwLock;

use crate::cache::file::cache::Cache as FileCache;

pub struct CacheBuilder {
    default_ttl: Duration,
    clean_interval: Duration,
}

impl CacheBuilder {
    pub fn new() -> Self {
        CacheBuilder {
            default_ttl: Duration::from_secs(60 * 60),
            clean_interval: Duration::from_secs(60 * 10),
        }
    }

    /// Sets the default TTL in seconds.
    pub fn with_max_alive_seconds(mut self, seconds: u64) -> Self {
        self.default_ttl = Duration::from_secs(seconds);
        self
    }

    pub fn with_clean_interval(mut self, seconds: u64) -> Self {
        self.clean_interval = Duration::from_secs(seconds);
        self
    }

    pub async fn build(self) -> FileCache {
        FileCache {
            cache: Arc::new(DashMap::new()),
            default_ttl: self.default_ttl,
            clean_interval: self.clean_interval,
            last_cleaned: Arc::new(TokioRwLock::new(Instant::now())),
        }
    }
}
