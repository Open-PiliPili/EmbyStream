use hyper::Uri;
use serde::Deserialize;

use super::{
    direct::types::DirectLink, disk::types::Disk, openlist::types::OpenList,
};
use crate::config::types::{AntiReverseProxyConfig, PathRewriteConfig};

#[derive(Clone, Debug, Deserialize)]
pub struct Backend {
    pub listen_port: u16,
    pub base_url: String,
    pub path: String,
    pub port: String,
    pub proxy_mode: String,
    #[serde(default)]
    pub client_speed_limit_kbs: u64,
    #[serde(default)]
    pub client_burst_speed_kbs: u64,
    #[serde(default, rename = "PathRewrite")]
    pub path_rewrites: Vec<PathRewriteConfig>,
    #[serde(default, rename = "AntiReverseProxy")]
    pub anti_reverse_proxy: AntiReverseProxyConfig,
    #[serde(default)]
    pub problematic_clients: Vec<String>,
}

impl Backend {
    pub fn uri(&self) -> Uri {
        let should_show_port = !(self.port == "443" || self.port == "80");
        let clean_url = self.base_url.trim_end_matches('/');
        let clean_path =
            self.path.trim_start_matches("/").trim_end_matches('/');

        let uri_str = if should_show_port {
            format!("{}:{}/{}", clean_url, self.port, clean_path)
        } else {
            format!("{clean_url}/{clean_path}")
        };

        uri_str.parse().expect("Failed to parse backend URI")
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "backend_type", content = "settings")]
pub enum BackendConfig {
    Disk(Disk),
    OpenList(OpenList),
    DirectLink(DirectLink),
}
