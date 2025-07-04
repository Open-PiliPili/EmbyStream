use std::{
    path::PathBuf,
    sync::Arc
};

use tokio::{
    fs::File as TokioFile,
    sync::RwLock
};

use crate::cache::file::{Entry, Error};

#[derive(Debug)]
pub struct FileEntryPool {
    entries: RwLock<Vec<Entry>>,
    path: PathBuf,
}

impl FileEntryPool {
    pub fn new(path: PathBuf) -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            path,
        }
    }

    pub async fn get_or_create_entry(&self) -> Result<Entry, Error> {
        let read_guard = self.entries.read().await;
        for entry in read_guard.iter() {
            if Arc::strong_count(&entry.handle) == 1 {
                return Ok(entry.clone());
            }
        }
        drop(read_guard);

        let mut write_guard = self.entries.write().await;

        for entry in write_guard.iter() {
            if Arc::strong_count(&entry.handle) == 1 {
                return Ok(entry.clone());
            }
        }

        let file = TokioFile::open(&self.path)
            .await
            .map_err(|e| Error::IoError(Arc::new(e)))?;

        let new_entry = Entry::new(file, self.path.clone());
        write_guard.push(new_entry.clone());

        Ok(new_entry)
    }
}