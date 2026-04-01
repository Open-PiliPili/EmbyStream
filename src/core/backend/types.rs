use std::{fs::File as StdFile, path::PathBuf};

use crate::cache::FileMetadata;
use crate::config::backend::{
    Backend, types::BackendConfig as StreamBackendConfig,
};

#[derive(Clone, Debug)]
pub struct BackendConfig {
    pub crypto_key: String,
    pub crypto_iv: String,
    pub backend: Backend,
    pub backend_config: StreamBackendConfig,
    pub fallback_video_path: Option<String>,
}

#[derive(Debug)]
pub struct PreparedLocalStreamTarget {
    pub path: PathBuf,
    pub file_metadata: FileMetadata,
    pub opened_file: Option<StdFile>,
    pub is_fallback: bool,
}

impl PreparedLocalStreamTarget {
    pub fn new(path: PathBuf, file_metadata: FileMetadata) -> Self {
        Self {
            path,
            file_metadata,
            opened_file: None,
            is_fallback: false,
        }
    }

    pub fn with_opened_file(mut self, opened_file: StdFile) -> Self {
        self.opened_file = Some(opened_file);
        self
    }

    pub fn with_fallback(mut self, is_fallback: bool) -> Self {
        self.is_fallback = is_fallback;
        self
    }

    pub fn has_opened_file(&self) -> bool {
        self.opened_file.is_some()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ContentRange {
    pub(crate) start: u64,
    pub(crate) end: u64,
    pub(crate) total_size: u64,
}

impl ContentRange {
    pub fn length(&self) -> u64 {
        self.end - self.start + 1
    }

    pub fn is_full_range(&self) -> bool {
        self.start == 0 && self.end >= self.total_size.saturating_sub(1)
    }
}

#[derive(Debug)]
pub enum RangeParseError {
    Malformed,
    Unsatisfiable,
}

pub struct ClientInfo {
    pub(crate) id: Option<String>,
    pub(crate) user_agent: Option<String>,
    pub(crate) ip: Option<String>,
}

impl ClientInfo {
    pub fn new(
        id: Option<String>,
        user_agent: Option<String>,
        ip: Option<String>,
    ) -> ClientInfo {
        Self { id, user_agent, ip }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File as StdFile, path::PathBuf, time::SystemTime};

    use tempfile::NamedTempFile;

    use super::PreparedLocalStreamTarget;
    use crate::cache::FileMetadata;

    fn sample_metadata() -> FileMetadata {
        FileMetadata {
            file_size: 123,
            file_name: "episode.mkv".to_string(),
            format: "mkv".to_string(),
            last_modified: None,
            updated_at: SystemTime::now(),
        }
    }

    #[test]
    fn prepared_local_stream_target_defaults_to_primary_without_file() {
        let target = PreparedLocalStreamTarget::new(
            PathBuf::from("/mnt/media/episode.mkv"),
            sample_metadata(),
        );

        assert_eq!(target.path, PathBuf::from("/mnt/media/episode.mkv"));
        assert_eq!(target.file_metadata.file_size, 123);
        assert!(!target.is_fallback);
        assert!(!target.has_opened_file());
    }

    #[test]
    fn prepared_local_stream_target_can_mark_fallback() {
        let target = PreparedLocalStreamTarget::new(
            PathBuf::from("/mnt/fallback/video_missing.mp4"),
            sample_metadata(),
        )
        .with_fallback(true);

        assert!(target.is_fallback);
        assert_eq!(
            target.path,
            PathBuf::from("/mnt/fallback/video_missing.mp4")
        );
    }

    #[test]
    fn prepared_local_stream_target_can_carry_opened_file() {
        let temp = NamedTempFile::new().expect("temp file");
        let file = StdFile::open(temp.path()).expect("open temp file");

        let target = PreparedLocalStreamTarget::new(
            temp.path().to_path_buf(),
            sample_metadata(),
        )
        .with_opened_file(file);

        assert!(target.has_opened_file());
    }
}
