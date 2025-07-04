use tokio::sync::OnceCell;

use crate::cache::FileCache;

pub struct AppState {
    file_cache: OnceCell<FileCache>,
}

impl AppState {
    pub async fn new() -> Self {
        Self {
            file_cache: OnceCell::new()
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
}
