use hyper::Uri;
use serde::Deserialize;

use crate::config::types::{AntiReverseProxyConfig, PathRewriteConfig};

#[derive(Clone, Debug, Deserialize)]
pub struct Backend {
    pub listen_port: u16,
    pub base_url: String,
    pub path: String,
    pub port: String,
    pub proxy_mode: String,
    #[serde(default, rename = "PathRewrite")]
    pub path_rewrite: PathRewriteConfig,
    #[serde(default, rename = "AntiReverseProxy")]
    pub anti_reverse_proxy: AntiReverseProxyConfig,
}

impl Backend {
    pub fn uri(&self) -> Uri {
        let should_show_port = !(self.port == "443" || self.port == "80");
        let clean_url = self.base_url.trim_end_matches('/');
        let clean_path = self.path.trim_start_matches("/").trim_end_matches('/');

        let uri_str = if should_show_port {
            format!("{}:{}/{}", clean_url, self.port, clean_path)
        } else {
            format!("{}/{}", clean_url, clean_path)
        };

        uri_str.parse().expect("Failed to parse backend URI")
    }
}
