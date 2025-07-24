use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct HlsConfig {
    pub transcode_root_path: PathBuf,
    pub segment_duration_seconds: u32,
}

#[derive(Clone, PartialEq, Eq)]
pub enum HlsTranscodingStatus {
    InProgress,
    Completed,
    Failed,
}
