use std::{sync::Arc, time::SystemTime};

use tokio::{
    fs::File as TokioFile,
    sync::RwLock as TokioRwLock
};

use crate::cache::file::Metadata as FileMetadata;

#[derive(Debug)]
pub struct Cached {
    pub file_handle: Arc<TokioRwLock<TokioFile>>,
    pub metadata: FileMetadata,
    pub reference_count: u64,
    pub last_accessed: SystemTime,
}
