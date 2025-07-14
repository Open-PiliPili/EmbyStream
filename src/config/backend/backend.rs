use hyper::Uri;
use serde::Deserialize;

use crate::config::types::PathRewriteConfig;

#[derive(Clone, Debug, Deserialize)]
pub struct Backend {
    pub listen_port: u16,
    pub base_url: String,
    pub path: String,
    pub port: String,
    pub proxy_mode: String,
    #[serde(default, rename = "PathRewrite")]
    pub path_rewrite: PathRewriteConfig,
}

impl Backend {

    pub fn uri(&self) -> Uri {
        let scheme = if self.port == "443" { "https" } else { "http" };
        let should_show_port = !(self.port == "443" || self.port == "80");
        let clean_url = self.base_url
            .trim_start_matches("//")
            .trim_end_matches('/');

        let uri_str = if should_show_port {
            format!(
                "{}://{}:{}/{}",
                scheme,
                clean_url,
                self.port,
                self.path.trim_start_matches('/')
            )
        } else {
            format!(
                "{}://{}/{}",
                scheme,
                clean_url,
                self.path.trim_start_matches('/')
            )
        };

        uri_str.parse().expect("Failed to parse backend URI")
    }
}