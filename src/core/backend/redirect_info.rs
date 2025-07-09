use http_serde;
use hyper::HeaderMap;
use hyper::Uri;
use serde::Serialize;

use crate::uri_serde;

#[derive(Debug, Serialize)]
pub struct RedirectInfo {
    #[serde(serialize_with = "uri_serde::serialize_uri_as_string")]
    pub target_url: Uri,

    #[serde(with = "http_serde::header_map")]
    pub final_headers: HeaderMap,
}
