use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct ForwardInfo {
    pub item_id: String,
    pub media_source_id: String,
    pub path: String,
    pub device_id: String,
}

#[derive(Clone, Debug)]
pub struct PathParams {
    pub item_id: String,
    pub media_source_id: String,
}

#[derive(Clone, Debug)]
pub struct ForwardConfig {
    pub expired_seconds: u64,
    pub backend_url: String,
    pub proxy_mode: String,
    pub crypto_key: String,
    pub crypto_iv: String,
    pub emby_server_url: String,
    pub emby_api_key: String,
    pub check_file_existence: bool,
    pub fallback_video_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InfuseAuthorization {
    #[serde(default, rename = "MediaBrowser Token")]
    pub(crate) media_browser_token: String,
    #[serde(default, rename = "Client")]
    pub(crate) client: String,
    #[serde(default, rename = "Device")]
    pub(crate) device: String,
    #[serde(default, rename = "Version")]
    pub(crate) version: String,
    #[serde(default, rename = "DeviceId")]
    pub(crate) device_id: String,
}

static INFUSE_AUTH_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(concat!(r#"([^=,]+)="([^"]+)""#)).expect("Invalid regex pattern")
});

impl InfuseAuthorization {
    pub fn from_header_str(header_str: &str) -> Option<Self> {
        let fields: HashMap<String, String> = INFUSE_AUTH_REGEX
            .captures_iter(header_str)
            .map(|cap| (cap[1].trim().to_string(), cap[2].to_string()))
            .collect();

        if fields.is_empty() {
            return None;
        }

        let value_result = serde_json::to_value(fields);
        value_result
            .ok()
            .and_then(|value| serde_json::from_value(value).ok())
    }

    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            k if k.eq_ignore_ascii_case("mediabrowser token") => {
                Some(self.media_browser_token.clone())
            }
            k if k.eq_ignore_ascii_case("client") => Some(self.client.clone()),
            k if k.eq_ignore_ascii_case("device") => Some(self.device.clone()),
            k if k.eq_ignore_ascii_case("version") => {
                Some(self.version.clone())
            }
            k if k.eq_ignore_ascii_case("deviceid") => {
                Some(self.device_id.clone())
            }
            _ => None,
        }
    }
}
