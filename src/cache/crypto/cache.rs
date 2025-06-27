use std::{
    sync::{Arc},
    time::{Duration, Instant},
    fmt::Debug
};
use super::{builder::CacheBuilder, cache_inner::CacheInner};
use crate::{CRYPTO_CACHE_LOGGER_DOMAIN, debug_log, error_log};

/// Thread-safe cache with automatic expiration and capacity limits.
pub struct Cache {
    // Inner cache state, using DashMap for thread-safe entries
    pub(crate) inner: Arc<CacheInner>,
    // Default TTL for entries
    pub(crate) default_ttl: Duration,
    // Maximum number of entries
    pub(crate) max_capacity: usize,
}

impl Cache {
    /// Creates a new CacheBuilder for configuring the cache.
    pub(crate) fn builder() -> CacheBuilder {
        CacheBuilder::new()
    }

    /// Inserts a key-value pair with optional TTL (uses default if None).
    /// Evicts oldest entries if capacity is exceeded.
    /// Refreshes TTL if key already exists.
    pub fn insert<V: 'static + Send + Sync + Debug>(&self, key: String, value: V) {
        let now = Instant::now();

        debug_log!(
            CRYPTO_CACHE_LOGGER_DOMAIN,
            "Inserting cache entry: key={}, value={:?}",
            key,
            value
        );

        // Update order
        let mut order = match self.inner.order.write() {
            Ok(order) => order,
            Err(e) => {
                error_log!(
                    CRYPTO_CACHE_LOGGER_DOMAIN,
                    "Failed to acquire write lock for order: {}",
                    e
                );
                return;
            }
        };
        order.retain(|k| k != &key);

        // Insert new entry
        self.inner
            .entries
            .insert(key.clone(), (Box::new(value), now, self.default_ttl));
        order.push_back(key.clone());

        // Evict the oldest entries if over capacity
        while self.len() > self.max_capacity {
            if let Some(oldest_key) = order.pop_front() {
                self.inner.entries.remove(&oldest_key);
                debug_log!(
                    CRYPTO_CACHE_LOGGER_DOMAIN,
                    "Evicted oldest cache entry: key={}",
                    oldest_key
                );
            }
        }
    }

    /// Retrieves a value by key, returning None if not found or expired.
    /// Returns a cloned value to avoid lifetime issues.
    pub fn get<V: 'static + Clone + Debug>(&self, key: &str) -> Option<V> {
        let now = Instant::now();

        // Get value
        let result = self.inner.entries.get(key).and_then(|entry| {
            let (value, inserted, ttl) = entry.value();
            if now.duration_since(*inserted) > *ttl {
                self.inner.entries.remove(key);
                if let Ok(mut order) = self.inner.order.write() {
                    order.retain(|k| k != key);
                    debug_log!(CRYPTO_CACHE_LOGGER_DOMAIN, "Removed expired cache entry: key={}", key);
                }
                None
            } else {
                value.downcast_ref::<V>().map(|v| v.clone())
            }
        });

        debug_log!(
            CRYPTO_CACHE_LOGGER_DOMAIN,
            "Retrieved cache entry: key={}, value={}",
            key,
            result.as_ref().map(|v| format!("{:?}", v)).unwrap_or("None".to_string())
        );

        result
    }

    /// Removes a key from the cache.
    pub fn remove(&self, key: &str) {
        if self.inner.entries.remove(key).is_some() {
            if let Ok(mut order) = self.inner.order.write() {
                order.retain(|k| k != key);
                debug_log!(CRYPTO_CACHE_LOGGER_DOMAIN, "Removed cache entry: key={}", key);
            } else {
                error_log!(
                    CRYPTO_CACHE_LOGGER_DOMAIN,
                    "Failed to acquire write lock for order"
                );
            }
        }
    }

    /// Returns the current number of entries in the cache.
    pub fn len(&self) -> usize {
        self.inner.entries.len()
    }
}

// Ensure Cache is Send and Sync for thread safety
unsafe impl Send for Cache {}
unsafe impl Sync for Cache {}
