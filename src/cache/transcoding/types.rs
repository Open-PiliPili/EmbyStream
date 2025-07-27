use std::{path::PathBuf, sync::Arc, time::Instant};

use tokio::{process::Child, sync::Mutex};

#[derive(Clone, Debug)]
pub struct HlsConfig {
    pub transcode_root_path: PathBuf,
    pub segment_duration_seconds: u32,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HlsTranscodingStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Clone, Debug)]
pub struct TranscodingTask {
    pub status: HlsTranscodingStatus,
    pub last_accessed: Instant,
    pub process: Arc<Mutex<Child>>,
}
