use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::{sync::Mutex, task};

use super::{
    transcoder,
    types::{HlsConfig, HlsTranscodingStatus},
};
use crate::{
    AppState, HLS_LOGGER_DOMAIN, error_log, info_log, util::StringUtil,
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
        original_path: &Path,
    ) -> Result<PathBuf, String> {
        let manifest_path = self.get_manifest_path(original_path)?;
        let status_map = self.state.get_hls_transcoding_cache().await;

        if status_map
            .get(original_path)
            .is_some_and(|s| *s.value() == HlsTranscodingStatus::Completed)
        {
            return Ok(manifest_path);
        }

        let lock = TRANSCODING_LOCKS
            .entry(original_path.to_path_buf())
            .or_default()
            .clone();
        let _guard = lock.lock().await;

        if status_map
            .get(original_path)
            .is_some_and(|s| *s.value() == HlsTranscodingStatus::Completed)
        {
            return Ok(manifest_path);
        }

        status_map.insert(
            original_path.to_path_buf(),
            HlsTranscodingStatus::InProgress,
        );

        let path_clone = original_path.to_path_buf();
        let dir_clone = manifest_path.parent().unwrap().to_path_buf();
        let config_clone = self.config.clone();
        let status_map_clone = status_map.clone();

        task::spawn_blocking(move || {
            info_log!(
                HLS_LOGGER_DOMAIN,
                "Spawning HLS transmux for: {:?}",
                &path_clone
            );
            match transcoder::transmux_to_dash(
                &path_clone,
                &dir_clone,
                &config_clone,
            ) {
                Ok(_) => {
                    info_log!(
                        HLS_LOGGER_DOMAIN,
                        "HLS transmux completed for: {:?}",
                        &path_clone
                    );
                    status_map_clone
                        .insert(path_clone, HlsTranscodingStatus::Completed);
                }
                Err(e) => {
                    error_log!(
                        HLS_LOGGER_DOMAIN,
                        "HLS transmux failed for: {:?}, error: {}",
                        &path_clone,
                        e
                    );
                    status_map_clone
                        .insert(path_clone, HlsTranscodingStatus::Failed);
                    fs::remove_dir_all(&dir_clone).ok();
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
            format!(
                "File path contains invalid UTF-8 characters: {:?}",
                original_path
            )
        })?;

        if path_str.is_empty() {
            return Err("File path cannot be empty".to_string());
        }

        let file_hash = StringUtil::md5(path_str);
        Ok(self
            .config
            .transcode_root_path
            .join(file_hash)
            .join("playlist.mpd"))
    }
}
