use crate::cache::crypto::cache::Cache;
use std::sync::{Arc, OnceLock};

pub struct CacheManager {
    encrypted_cache: OnceLock<Arc<Cache>>,
    decrypted_cache: OnceLock<Arc<Cache>>,
    encrypted_capacity: usize,
    encrypted_ttl_secs: u64,
    decrypted_capacity: usize,
    decrypted_ttl_secs: u64,
}

impl CacheManager {
    /// Creates a new CacheManager with uninitialized cache instances.
    pub fn new(
        encrypted_capacity: usize,
        encrypted_ttl_secs: u64,
        decrypted_capacity: usize,
        decrypted_ttl_secs: u64,
    ) -> Self {
        CacheManager {
            encrypted_cache: OnceLock::new(),
            decrypted_cache: OnceLock::new(),
            encrypted_capacity,
            encrypted_ttl_secs,
            decrypted_capacity,
            decrypted_ttl_secs,
        }
    }

    /// Returns a reference to the encrypted cache, initializing it if necessary.
    pub fn encrypted_cache(&self) -> &Arc<Cache> {
        self.encrypted_cache.get_or_init(|| {
            Arc::new(
                Cache::builder()
                    .with_max_capacity(self.encrypted_capacity)
                    .with_max_alive_seconds(self.encrypted_ttl_secs)
                    .build(),
            )
        })
    }

    /// Returns a reference to the decrypted cache, initializing it if necessary.
    pub fn decrypted_cache(&self) -> &Arc<Cache> {
        self.decrypted_cache.get_or_init(|| {
            Arc::new(
                Cache::builder()
                    .with_max_capacity(self.decrypted_capacity)
                    .with_max_alive_seconds(self.decrypted_ttl_secs)
                    .build(),
            )
        })
    }
}
