use std::{path::PathBuf, sync::Arc};

use tokio::{fs::File as TokioFile, sync::RwLock};

#[derive(Clone, Debug)]
pub struct Entry {
    pub handle: Arc<RwLock<TokioFile>>,
    pub path: PathBuf,
}

impl Entry {
    pub fn new(handle: TokioFile, path: PathBuf) -> Self {
        Self {
            handle: Arc::new(RwLock::new(handle)),
            path,
        }
    }
}
