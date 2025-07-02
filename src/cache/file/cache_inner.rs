use std::{collections::VecDeque, path::PathBuf, sync::Arc};

use tokio::sync::RwLock as TokioRwLock;

use crate::cache::file::{Entry as FileEntry, Metadata as FileMetadata};

#[derive(Debug)]
pub struct CacheInner {
    pub entries: Arc<TokioRwLock<Vec<FileEntry>>>,
    pub metadata: Arc<TokioRwLock<Option<FileMetadata>>>,
    pub order: Arc<TokioRwLock<VecDeque<PathBuf>>>,
}

impl CacheInner {
    pub fn new(metadata: Option<FileMetadata>) -> Self {
        CacheInner {
            entries: Arc::new(TokioRwLock::new(Vec::new())),
            metadata: Arc::new(TokioRwLock::new(metadata)),
            order: Arc::new(TokioRwLock::new(VecDeque::new())),
        }
    }
}
