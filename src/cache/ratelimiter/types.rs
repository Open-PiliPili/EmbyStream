use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct RateLimiter {
    pub semaphore: Arc<Semaphore>,
}
