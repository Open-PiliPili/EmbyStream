use std::{
    any::Any,
    collections::VecDeque,
    sync::RwLock,
    time::{Duration, Instant},
};

use dashmap::DashMap;

/// Inner cache state, holding entries and insertion order
pub(crate) struct CacheInner {
    // Key-value store: key -> (value, insertion time, TTL)
    pub entries: DashMap<String, (Box<dyn Any + Send + Sync>, Instant, Duration)>,
    // Tracks insertion order for FIFO eviction
    pub order: RwLock<VecDeque<String>>,
}
