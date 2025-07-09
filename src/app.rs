use tokio::sync::OnceCell;

use crate::{
    cache::{GeneralCache, FileCache}
};

pub struct AppState {
    file_cache: OnceCell<FileCache>,
    encrypt_cache: OnceCell<GeneralCache>,
    decrypt_cache: OnceCell<GeneralCache>,
}

impl AppState {
    pub async fn new() -> Self {
        Self {
            file_cache: OnceCell::new(),
            encrypt_cache: OnceCell::new(),
            decrypt_cache: OnceCell::new(),
        }
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
}
