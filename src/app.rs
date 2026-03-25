use std::{collections::HashSet, ops::Deref as DerefTrait, sync::Arc};

use dashmap::DashMap;
use tokio::sync::{Mutex as TokioMutex, OnceCell, RwLock as TokioRwLock};

use crate::{
    cache::{GeneralCache, MetadataCache, RateLimiterCache},
    config::core::Config,
    core::backend::constants::DISK_BACKEND_TYPE,
    util::path_rewriter::PathRewriter,
};

// These constants define the user agent substrings for clients that require
// a workaround for missing Range headers.
const PROBLEMATIC_CLIENTS: &[&str] =
    &["yamby", "hills", "embytolocalplayer", "Emby/"];

pub struct AppState {
    config: TokioRwLock<Config>,
    frontend_path_rewrite_cache: OnceCell<Vec<PathRewriter>>,
    problematic_clients_cache: OnceCell<Vec<String>>,
    metadata_cache: OnceCell<MetadataCache>,
    encrypt_cache: OnceCell<GeneralCache>,
    decrypt_cache: OnceCell<GeneralCache>,
    strm_file_cache: OnceCell<GeneralCache>,
    forward_info_cache: OnceCell<GeneralCache>,
    open_list_cache: OnceCell<GeneralCache>,
    api_response_cache: OnceCell<GeneralCache>,
    rate_limiter_cache: OnceCell<DashMap<String, RateLimiterCache>>,
    pub(crate) webdav_auth_cache: DashMap<String, String>,
    pub(crate) webdav_auth_probe_locks: DashMap<String, Arc<TokioMutex<()>>>,
}

impl AppState {
    pub async fn new(config: Config) -> Self {
        Self {
            config: TokioRwLock::new(config),
            frontend_path_rewrite_cache: OnceCell::new(),
            problematic_clients_cache: OnceCell::new(),
            metadata_cache: OnceCell::new(),
            encrypt_cache: OnceCell::new(),
            decrypt_cache: OnceCell::new(),
            strm_file_cache: OnceCell::new(),
            forward_info_cache: OnceCell::new(),
            open_list_cache: OnceCell::new(),
            api_response_cache: OnceCell::new(),
            rate_limiter_cache: OnceCell::new(),
            webdav_auth_cache: DashMap::new(),
            webdav_auth_probe_locks: DashMap::new(),
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

    pub async fn get_api_response_cache(&self) -> &GeneralCache {
        self.api_response_cache
            .get_or_init(|| async move {
                let config = self.get_config().await;
                let (max_capacity, default_ttl) =
                    match config.general.memory_mode.as_str() {
                        "low" => (2048, 60 * 60 * 2),
                        "high" => (8192, 60 * 60 * 4),
                        _ => (4096, 60 * 60 * 2),
                    };
                GeneralCache::new(max_capacity, default_ttl)
            })
            .await
    }

    pub async fn get_rate_limiter_cache(
        &self,
        node_uuid: &str,
    ) -> Option<RateLimiterCache> {
        let cache_map = self
            .rate_limiter_cache
            .get_or_init(|| async move {
                let config = self.get_config().await;
                let (capacity, ttl) = self.get_cache_settings().await;
                let map = DashMap::new();

                // Per-client byte limiting is only applied in `LocalStreamer` (Disk → local file).
                // WebDAV / OpenList / DirectLink / StreamRelay proxy paths do not use this cache.
                for node in &config.backend_nodes {
                    if !node
                        .backend_type
                        .eq_ignore_ascii_case(DISK_BACKEND_TYPE)
                    {
                        continue;
                    }
                    let cache = RateLimiterCache::new(
                        capacity * 2,
                        ttl,
                        node.client_speed_limit_kbs,
                        node.client_burst_speed_kbs,
                    );
                    cache.start_refill_task();
                    map.insert(node.uuid.clone(), cache);
                }

                map
            })
            .await;

        cache_map.get(node_uuid).map(|r| r.value().clone())
    }
    pub async fn init_rate_limiters(&self) {
        self.get_rate_limiter_cache("").await;
    }
}
