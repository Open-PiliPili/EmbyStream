use hyper::Uri;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OpenList {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub port: String,
    #[serde(default)]
    pub token: String,
}

impl OpenList {
    pub fn uri(&self) -> Uri {
        let should_show_port = !(self.port == "443" || self.port == "80");
        let clean_url = self.base_url.trim_end_matches('/');

        let uri_str = if should_show_port {
            format!("{}:{}", clean_url, self.port)
        } else {
            clean_url.to_string()
        };

        uri_str.parse().unwrap_or_else(|error| {
            eprintln!("Failed to parse OpenList URI '{uri_str}': {error}");
            Uri::from_static("/")
        })
    }
}
