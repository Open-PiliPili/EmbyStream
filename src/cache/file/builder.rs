use std::{
    num::NonZeroUsize,
    sync::Arc
};

use dashmap::DashMap;
use lru::LruCache;
use tokio::{
    sync::RwLock as TokioRwLock
};

use crate::cache::file::cache::Cache;

pub struct CacheBuilder {
    max_capacity: usize,
    metadata_expiry: u64
}

impl CacheBuilder {

    pub fn new() -> Self {
        CacheBuilder {
            max_capacity: 2000,
            metadata_expiry: 60 * 60 * 60
        }
    }

    pub fn with_max_capacity(mut self, capacity: usize) -> Self {
        self.max_capacity = capacity;
        self
    }

    pub fn with_metadata_expiry(mut self, expiry_seconds: u64) -> Self {
        self.metadata_expiry = expiry_seconds;
        self
    }

    pub async fn build(self) -> Cache {
        let cache = Arc::new(DashMap::new());
        let lru = Arc::new(TokioRwLock::new(LruCache::new(
            NonZeroUsize::new(self.max_capacity).unwrap(),
        )));
        Cache {
            cache,
            lru,
            capacity: self.max_capacity,
            metadata_expiry: self.metadata_expiry
        }
    }
}