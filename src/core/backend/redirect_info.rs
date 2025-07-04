use hyper::HeaderMap;
use reqwest::Url;
use serde::{Serialize, Serializer};

fn serialize_url_as_string<S>(url: &Url, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(url.as_str())
}

#[derive(Debug, Serialize)]
pub struct RedirectInfo {
    #[serde(serialize_with = "serialize_url_as_string")]
    pub target_url: Url,
    #[serde(with = "http_serde::header_map")]
    pub final_headers: HeaderMap,
}
