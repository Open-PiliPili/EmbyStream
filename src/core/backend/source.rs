use std::path::PathBuf;

use reqwest::Url;

use super::proxy_mode::ProxyMode;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum Source {
    Local(PathBuf),
    Remote { url: Url, mode: ProxyMode },
}
