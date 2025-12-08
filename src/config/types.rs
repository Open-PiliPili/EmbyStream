use serde::Deserialize;

use crate::config::{
    backend::{
        Backend,
        direct::DirectLink,
        disk::Disk,
        openlist::OpenList,
        types::{
            BackendFallbackConfig, BackendRouteConfig, BackendRoutingConfig,
        },
    },
    frontend::Frontend,
    general::{Emby, General, Log, UserAgent},
    http2::Http2,
};

#[derive(Clone, Debug, Deserialize, Default)]
pub struct PathRewriteConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default)]
    pub pattern: String,
    #[serde(default)]
    pub replacement: String,
}

impl PathRewriteConfig {
    pub fn is_need_rewrite(&self, path: &str) -> bool {
        if path.is_empty() || !self.enable {
            return false;
        }
        !self.pattern.is_empty() && !self.replacement.is_empty()
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AntiReverseProxyConfig {
    #[serde(default)]
    pub enable: bool,
    #[serde(default, rename = "host")]
    pub trusted_host: String,
}

impl AntiReverseProxyConfig {
    #[inline]
    pub fn is_need_anti(&self, host: &str) -> bool {
        if !self.enable || self.trusted_host.is_empty() {
            return false;
        }

        fn extract_valid_host(url: &str) -> Option<&str> {
            let cleaned = url
                .trim_start_matches("http://")
                .trim_start_matches("https://");

            cleaned
                .split(['/', ':'])
                .next()
                .filter(|&s| !s.is_empty())
                .map(|s| s.trim_end_matches('/'))
        }

        match (
            extract_valid_host(host),
            extract_valid_host(&self.trusted_host),
        ) {
            (Some(request_host), Some(trusted_host)) => {
                !request_host.eq_ignore_ascii_case(trusted_host)
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct FallbackConfig {
    #[serde(default)]
    pub video_missing_path: String,
}

#[derive(Deserialize)]
pub struct RawConfig {
    #[serde(rename = "General")]
    pub general: General,
    #[serde(rename = "Log")]
    pub log: Log,
    #[serde(rename = "Emby")]
    pub emby: Emby,
    #[serde(rename = "UserAgent")]
    pub user_agent: UserAgent,
    #[serde(rename = "Http2")]
    pub http2: Option<Http2>,
    #[serde(rename = "Frontend")]
    pub frontend: Option<Frontend>,
    #[serde(rename = "Backend")]
    pub backend: Option<Backend>,
    /// Backend routing configuration
    #[serde(rename = "Backend.Routing", default)]
    pub backend_routing: Option<BackendRoutingConfig>,
    /// Backend route rules (matched in order)
    #[serde(rename = "Backend.Routes", default)]
    pub backend_routes: Vec<BackendRouteConfig>,
    /// Fallback backend configuration when no route matches
    #[serde(rename = "Backend.Fallback")]
    pub backend_fallback: Option<BackendFallbackConfig>,
    #[serde(rename = "Disk")]
    pub disk: Option<Disk>,
    #[serde(rename = "OpenList")]
    pub open_list: Option<OpenList>,
    #[serde(rename = "DirectLink")]
    pub direct_link: Option<DirectLink>,
    #[serde(rename = "Fallback", default)]
    pub fallback: FallbackConfig,
}
