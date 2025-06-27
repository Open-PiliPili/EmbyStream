use std::{path::PathBuf, sync::Arc, time::SystemTime};

use tokio::{fs::File as TokioFile, sync::RwLock as TokioRwLock};

use crate::cache::file::{HandleEntry as FileHandleEntry, Metadata as FileMetadata};
use crate::{FILE_CACHE_LOGGER_DOMAIN, debug_log, error_log};

#[derive(Debug)]
pub struct Cached {
    pub file_handles: Vec<FileHandleEntry>,
    pub metadata: FileMetadata,
}

impl Cached {
    pub fn new(metadata: FileMetadata) -> Self {
        Cached {
            file_handles: Vec::new(),
            metadata,
        }
    }

    pub async fn get_available_handle(
        &mut self,
        path: &PathBuf,
    ) -> Option<Arc<TokioRwLock<TokioFile>>> {
        let mut reused_handle = None;
        for entry in &mut self.file_handles {
            if !entry.in_use {
                entry.in_use = true;
                entry.last_accessed = SystemTime::now();
                reused_handle = Some(Arc::clone(&entry.file_handle));
                break;
            }
        }

        if let Some(handle) = reused_handle {
            debug_log!(
                FILE_CACHE_LOGGER_DOMAIN,
                "Reused existing handle for path={:?}, pool_size={}",
                path,
                self.file_handles.len()
            );
            return Some(handle);
        }

        match TokioFile::open(path).await {
            Ok(file) => {
                let mut new_entry = FileHandleEntry::new(file);
                new_entry.in_use = true;
                let new_handle = Arc::clone(&new_entry.file_handle);
                self.file_handles.push(new_entry);
                debug_log!(
                    FILE_CACHE_LOGGER_DOMAIN,
                    "Created new handle for path={:?}, pool_size={}",
                    path,
                    self.file_handles.len()
                );

                self.evict_if_needed(path);
                Some(new_handle)
            }
            Err(e) => {
                error_log!(
                    FILE_CACHE_LOGGER_DOMAIN,
                    "Failed to create new handle for path={:?}: {}",
                    path,
                    e
                );
                None
            }
        }
    }

    #[allow(unused_variables)]
    pub fn evict_if_needed(&mut self, path: &PathBuf) {}
}
