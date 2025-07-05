use hyper::Uri;
use hyper::HeaderMap;
use serde::{Serialize, Serializer};
use http_serde;

fn serialize_uri_as_string<S>(uri: &Uri, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&uri.to_string())
}

#[derive(Debug, Serialize)]
pub struct RedirectInfo {
    #[serde(serialize_with = "serialize_uri_as_string")]
    pub target_url: Uri,

    #[serde(with = "http_serde::header_map")]
    pub final_headers: HeaderMap,
}