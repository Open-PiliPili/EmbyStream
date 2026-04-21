use hyper::Uri;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::{
    direct::types::DirectLink, disk::types::Disk,
    google_drive::GoogleDriveConfig, openlist::types::OpenList,
    webdav::WebDavConfig,
};
use crate::{
    config::types::{AntiReverseProxyConfig, PathRewriteConfig},
    util::path_rewriter::PathRewriter,
};

fn default_check_file_existence() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Backend {
    pub listen_port: u16,
    pub base_url: String,
    pub port: String,
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_check_file_existence")]
    pub check_file_existence: bool,
    #[serde(default)]
    pub problematic_clients: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BackendNode {
    pub name: String,
    #[serde(rename = "backend_type", alias = "type")]
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
    #[serde(default, rename = "path_rewrites", alias = "PathRewrite")]
    pub path_rewrites: Vec<PathRewriteConfig>,
    #[serde(
        default,
        rename = "anti_reverse_proxy",
        alias = "AntiReverseProxy"
    )]
    pub anti_reverse_proxy: AntiReverseProxyConfig,
    #[serde(skip)]
    pub path_rewriter_cache: Vec<PathRewriter>,
    #[serde(skip)]
    pub uuid: String,
    #[serde(rename = "disk", alias = "Disk")]
    pub disk: Option<Disk>,
    #[serde(rename = "open_list", alias = "OpenList")]
    pub open_list: Option<OpenList>,
    #[serde(rename = "direct_link", alias = "DirectLink")]
    pub direct_link: Option<DirectLink>,
    #[serde(rename = "google_drive", alias = "GoogleDrive")]
    pub google_drive: Option<GoogleDriveConfig>,
    #[serde(rename = "webdav", alias = "WebDav")]
    pub webdav: Option<WebDavConfig>,
}

macro_rules! impl_uri {
    ($t:ty) => {
        impl $t {
            pub fn uri(&self) -> Uri {
                if self.base_url.is_empty() {
                    return Uri::from_static("/");
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

                uri_str
                    .parse::<Uri>()
                    .unwrap_or_else(|_| Uri::from_static("/"))
            }
        }
    };
}

impl_uri!(Backend);
impl_uri!(BackendNode);

fn default_proxy_mode() -> String {
    "redirect".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "backend_type", content = "settings")]
pub enum BackendConfig {
    Disk(Disk),
    OpenList(OpenList),
    DirectLink(DirectLink),
    GoogleDrive(GoogleDriveConfig),
}
