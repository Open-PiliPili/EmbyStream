use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use hyper::Uri;
use serde::{Deserialize, Serialize};

use crate::backend::proxy_mode::ProxyMode;
use crate::uri_serde;

#[derive(Debug, Deserialize)]
pub struct SignParams {
    #[serde(default)]
    pub(crate) sign: String,

    #[serde(default)]
    pub(crate) proxy_mode: ProxyMode,
}

impl Default for SignParams {
    fn default() -> Self {
        Self {
            sign: "".into(),
            proxy_mode: ProxyMode::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sign {
    #[serde(with = "uri_serde", default)]
    pub uri: Option<Uri>,
    #[serde(default)]
    pub expired_at: Option<u64>,
}

impl Sign {
    pub fn new(uri: Option<Uri>, expired_at: Option<u64>) -> Self {
        Self { uri, expired_at }
    }

    pub fn from_map(map: &HashMap<String, String>) -> Self {
        serde_json::from_value(serde_json::json!(map)).unwrap_or_default()
    }

    pub fn to_map(&self) -> HashMap<String, String> {
        let value = serde_json::to_value(self).unwrap_or_default();
        serde_json::from_value(value).unwrap_or_default()
    }

    pub fn is_valid(&self) -> bool {
        let Some(expired_at) = self.expired_at else {
            return false;
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if now >= expired_at + 300 {
            return false;
        }

        let Some(uri) = &self.uri else {
            return false;
        };

        !uri.to_string().is_empty()
    }

    pub fn is_local(&self) -> bool {
        let Some(uri) = &self.uri else {
            return false;
        };

        if let Some(scheme) = uri.scheme_str() {
            return scheme.to_lowercase() == "file";
        }

        uri.host().is_none() && uri.path().starts_with('/')
    }
}
