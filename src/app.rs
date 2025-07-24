use std::{ops::Deref as DerefTrait, path::PathBuf, sync::Arc};

use dashmap::DashMap;
use directories::BaseDirs;
use tokio::{
    fs as TokioFs,
    sync::{OnceCell, RwLock as TokioRwLock},
};

use crate::hls::HlsTranscodingStatus;

use crate::{
    cache::{GeneralCache, MetadataCache},
    config::core::Config,
};

const CONFIG_DIR_NAME: &str = "embystream";
const ROOT_CONFIG_PATH: &str = "/root/.config/embystream";
const TRANSCODE_SUBDIR_NAME: &str = "transcode";

pub struct AppState {
    config: TokioRwLock<Config>,
    metadata_cache: OnceCell<MetadataCache>,
    encrypt_cache: OnceCell<GeneralCache>,
    decrypt_cache: OnceCell<GeneralCache>,
    strm_file_cache: OnceCell<GeneralCache>,
    forward_info_cache: OnceCell<GeneralCache>,
    open_list_cache: OnceCell<GeneralCache>,
    hls_info_cache: OnceCell<GeneralCache>,
    hls_path_cache: OnceCell<PathBuf>,
    hls_transcoding_cache:
        OnceCell<Arc<DashMap<PathBuf, HlsTranscodingStatus>>>,
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
            hls_info_cache: OnceCell::new(),
            hls_path_cache: OnceCell::new(),
            hls_transcoding_cache: OnceCell::new(),
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

    pub async fn get_hls_info_cache(&self) -> &GeneralCache {
        let (capacity, ttl) = self.get_cache_settings().await;
        self.hls_info_cache
            .get_or_init(|| async move { GeneralCache::new(capacity, ttl) })
            .await
    }

    pub async fn get_hls_transcoding_cache(
        &self,
    ) -> &Arc<DashMap<PathBuf, HlsTranscodingStatus>> {
        self.hls_transcoding_cache
            .get_or_init(|| async { Arc::new(DashMap::new()) })
            .await
    }

    pub async fn get_hls_path_cache(&self) -> &PathBuf {
        self.hls_path_cache
            .get_or_init(|| async {
                let config = self.get_config().await;
                let mut path =
                    PathBuf::from(config.general.transcode_root_path.clone());

                if path.as_os_str().is_empty() {
                    let base_dirs =
                        BaseDirs::new().expect("Could not find home directory");

                    let default_base = if cfg!(target_os = "linux")
                        && unsafe { libc::getuid() } == 0
                    {
                        PathBuf::from(ROOT_CONFIG_PATH)
                    } else if cfg!(target_os = "windows") {
                        base_dirs.config_dir().join(CONFIG_DIR_NAME)
                    } else {
                        // macOS and other Unix-like systems
                        base_dirs
                            .home_dir()
                            .join(".config")
                            .join(CONFIG_DIR_NAME)
                    };

                    path = default_base.join(TRANSCODE_SUBDIR_NAME);
                }

                if !path.exists() {
                    TokioFs::create_dir_all(&path)
                        .await
                        .expect("Failed to create HLS cache directory");
                }

                path.canonicalize().unwrap_or(path)
            })
            .await
    }
}
