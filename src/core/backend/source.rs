use reqwest::Url;
use std::path::PathBuf;

use super::proxy_mode::ProxyMode;

#[derive(Debug, Clone)]
pub(crate) enum Source {
    Local(PathBuf),
    Remote { url: Url, mode: ProxyMode },
}
