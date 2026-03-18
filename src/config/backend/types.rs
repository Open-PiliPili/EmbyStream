use hyper::Uri;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::{
    direct::types::DirectLink, disk::types::Disk, openlist::types::OpenList,
};
use crate::{
    config::types::{AntiReverseProxyConfig, PathRewriteConfig},
    util::path_rewriter::PathRewriter,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Backend {
    pub listen_port: u16,
    pub base_url: String,
    pub port: String,
    pub path: String,
    #[serde(default)]
    pub problematic_clients: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BackendNode {
    pub name: String,
    #[serde(rename = "type")]
    pub backend_type: String,
    #[serde(default)]
    pub pattern: String,
    #[serde(skip)]
    pub pattern_regex: Option<Regex>,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub port: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_proxy_mode")]
    pub proxy_mode: String,
    #[serde(default)]
    pub client_speed_limit_kbs: u64,
    #[serde(default)]
    pub client_burst_speed_kbs: u64,
    #[serde(default, rename = "PathRewrite")]
    pub path_rewrites: Vec<PathRewriteConfig>,
    #[serde(default, rename = "AntiReverseProxy")]
    pub anti_reverse_proxy: AntiReverseProxyConfig,
    #[serde(skip)]
    pub path_rewriter_cache: Vec<PathRewriter>,
    #[serde(skip)]
    pub uuid: String,
    #[serde(rename = "Disk")]
    pub disk: Option<Disk>,
    #[serde(rename = "OpenList")]
    pub open_list: Option<OpenList>,
    #[serde(rename = "DirectLink")]
    pub direct_link: Option<DirectLink>,
}

macro_rules! impl_uri {
    ($t:ty) => {
        impl $t {
            pub fn uri(&self) -> Uri {
                if self.base_url.is_empty() {
                    return "/".parse().expect("Failed to parse fallback URI");
                }

                let should_show_port = !self.port.is_empty()
                    && self.port != "443"
                    && self.port != "80";
                let clean_url = self.base_url.trim_end_matches('/');
                let clean_path =
                    self.path.trim_start_matches('/').trim_end_matches('/');

                let uri_str = if should_show_port {
                    format!("{}:{}/{}", clean_url, self.port, clean_path)
                } else if clean_path.is_empty() {
                    clean_url.to_string()
                } else {
                    format!("{}/{}", clean_url, clean_path)
                };

                uri_str.parse().expect("Failed to parse backend URI")
            }
        }
    };
}

impl_uri!(Backend);
impl_uri!(BackendNode);

fn default_proxy_mode() -> String {
    "proxy".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "backend_type", content = "settings")]
pub enum BackendConfig {
    Disk(Disk),
    OpenList(OpenList),
    DirectLink(DirectLink),
}
