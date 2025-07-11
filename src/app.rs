use std::ops::Deref as DerefTrait;

use tokio::sync::{OnceCell, RwLock as TokioRwLock};

use crate::{
    cache::{FileCache, GeneralCache},
    config::config::Config,
    error::Error,
};

pub struct AppState {
    config: TokioRwLock<Config>,
    file_cache: OnceCell<FileCache>,
    encrypt_cache: OnceCell<GeneralCache>,
    decrypt_cache: OnceCell<GeneralCache>,
    strm_file_cache: OnceCell<GeneralCache>,
    forward_info_cache: OnceCell<GeneralCache>,
}

impl AppState {
    pub async fn new(config: Config) -> Self {
        Self {
            config: TokioRwLock::new(config),
            file_cache: OnceCell::new(),
            encrypt_cache: OnceCell::new(),
            decrypt_cache: OnceCell::new(),
            strm_file_cache: OnceCell::new(),
            forward_info_cache: OnceCell::new(),
        }
    }

    pub async fn reload_config(&self, new_config: Config) {
        *self.config.write().await = new_config;
    }

    pub async fn get_config(&self) -> impl DerefTrait<Target = Config> + '_ {
        self.config.read().await
    }

    pub async fn full_reload(&self) -> Result<(), Error> {
        let new_config =
            Config::load_or_init().map_err(|e| Error::LoadConfigError(e.to_string()))?;
        self.reload_config(new_config).await;
        Ok(())
    }

    pub async fn get_file_cache(&self) -> &FileCache {
        self.file_cache
            .get_or_init(|| async {
                let cache = FileCache::new(256, 60 * 60);
                cache
            })
            .await
    }

    pub async fn get_encrypt_cache(&self) -> &GeneralCache {
        self.encrypt_cache
            .get_or_init(|| async {
                let cache = GeneralCache::new(256, 60 * 60);
                cache
            })
            .await
    }

    pub async fn get_decrypt_cache(&self) -> &GeneralCache {
        self.decrypt_cache
            .get_or_init(|| async {
                let cache = GeneralCache::new(256, 60 * 60);
                cache
            })
            .await
    }

    pub async fn get_strm_file_cache(&self) -> &GeneralCache {
        self.strm_file_cache
            .get_or_init(|| async {
                let cache = GeneralCache::new(256, 60 * 60);
                cache
            })
            .await
    }

    pub async fn get_forward_info_cache(&self) -> &GeneralCache {
        self.forward_info_cache
            .get_or_init(|| async {
                let cache = GeneralCache::new(256, 60 * 60);
                cache
            })
            .await
    }
}
