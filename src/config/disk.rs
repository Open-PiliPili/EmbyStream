use std::fmt;

use serde::Deserialize;

/// Configuration for the Disk backend.
#[derive(Deserialize, Clone, Debug)]
pub struct DiskConfig {
    pub listen_port: u16,
    pub stream_url: String,
    #[serde(default = "default_stream_port")]
    pub stream_port: String,
    pub storage_base_path: Option<String>,
    #[serde(default)]
    pub path_replace_rule_regex: String,
}

// Default stream port value.
fn default_stream_port() -> String {
    "443".to_string()
}

impl fmt::Display for DiskConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DiskConfig {{ listen_port: {}, stream_url: {}, stream_port: {}, storage_base_path: {}, path_replace_rule_regex: {} }}",
            self.listen_port,
            self.stream_url,
            self.stream_port,
            self.storage_base_path.as_ref().map_or("None", |s| s),
            self.path_replace_rule_regex
        )
    }
}