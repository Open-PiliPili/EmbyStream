use std::fmt;

use hyper::Uri;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StreamMode {
    #[default]
    Frontend,
    Backend,
    Dual,
}

impl fmt::Display for StreamMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamMode::Frontend => write!(f, "frontend"),
            StreamMode::Backend => write!(f, "backend"),
            StreamMode::Dual => write!(f, "dual"),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct General {
    pub log_level: String,
    pub memory_mode: String,
    pub expired_seconds: u64,
    #[serde(default)]
    pub stream_mode: StreamMode,
    pub backend_type: String,
    pub encipher_key: String,
    pub encipher_iv: String,
    pub emby_url: String,
    pub emby_port: String,
    pub emby_api_key: String,
}

impl General {
    pub fn emby_uri(&self) -> Uri {
        let should_show_port =
            !(self.emby_port == "443" || self.emby_port == "80");
        let clean_url = self.emby_url.trim_end_matches('/');

        let uri_str = if should_show_port {
            format!("{}:{}", clean_url, self.emby_port)
        } else {
            clean_url.to_string()
        };

        uri_str.parse().expect("Failed to parse backend URI")
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserAgent {
    pub mode: String,
    pub allow_ua: Vec<String>,
    pub deny_ua: Vec<String>,
}

impl UserAgent {
    pub fn is_allow_mode(&self) -> bool {
        self.mode == "allow"
    }
}
