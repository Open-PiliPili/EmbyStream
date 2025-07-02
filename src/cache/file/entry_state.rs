use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct EntryState {
    pub in_use: bool,
    pub last_accessed: SystemTime,
}

impl EntryState {
    /// Checks if the entry is expired based on the given TTL and current time.
    pub fn is_expired(&self, now: SystemTime, ttl: Duration) -> bool {
        now.duration_since(self.last_accessed)
            .map(|elapsed| elapsed > ttl)
            .unwrap_or(true)
    }
}

impl Default for EntryState {
    fn default() -> Self {
        EntryState {
            in_use: true,
            last_accessed: SystemTime::now(),
        }
    }
}
