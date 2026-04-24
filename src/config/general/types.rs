use std::fmt;

use hyper::Uri;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq,
)]
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

fn default_memory_mode_str() -> String {
    "middle".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct General {
    #[serde(default = "default_memory_mode_str")]
    pub memory_mode: String,
    #[serde(default)]
    pub stream_mode: StreamMode,
    pub encipher_key: String,
    pub encipher_iv: String,
}

fn default_log_level_str() -> String {
    "info".to_string()
}

fn default_logs_root_str() -> String {
    "./logs".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Log {
    #[serde(default = "default_log_level_str")]
    pub level: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default = "default_logs_root_str")]
    pub root_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Emby {
    pub url: String,
    pub port: String,
    #[serde(default)]
    pub token: String,
}

impl Emby {
    pub fn get_uri(&self) -> Uri {
        let should_show_port = !(self.port == "443" || self.port == "80");
        let clean_url = self.url.trim_end_matches('/');

        let uri_str = if should_show_port {
            format!("{}:{}", clean_url, self.port)
        } else {
            clean_url.to_string()
        };

        uri_str.parse().unwrap_or_else(|error| {
            eprintln!("Failed to parse Emby URI '{uri_str}': {error}");
            Uri::from_static("/")
        })
    }
}

fn default_user_agent_mode_str() -> String {
    "allow".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserAgent {
    #[serde(default = "default_user_agent_mode_str")]
    pub mode: String,
    #[serde(default)]
    pub allow_ua: Vec<String>,
    #[serde(default)]
    pub deny_ua: Vec<String>,
}

impl UserAgent {
    pub fn is_allow_mode(&self) -> bool {
        self.mode == "allow"
    }
}

impl fmt::Display for UserAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mode.as_str() {
            "allow" => write!(
                f,
                "Mode: {}, Allowed User Agents: [{}]",
                self.mode,
                self.allow_ua.join(", ")
            ),
            _ => write!(
                f,
                "Mode: {}, Denied User Agents: [{}]",
                self.mode,
                self.deny_ua.join(", ")
            ),
        }
    }
}
