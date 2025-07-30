use std::{
    sync::{Arc, Weak},
    time::Duration,
};

use dashmap::DashMap;
use moka::future::{Cache, CacheBuilder};
use tokio::{sync::Semaphore, time};

use super::types::RateLimiter;

#[derive(Clone)]
pub struct RateLimiterCache {
    limiters: Cache<String, Arc<RateLimiter>>,
    active_limiters: Arc<DashMap<String, Weak<RateLimiter>>>,
    rate_kbs: u64,
}

impl RateLimiterCache {
    pub fn new(max_capacity: u64, time_to_live: u64, rate_kbs: u64) -> Self {
        let active_limiters = Arc::new(DashMap::new());
        let active_limiters_clone = active_limiters.clone();

        let limiters = CacheBuilder::new(max_capacity)
            .time_to_live(Duration::from_secs(time_to_live))
            .eviction_listener(move |key: Arc<String>, _value, _cause| {
                active_limiters_clone.remove(key.as_ref());
            })
            .build();

        Self {
            limiters,
            active_limiters,
            rate_kbs,
        }
    }

    pub async fn fetch_limiter(&self, device_id: &str) -> Arc<RateLimiter> {
        if self.rate_kbs == 0 {
            return Arc::new(RateLimiter {
                semaphore: Arc::new(Semaphore::new(usize::MAX / 2)),
            });
        }

        self.limiters
            .get_with(device_id.to_string(), async {
                let bytes_per_sec = (self.rate_kbs * 1024) as usize;

                let limiter = Arc::new(RateLimiter {
                    semaphore: Arc::new(Semaphore::new(bytes_per_sec)),
                });

                self.active_limiters
                    .insert(device_id.to_string(), Arc::downgrade(&limiter));

                limiter
            })
            .await
    }

    pub fn start_refill_task(&self) {
        if self.rate_kbs == 0 {
            return;
        }

        let active_limiters = self.active_limiters.clone();
        let bytes_to_add_per_second = (self.rate_kbs * 1024) as usize;
        let max_permits = bytes_to_add_per_second * 2;

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                active_limiters.retain(|_key, weak_limiter| {
                    if let Some(limiter) = weak_limiter.upgrade() {
                        let current_permits =
                            limiter.semaphore.available_permits();
                        if current_permits < max_permits {
                            limiter
                                .semaphore
                                .add_permits(bytes_to_add_per_second);
                        }
                        true
                    } else {
                        false
                    }
                });
            }
        });
    }

    pub fn get_limiters_count(&self) -> u64 {
        self.limiters.entry_count()
    }
}
