use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use tokio::{fs::File as TokioFile, sync::RwLock as TokioRwLock};

use crate::cache::file::EntryState as CacheEntryState;

#[derive(Clone, Debug)]
pub struct Entry {
    pub handle: Arc<TokioRwLock<TokioFile>>,
    pub path: PathBuf,
    pub identifier: Uuid,
    pub state: Arc<TokioRwLock<CacheEntryState>>,
}

impl Entry {
    pub fn new(handle: TokioFile, path: PathBuf) -> Self {
        Entry {
            handle: Arc::new(TokioRwLock::new(handle)),
            path,
            identifier: Uuid::new_v4(),
            state: Arc::new(TokioRwLock::new(CacheEntryState::default())),
        }
    }
}
