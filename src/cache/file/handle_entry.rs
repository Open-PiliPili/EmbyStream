use std::{sync::Arc, time::SystemTime};

use tokio::{fs::File as TokioFile, sync::RwLock as TokioRwLock};

#[derive(Debug, Clone)]
pub struct HandleEntry {
    pub file_handle: Arc<TokioRwLock<TokioFile>>,
    pub in_use: bool,
    pub last_accessed: SystemTime,
}

impl HandleEntry {
    pub fn new(file: TokioFile) -> Self {
        HandleEntry {
            file_handle: Arc::new(TokioRwLock::new(file)),
            in_use: false,
            last_accessed: SystemTime::now(),
        }
    }
}
