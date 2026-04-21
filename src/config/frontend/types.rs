use serde::{Deserialize, Serialize};

use crate::config::types::{AntiReverseProxyConfig, PathRewriteConfig};

fn default_check_file_existence() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Frontend {
    pub listen_port: u16,

    #[serde(default = "default_check_file_existence")]
    pub check_file_existence: bool,

    #[serde(default, rename = "PathRewrite")]
    pub path_rewrites: Vec<PathRewriteConfig>,

    #[serde(default, rename = "AntiReverseProxy")]
    pub anti_reverse_proxy: AntiReverseProxyConfig,
}
