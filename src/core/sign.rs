use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use hyper::Uri;
use serde::Deserialize;

use crate::backend::proxy_mode::ProxyMode;
use crate::{FORWARD_LOGGER_DOMAIN, debug_log};

const PSEUDO_HOST: &str = "local-file.invalid";

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

#[derive(Clone, Debug, Default)]
pub struct Sign {
    pub uri: Option<Uri>,
    pub expired_at: Option<u64>,
}

impl Sign {
    pub fn new(uri: Option<Uri>, expired_at: Option<u64>) -> Self {
        Self { uri, expired_at }
    }

    pub fn from_map(map: &HashMap<String, String>) -> Self {
        debug_log!(FORWARD_LOGGER_DOMAIN, "Map to sign: {:?}", map);
        let mut sign = Sign::default();

        if let Some(uri_str) = map.get("uri") {
            sign.uri = uri_str.parse::<Uri>().ok();
        }

        if let Some(expired_at_str) = map.get("expired_at") {
            sign.expired_at = expired_at_str.parse::<u64>().ok();
        }

        sign
    }

    pub fn to_map(&self) -> HashMap<String, String> {
        debug_log!(
            FORWARD_LOGGER_DOMAIN,
            "Sign to map by uri: {:?} expired_at: {:?}",
            self.uri,
            self.expired_at
        );
        let mut map = HashMap::new();

        if let Some(uri) = &self.uri {
            map.insert("uri".to_string(), uri.to_string());
        }

        if let Some(expired_at) = self.expired_at {
            map.insert("expired_at".to_string(), expired_at.to_string());
        }

        map
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

        if let Some(scheme) = uri.host() {
            return scheme.to_lowercase() == PSEUDO_HOST;
        }

        uri.host().is_none() && uri.path().starts_with('/')
    }
}
