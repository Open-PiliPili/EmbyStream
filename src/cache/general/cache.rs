use std::{any::Any, fmt::Debug, sync::Arc, time::Duration};

use moka::sync::Cache as MokaCache;

/// A high-performance, thread-safe, generic cache powered by Moka.
///
/// This cache handles automatic expiration (TTL) and capacity-based
/// eviction (LRU) internally.
#[derive(Clone)]
pub struct Cache {
    inner: MokaCache<String, Arc<dyn Any + Send + Sync>>,
}

impl Cache {
    /// Creates a new cache with a given capacity and default item TTL.
    ///
    /// # Arguments
    ///
    /// * `max_capacity` - The maximum number of items to store in the cache.
    /// * `time_to_live` - The default time-to-live for each item.
    pub fn new(max_capacity: u64, time_to_live: u64) -> Self {
        let inner = MokaCache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(time_to_live))
            .build();

        Self { inner }
    }

    /// Inserts a key-value pair into the cache.
    ///
    /// If the key already exists, its value and expiration time are updated.
    /// If inserting a new item exceeds capacity, the least recently used
    /// item will be evicted.
    pub fn insert<V: 'static + Send + Sync + Debug>(&self, key: String, value: V) {
        self.inner.insert(key, Arc::new(value));
    }

    /// Retrieves a clone of a value from the cache by its key.
    ///
    /// Returns `None` if the key does not exist or the item has expired.
    /// The type `V` must match the type that was originally inserted.
    pub fn get<V: 'static + Clone>(&self, key: &str) -> Option<V> {
        self.inner
            .get(key)
            .and_then(|value| value.downcast_ref::<V>().map(|v| v.clone()))
    }

    /// Removes a key-value pair from the cache.
    pub fn remove(&self, key: &str) {
        self.inner.invalidate(key);
    }

    /// Returns the current number of entries in the cache.
    pub fn len(&self) -> u64 {
        self.inner.entry_count()
    }
}
