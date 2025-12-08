use hyper::Uri;
use serde::Deserialize;

use super::{
    direct::types::DirectLink, disk::types::Disk, openlist::types::OpenList,
};
use crate::config::types::{AntiReverseProxyConfig, PathRewriteConfig};

/// Configuration for backend routing behavior
#[derive(Clone, Debug, Deserialize)]
pub struct BackendRoutingConfig {
    /// Enable backend routing (default: false)
    #[serde(default = "default_enable")]
    pub enable: bool,
    /// Match routes before path rewriting (default: false)
    #[serde(default = "default_match_before_rewrite")]
    pub match_before_rewrite: bool,
    /// Match priority: "first" or "last" (default: "first")
    #[serde(default = "default_match_priority")]
    pub match_priority: String,
}

fn default_enable() -> bool {
    false
}

fn default_match_before_rewrite() -> bool {
    false
}

fn default_match_priority() -> String {
    "first".to_string()
}

/// Configuration for a single backend route rule
#[derive(Clone, Debug, Deserialize)]
pub struct BackendRouteConfig {
    /// Enable this route rule (default: false)
    #[serde(default = "default_enable")]
    pub enable: bool,
    /// Regex pattern to match against request path
    pub pattern: String,
    /// Backend type to use when pattern matches: "disk", "openlist", or "direct_link"
    pub backend_type: String,
}

/// Configuration for fallback backend when no route matches
#[derive(Clone, Debug, Deserialize)]
pub struct BackendFallbackConfig {
    /// Enable fallback backend (default: false)
    #[serde(default = "default_enable")]
    pub enable: bool,
    /// Backend type to use as fallback: "disk", "openlist", or "direct_link"
    pub backend_type: String,
}

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
    /// Backend routing configuration (Backend.Routing)
    #[serde(default, rename = "Routing")]
    pub routing: Option<BackendRoutingConfig>,
    /// Backend route rules (Backend.Routes)
    #[serde(default, rename = "Routes")]
    pub routes: Vec<BackendRouteConfig>,
    /// Fallback backend configuration (Backend.Fallback)
    #[serde(rename = "Fallback")]
    pub fallback: Option<BackendFallbackConfig>,
}

/// Get backend type string from BackendConfig enum
pub fn backend_type_str(config: &BackendConfig) -> &'static str {
    match config {
        BackendConfig::Disk(_) => "disk",
        BackendConfig::OpenList(_) => "openlist",
        BackendConfig::DirectLink(_) => "direct_link",
    }
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
