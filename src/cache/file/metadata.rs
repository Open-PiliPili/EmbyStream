use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct Metadata {
    pub file_size: u64,
    pub format: String,
    pub last_modified: Option<SystemTime>,
    pub updated_at: SystemTime
}

impl Metadata {

    pub fn is_valid(&self) -> bool {
        self.file_size > 0
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            file_size: 0,
            format: "unknown".to_string(),
            last_modified: None,
            updated_at: SystemTime::now()
        }
    }
}