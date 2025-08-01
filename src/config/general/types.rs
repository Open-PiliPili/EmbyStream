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
    pub memory_mode: String,
    #[serde(default)]
    pub stream_mode: StreamMode,
    pub backend_type: String,
    pub encipher_key: String,
    pub encipher_iv: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Log {
    pub level: String,
    pub prefix: String,
    pub root_path: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Emby {
    pub url: String,
    pub port: String,
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
