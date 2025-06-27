use std::fmt;

use serde::Deserialize;

/// Configuration for the DirectLink backend.
#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub base_url: String,
    #[serde(default = "default_stream_port")]
    pub stream_port: String,
    #[serde(default)]
    pub path_replace_rule_regex: String,
}

// Default stream port value.
fn default_stream_port() -> String {
    "443".to_string()
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DirectLinkConfig {{ base_url: {}, stream_port: {}, path_replace_rule_regex: {} }}",
            self.base_url, self.stream_port, self.path_replace_rule_regex
        )
    }
}
