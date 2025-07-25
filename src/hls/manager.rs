use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::{
    io::{AsyncReadExt, BufReader as TokioBufReader},
    sync::{Mutex, RwLock as TokioRwLock},
};

use super::{codec, playlist};
use crate::{
    AppState, HLS_STREAM_LOGGER_DOMAIN,
    cache::transcoding::{HlsConfig, HlsTranscodingStatus, TranscodingTask},
    error_log, info_log,
    util::StringUtil,
};

lazy_static! {
    static ref TRANSCODING_LOCKS: DashMap<PathBuf, Arc<Mutex<()>>> =
        DashMap::new();
}

pub struct HlsManager {
    state: Arc<AppState>,
    config: HlsConfig,
}

impl HlsManager {
    pub fn new(state: Arc<AppState>, config: HlsConfig) -> Self {
        Self { state, config }
    }

    pub async fn ensure_stream(
        &self,
        original_path: &PathBuf,
    ) -> Result<PathBuf, String> {
        let manifest_path = self.get_manifest_path(original_path)?;
        let output_dir = manifest_path.parent().unwrap().to_path_buf();
        let status_cache = self.state.get_hls_transcoding_cache().await;

        if let Some(task_lock) = status_cache.get(original_path).await {
            let task = task_lock.read().await;
            if task.status == HlsTranscodingStatus::Completed {
                return Ok(manifest_path.clone());
            }
        }

        let lock = TRANSCODING_LOCKS
            .entry(original_path.to_path_buf())
            .or_default()
            .clone();
        let _guard = lock.lock().await;

        if let Some(task_lock) = status_cache.get(original_path).await {
            let task = task_lock.read().await;
            if task.status != HlsTranscodingStatus::Failed
                && manifest_path.exists()
            {
                return Ok(manifest_path.clone());
            }
        }

        playlist::generate_m3u8_playlist(
            original_path,
            &output_dir,
            &self.config,
        )
        .await?;

        let mut child_process = codec::transmux_to_hls_segments(
            original_path,
            &output_dir,
            &self.config,
        )
        .await?;

        let stderr = child_process
            .stderr
            .take()
            .expect("Failed to capture stderr");

        let new_task = Arc::new(TokioRwLock::new(TranscodingTask {
            status: HlsTranscodingStatus::InProgress,
            last_accessed: Instant::now(),
            process: Arc::new(Mutex::new(child_process)),
        }));

        status_cache
            .insert(original_path.to_path_buf(), new_task.clone())
            .await;

        let path_clone_for_status_update = original_path.to_path_buf();
        tokio::spawn(async move {
            let process_arc = {
                let task_read = new_task.read().await;
                task_read.process.clone()
            };

            let mut reader = TokioBufReader::new(stderr);
            let mut stderr_output = String::new();

            reader.read_to_string(&mut stderr_output).await.ok();

            let status = {
                let mut process_guard = process_arc.lock().await;
                process_guard.wait().await
            };

            let mut task_write = new_task.write().await;
            match status {
                Ok(exit_status) if exit_status.success() => {
                    task_write.status = HlsTranscodingStatus::Completed;
                    info_log!(
                        HLS_STREAM_LOGGER_DOMAIN,
                        "HLS transmux completed for: {:?}",
                        &path_clone_for_status_update
                    );
                }
                Ok(exit_status) => {
                    task_write.status = HlsTranscodingStatus::Failed;
                    error_log!(
                        HLS_STREAM_LOGGER_DOMAIN,
                        "HLS transmux failed for: {:?} with status {}. Stderr:\n{}",
                        &path_clone_for_status_update,
                        exit_status,
                        stderr_output
                    );
                }
                Err(e) => {
                    task_write.status = HlsTranscodingStatus::Failed;
                    error_log!(
                        HLS_STREAM_LOGGER_DOMAIN,
                        "HLS process wait failed for: {:?}, error: {}",
                        &path_clone_for_status_update,
                        e
                    );
                }
            }
        });

        Ok(manifest_path)
    }

    pub fn get_manifest_path(
        &self,
        original_path: &Path,
    ) -> Result<PathBuf, String> {
        let path_str = original_path.to_str().ok_or_else(|| {
            format!("Invalid UTF-8 in path: {:?}", original_path)
        })?;
        if path_str.is_empty() {
            return Err("File path is empty".to_string());
        }
        let file_hash = StringUtil::md5(path_str);
        Ok(self
            .config
            .transcode_root_path
            .join(file_hash)
            .join("master.m3u8"))
    }
}
