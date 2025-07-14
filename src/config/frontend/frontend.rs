use serde::Deserialize;

use crate::config::types::PathRewriteConfig;

#[derive(Clone, Debug, Deserialize)]
pub struct Frontend {
    pub listen_port: u16,
    #[serde(default, rename = "PathRewrite")]
    pub path_rewrite: PathRewriteConfig,
}