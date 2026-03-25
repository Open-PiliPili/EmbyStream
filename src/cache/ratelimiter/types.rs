use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;

pub struct RateLimiter {
    pub semaphore: Arc<Semaphore>,
    /// When true, local streaming skips per-chunk semaphore acquire (node `client_speed_limit_kbs == 0`).
    pub skip_semaphore: bool,
}

impl RateLimiter {
    /// No per-client byte cap (non-Disk local paths, missing cache entry, or `client_speed_limit_kbs == 0`).
    pub fn unlimited() -> Arc<Self> {
        static UNLIMITED: OnceLock<Arc<RateLimiter>> = OnceLock::new();
        UNLIMITED
            .get_or_init(|| {
                Arc::new(Self {
                    semaphore: Arc::new(Semaphore::new(Semaphore::MAX_PERMITS)),
                    skip_semaphore: true,
                })
            })
            .clone()
    }
}
