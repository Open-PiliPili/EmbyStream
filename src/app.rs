use tokio::sync::RwLock as TokioRwLock;

use crate::cache::FileCache;

pub struct AppState {
    pub file_cache: TokioRwLock<FileCache>,
}

impl AppState {
    pub async fn new() -> Self {
        let file_cache = FileCache::builder()
            .with_max_alive_seconds(60 * 60)
            .with_clean_interval(30 * 60)
            .build()
            .await;
        Self {
            file_cache: TokioRwLock::new(file_cache),
        }
    }
}
