use serde::Deserialize;

use crate::config::types::{AntiReverseProxyConfig, PathRewriteConfig};

use crate::defaults;

#[derive(Clone, Debug, Deserialize)]
pub struct Frontend {
    pub listen_port: u16,

    #[serde(default = "defaults::default_true")]
    pub check_file_existence: bool,

    #[serde(default, rename = "PathRewrite")]
    pub path_rewrites: Vec<PathRewriteConfig>,

    #[serde(default, rename = "AntiReverseProxy")]
    pub anti_reverse_proxy: AntiReverseProxyConfig,
}
