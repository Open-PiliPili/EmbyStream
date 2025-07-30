use std::path::PathBuf;

use hyper::Uri;

use super::proxy_mode::ProxyMode;

#[derive(Debug, Clone)]
pub(crate) enum Source {
    Local { path: PathBuf, device_id: String },
    Remote { uri: Uri, mode: ProxyMode },
}
