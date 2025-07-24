use std::ops::Deref as DerefTrait;

use tokio::sync::{OnceCell, RwLock as TokioRwLock};

use crate::{
    cache::{GeneralCache, MetadataCache},
    config::core::Config,
};

pub struct AppState {
    config: TokioRwLock<Config>,
    metadata_cache: OnceCell<MetadataCache>,
    encrypt_cache: OnceCell<GeneralCache>,
    decrypt_cache: OnceCell<GeneralCache>,
    strm_file_cache: OnceCell<GeneralCache>,
    forward_info_cache: OnceCell<GeneralCache>,
    open_list_cache: OnceCell<GeneralCache>,
}

impl AppState {
    pub async fn new(config: Config) -> Self {
        Self {
            config: TokioRwLock::new(config),
            metadata_cache: OnceCell::new(),
            encrypt_cache: OnceCell::new(),
            decrypt_cache: OnceCell::new(),
            strm_file_cache: OnceCell::new(),
            forward_info_cache: OnceCell::new(),
            open_list_cache: OnceCell::new(),
        }
    }

    pub async fn get_config(&self) -> impl DerefTrait<Target = Config> + '_ {
        self.config.read().await
    }

    async fn get_cache_settings(&self) -> (u64, u64) {
        let config = self.get_config().await;
        match config.general.memory_mode.as_str() {
            "low" => (128, 30 * 30),
            "high" => (512, 60 * 60 * 2),
            _ => (256, 60 * 60),
        }
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
}
