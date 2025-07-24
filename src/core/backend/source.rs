use std::path::PathBuf;

use hyper::Uri;

use super::proxy_mode::ProxyMode;

#[derive(Debug, Clone)]
pub(crate) enum Source {
    Local { id: String, path: PathBuf },
    Remote { uri: Uri, mode: ProxyMode },
}
