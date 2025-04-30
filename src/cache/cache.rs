use std::{
    any::Any,
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::{Duration, Instant}
};

use dashmap::DashMap;
use once_cell::sync::OnceCell;

use super::builder;
use crate::{CACHE_LOGGER_DOMAIN, error_log, debug_log};

/// Thread-safe cache with automatic expiration and capacity limits.
pub struct Cache {
    // Inner cache state, using DashMap for thread-safe entries
    pub(crate) inner: Arc<CacheInner>,
    // Default TTL for entries
    pub(crate) default_ttl: Duration,
    // Maximum number of entries
    pub(crate) max_capacity: usize,
}

/// Inner cache state, holding entries and insertion order
pub(crate) struct CacheInner {
    // Key-value store: key -> (value, insertion time, TTL)
    pub entries: DashMap<String, (Box<dyn Any + Send + Sync>, Instant, Duration)>,
    // Tracks insertion order for FIFO eviction
    pub order: RwLock<VecDeque<String>>,
}

// Global singleton instance of Cache
static INSTANCE: OnceCell<Arc<Cache>> = OnceCell::new();

impl Cache {
    /// Returns the global singleton Cache instance.
    pub fn get_instance() -> &'static Arc<Cache> {
        INSTANCE.get_or_init(|| {
            builder::CacheBuilder::new()
                .with_max_capacity(2000)
                .with_max_alive_seconds(30 * 60)
                .build()
        })
    }

    /// Creates a new CacheBuilder for configuring the cache.
    pub fn builder() -> builder::CacheBuilder {
        builder::CacheBuilder::new()
    }

    /// Inserts a key-value pair with optional TTL (uses default if None).
    /// Evicts oldest entries if capacity is exceeded.
    /// Refreshes TTL if key already exists.
    pub fn insert<V: 'static + Send + Sync>(&self, key: String, value: V) {
        let now = Instant::now();

        debug_log!(CACHE_LOGGER_DOMAIN, "Inserting cache entry: key={}", key);

        // Clean expired entries
        Self::clean_expired(&self.inner, &now);

        // Update order
        let mut order = match self.inner.order.write() {
            Ok(order) => order,
            Err(e) => {
                error_log!(CACHE_LOGGER_DOMAIN, "Failed to acquire write lock for order: {}", e);
                return;
            }
        };
        order.retain(|k| k != &key);

        // Insert new entry
        self.inner.entries.insert(key.clone(), (Box::new(value), now, self.default_ttl));
        order.push_back(key.clone());

        // Evict the oldest entries if over capacity
        while self.inner.entries.len() > self.max_capacity {
            if let Some(oldest_key) = order.pop_front() {
                self.inner.entries.remove(&oldest_key);
                debug_log!(CACHE_LOGGER_DOMAIN, "Evicted oldest cache entry: key={}", oldest_key);
            }
        }
    }

    /// Retrieves a value by key, returning None if not found or expired.
    /// Returns a cloned value to avoid lifetime issues.
    pub fn get<V: 'static + Clone>(&self, key: &str) -> Option<V> {
        let now = Instant::now();

        // Clean expired entries
        Self::clean_expired(&self.inner, &now);

        // Get value
        self.inner
            .entries
            .get(key)
            .and_then(|entry| {
                let (value, inserted, ttl) = entry.value();
                if now.duration_since(*inserted) > *ttl {
                    None // Expired
                } else {
                    value.downcast_ref::<V>().map(|v| v.clone())
                }
            })
    }

    /// Removes a key from the cache.
    pub fn remove(&self, key: &str) {
        if self.inner.entries.remove(key).is_some() {
            if let Ok(mut order) = self.inner.order.write() {
                order.retain(|k| k != key);
                debug_log!(CACHE_LOGGER_DOMAIN, "Removed cache entry: key={}", key);
            } else {
                error_log!(CACHE_LOGGER_DOMAIN, "Failed to acquire write lock for order");
            }
        }
    }

    /// Returns the current number of entries in the cache.
    pub fn len(&self) -> usize {
        self.inner.entries.len()
    }

    /// Cleans expired entries from the cache.
    fn clean_expired(inner: &CacheInner, now: &Instant) {
        let expired_keys: Vec<String> = inner
            .entries
            .iter()
            .filter(|entry| {
                let (_, inserted, ttl) = entry.value();
                now.duration_since(*inserted) > *ttl
            })
            .map(|entry| entry.key().clone())
            .collect();

        for key in expired_keys {
            inner.entries.remove(&key);
            if let Ok(mut order) = inner.order.write() {
                order.retain(|k| k != &key);
                debug_log!(CACHE_LOGGER_DOMAIN, "Removed expired cache entry: key={}", key);
            }
        }
    }
}

// Ensure Cache is Send and Sync for thread safety
unsafe impl Send for Cache {}
unsafe impl Sync for Cache {}