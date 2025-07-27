use std::{path::PathBuf, sync::Arc, time::Duration};

use moka::future::Cache;
use tokio::sync::RwLock as TokioRwLock;

use super::types::TranscodingTask;
use crate::{HLS_CACHE_LOGGER_DOMAIN, debug_log, error_log, info_log};

#[derive(Clone)]
pub struct TranscodingCache {
    inner: Cache<PathBuf, Arc<TokioRwLock<TranscodingTask>>>,
}

impl TranscodingCache {
    pub fn new(max_capacity: u64, time_to_idle_secs: u64) -> Self {
        let eviction_listener = |path: Arc<PathBuf>,
                                 task: Arc<TokioRwLock<TranscodingTask>>,
                                 _| {
            tokio::spawn(async move {
                let task_guard = task.read().await;
                let mut process_guard = task_guard.process.lock().await;
                if let Err(e) = process_guard.kill().await {
                    error_log!(
                        HLS_CACHE_LOGGER_DOMAIN,
                        "Failed to kill process for {:?}: {}",
                        path,
                        e
                    );
                } else {
                    debug_log!(
                        HLS_CACHE_LOGGER_DOMAIN,
                        "Successfully killed process for {:?}",
                        path
                    );
                }

                if let Some(dir) = task_guard.manifest_path.parent() {
                    if let Err(e) = tokio::fs::remove_dir_all(dir).await {
                        error_log!(
                            HLS_CACHE_LOGGER_DOMAIN,
                            "Failed to remove dir for {:?}, manifest dir: {:?}, error: {}",
                            path,
                            dir,
                            e
                        );
                    } else {
                        info_log!(
                            HLS_CACHE_LOGGER_DOMAIN,
                            "Successfully removed cached dir for {:?}, manifest dir: {:?}",
                            path,
                            dir
                        );
                    }
                }
            });
        };

        let inner = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_idle(Duration::from_secs(time_to_idle_secs))
            .eviction_listener(eviction_listener)
            .build();

        Self { inner }
    }

    pub async fn get(
        &self,
        path: &PathBuf,
    ) -> Option<Arc<TokioRwLock<TranscodingTask>>> {
        self.inner.get(path).await
    }

    pub async fn insert(
        &self,
        path: PathBuf,
        task: Arc<TokioRwLock<TranscodingTask>>,
    ) {
        self.inner.insert(path, task).await
    }
}
