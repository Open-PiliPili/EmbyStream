use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub file_size: u64,
    pub file_name: String,
    pub format: String,
    pub last_modified: Option<SystemTime>,
    pub updated_at: SystemTime,
}

impl FileMetadata {
    pub fn is_valid(&self) -> bool {
        self.file_size > 0
    }
}
