use std::sync::{Arc, RwLock};
use std::time::Duration;

use dashmap::DashMap;

use super::cache_inner::CacheInner;
use crate::cache::crypto::cache::Cache;

/// Builder for configuring and creating a Cache instance.
pub(crate) struct CacheBuilder {
    max_capacity: usize,
    default_ttl: Duration,
}

impl CacheBuilder {
    /// Creates a new CacheBuilder with default settings (2000 capacity, 30 minutes TTL).
    pub fn new() -> Self {
        CacheBuilder {
            max_capacity: 2000,
            default_ttl: Duration::from_secs(30 * 60),
        }
    }

    /// Sets the maximum cache capacity.
    pub fn with_max_capacity(mut self, capacity: usize) -> Self {
        self.max_capacity = capacity;
        self
    }

    /// Sets the default TTL in seconds.
    pub fn with_max_alive_seconds(mut self, seconds: u64) -> Self {
        self.default_ttl = Duration::from_secs(seconds);
        self
    }

    /// Builds and returns a Cache instance.
    pub fn build(self) -> Cache {
        Cache {
            inner: Arc::new(CacheInner {
                entries: DashMap::new(),
                order: RwLock::new(std::collections::VecDeque::new()),
            }),
            default_ttl: self.default_ttl,
            max_capacity: self.max_capacity,
        }
    }
}

impl Default for CacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}