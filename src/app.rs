use std::{collections::HashSet, ops::Deref as DerefTrait};

use regex::Regex;
use tokio::sync::{OnceCell, RwLock as TokioRwLock};

use crate::{
    cache::{GeneralCache, MetadataCache, RateLimiterCache},
    config::core::Config,
    core::backend::types::BackendRoutes,
    util::path_rewriter::PathRewriter,
};

// These constants define the user agent substrings for clients that require
// a workaround for missing Range headers.
const PROBLEMATIC_CLIENTS: &[&str] =
    &["yamby", "hills", "embytolocalplayer", "Emby/"];

pub struct AppState {
    config: TokioRwLock<Config>,
    frontend_path_rewrite_cache: OnceCell<Vec<PathRewriter>>,
    backend_path_rewrite_cache: OnceCell<Vec<PathRewriter>>,
    problematic_clients_cache: OnceCell<Vec<String>>,
    metadata_cache: OnceCell<MetadataCache>,
    encrypt_cache: OnceCell<GeneralCache>,
    decrypt_cache: OnceCell<GeneralCache>,
    strm_file_cache: OnceCell<GeneralCache>,
    forward_info_cache: OnceCell<GeneralCache>,
    open_list_cache: OnceCell<GeneralCache>,
    rate_limiter_cache: OnceCell<RateLimiterCache>,
    backend_routes_cache: OnceCell<BackendRoutes>,
}

impl AppState {
    pub async fn new(config: Config) -> Self {
        Self {
            config: TokioRwLock::new(config),
            frontend_path_rewrite_cache: OnceCell::new(),
            backend_path_rewrite_cache: OnceCell::new(),
            problematic_clients_cache: OnceCell::new(),
            metadata_cache: OnceCell::new(),
            encrypt_cache: OnceCell::new(),
            decrypt_cache: OnceCell::new(),
            strm_file_cache: OnceCell::new(),
            forward_info_cache: OnceCell::new(),
            open_list_cache: OnceCell::new(),
            rate_limiter_cache: OnceCell::new(),
            backend_routes_cache: OnceCell::new(),
        }
    }

    pub async fn get_config(&self) -> impl DerefTrait<Target = Config> + '_ {
        self.config.read().await
    }

    pub async fn get_cache_settings(&self) -> (u64, u64) {
        let config = self.get_config().await;
        match config.general.memory_mode.as_str() {
            "low" => (256, 60 * 60 * 2),
            "high" => (512, 60 * 60 * 6),
            _ => (512, 60 * 60 * 4),
        }
    }

    pub async fn get_frontend_path_rewrite_cache(&self) -> &Vec<PathRewriter> {
        let config = self.get_config().await;
        self.frontend_path_rewrite_cache
            .get_or_init(|| async move {
                let frontend_config = match &config.frontend {
                    Some(config) => config,
                    None => return vec![],
                };
                frontend_config
                    .clone()
                    .path_rewrites
                    .into_iter()
                    .map(|path_rewrite| {
                        PathRewriter::new(
                            path_rewrite.enable,
                            &path_rewrite.pattern,
                            &path_rewrite.replacement,
                        )
                    })
                    .collect()
            })
            .await
    }

    pub async fn get_backend_path_rewrite_cache(&self) -> &Vec<PathRewriter> {
        let config = self.get_config().await;
        self.backend_path_rewrite_cache
            .get_or_init(|| async move {
                let backend_config = match &config.backend {
                    Some(config) => config,
                    None => return vec![],
                };
                backend_config
                    .clone()
                    .path_rewrites
                    .into_iter()
                    .map(|path_rewrite| {
                        PathRewriter::new(
                            path_rewrite.enable,
                            &path_rewrite.pattern,
                            &path_rewrite.replacement,
                        )
                    })
                    .collect()
            })
            .await
    }

    pub async fn get_problematic_clients(&self) -> &Vec<String> {
        let config = self.get_config().await;
        self.problematic_clients_cache
            .get_or_init(|| async move {
                let mut clients: HashSet<String> = PROBLEMATIC_CLIENTS
                    .iter()
                    .map(|s| s.to_lowercase())
                    .collect();

                if let Some(backend_config) = config.backend.as_ref() {
                    clients.extend(
                        backend_config
                            .problematic_clients
                            .iter()
                            .map(|s| s.to_lowercase()),
                    );
                }

                clients.into_iter().filter(|s| !s.is_empty()).collect()
            })
            .await
    }

    pub async fn get_metadata_cache(&self) -> &MetadataCache {
        let (capacity, ttl) = self.get_cache_settings().await;
        self.metadata_cache
            .get_or_init(|| async move { MetadataCache::new(capacity, ttl) })
            .await
    }

    pub async fn get_encrypt_cache(&self) -> &GeneralCache {
        let (capacity, ttl) = self.get_cache_settings().await;
        self.encrypt_cache
            .get_or_init(|| async move { GeneralCache::new(capacity, ttl) })
            .await
    }

    pub async fn get_decrypt_cache(&self) -> &GeneralCache {
        let (capacity, ttl) = self.get_cache_settings().await;
        self.decrypt_cache
            .get_or_init(|| async move { GeneralCache::new(capacity, ttl) })
            .await
    }

    pub async fn get_strm_file_cache(&self) -> &GeneralCache {
        let (capacity, ttl) = self.get_cache_settings().await;
        self.strm_file_cache
            .get_or_init(|| async move { GeneralCache::new(capacity, ttl) })
            .await
    }

    pub async fn get_forward_info_cache(&self) -> &GeneralCache {
        let (capacity, ttl) = self.get_cache_settings().await;
        self.forward_info_cache
            .get_or_init(|| async move { GeneralCache::new(capacity, ttl) })
            .await
    }

    pub async fn get_open_list_cache(&self) -> &GeneralCache {
        let (capacity, ttl) = self.get_cache_settings().await;
        self.open_list_cache
            .get_or_init(|| async move { GeneralCache::new(capacity, ttl) })
            .await
    }

    pub async fn get_rate_limiter_cache(&self) -> &RateLimiterCache {
        self.rate_limiter_cache
            .get_or_init(|| async move {
                let config = self.get_config().await;
                let (capacity, ttl) = self.get_cache_settings().await;

                let (limit_kbs, burst_kbs) =
                    config.backend.as_ref().map_or((0, 0), |b| {
                        (b.client_speed_limit_kbs, b.client_burst_speed_kbs)
                    });

                RateLimiterCache::new(capacity * 2, ttl, limit_kbs, burst_kbs)
            })
            .await
    }

    /// Get backend routes with compiled regex patterns (cached at startup)
    pub async fn get_backend_routes(&self) -> Option<&BackendRoutes> {
        let config = self.get_config().await;
        if let Some(routes) = config.backend_routes.as_ref() {
            Some(
                self.backend_routes_cache
                    .get_or_init(|| async move {
                        let mut routes = routes.clone();
                        // Compile all regex patterns
                        for route in &mut routes.routes {
                            let pattern = route.pattern.clone();
                            route
                                .regex
                                .get_or_init(|| async {
                                    Regex::new(&pattern).expect(
                                        "Failed to compile regex pattern",
                                    )
                                })
                                .await;
                        }
                        routes
                    })
                    .await,
            )
        } else {
            None
        }
    }
}
