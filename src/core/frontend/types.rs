use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;

use crate::{FORWARD_LOGGER_DOMAIN, debug_log, error_log};

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
    Regex::new(r#"([^=,]+)="([^"]+)""#).expect("Invalid regex pattern")
});

impl InfuseAuthorization {
    pub fn from_header_str(header_str: &str) -> Option<Self> {
        let fields: HashMap<String, String> = INFUSE_AUTH_REGEX
            .captures_iter(header_str)
            .map(|cap| (cap[1].trim().to_string(), cap[2].to_string()))
            .collect();

        if fields.is_empty() {
            debug_log!(
                FORWARD_LOGGER_DOMAIN,
                "Failed to parse InfuseAuthorization: no fields found in header"
            );
            return None;
        }

        debug_log!(
            FORWARD_LOGGER_DOMAIN,
            "Parsed InfuseAuthorization fields: {:?}",
            fields
        );

        let normalized_fields: HashMap<String, String> = fields
            .into_iter()
            .map(|(k, v)| {
                let normalized_key = match k.to_lowercase().as_str() {
                    "mediabrowser token" => "MediaBrowser Token".to_string(),
                    "token" => "MediaBrowser Token".to_string(),
                    "client" => "Client".to_string(),
                    "device" => "Device".to_string(),
                    "version" => "Version".to_string(),
                    "deviceid" => "DeviceId".to_string(),
                    _ => k,
                };
                (normalized_key, v)
            })
            .collect();

        let value_result = serde_json::to_value(&normalized_fields);
        match value_result {
            Ok(value) => match serde_json::from_value::<Self>(value) {
                Ok(auth) => {
                    debug_log!(
                        FORWARD_LOGGER_DOMAIN,
                        "Successfully parsed InfuseAuthorization: \
                        token=\"{}\", client=\"{}\", device=\"{}\", \
                        version=\"{}\", device_id=\"{}\"",
                        auth.media_browser_token,
                        auth.client,
                        auth.device,
                        auth.version,
                        auth.device_id
                    );
                    Some(auth)
                }
                Err(e) => {
                    error_log!(
                        FORWARD_LOGGER_DOMAIN,
                        "Failed to deserialize InfuseAuthorization: {}, fields: {:?}",
                        e,
                        normalized_fields
                    );
                    None
                }
            },
            Err(e) => {
                error_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "Failed to convert fields to JSON value: {}, fields: {:?}",
                    e,
                    normalized_fields
                );
                None
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            k if k.eq_ignore_ascii_case("mediabrowser token") => {
                Some(self.media_browser_token.clone())
            }
            k if k.eq_ignore_ascii_case("token") => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token_header() {
        let header = r#"Token="335558c5e9194253a20ba0edd65dd4a6", Version="8.3.4", DeviceId="4FEDEC52-9195-4867-BD9F-D77C0ABFEBDD", Device="Apple TV", Client="Infuse-Direct""#;
        let auth = InfuseAuthorization::from_header_str(header);
        assert!(auth.is_some(), "Should parse Token header");
        let auth = auth.unwrap();
        assert_eq!(
            auth.media_browser_token,
            "335558c5e9194253a20ba0edd65dd4a6"
        );
        assert_eq!(auth.version, "8.3.4");
        assert_eq!(auth.device_id, "4FEDEC52-9195-4867-BD9F-D77C0ABFEBDD");
        assert_eq!(auth.device, "Apple TV");
        assert_eq!(auth.client, "Infuse-Direct");
    }

    #[test]
    fn test_parse_mediabrowser_token_header() {
        let header = r#"MediaBrowser Token="335558c5e9194253a20ba0edd65dd4a6", Version="8.3.4", DeviceId="4FEDEC52-9195-4867-BD9F-D77C0ABFEBDD", Device="Apple TV", Client="Infuse-Direct""#;
        let auth = InfuseAuthorization::from_header_str(header);
        assert!(auth.is_some(), "Should parse MediaBrowser Token header");
        let auth = auth.unwrap();
        assert_eq!(
            auth.media_browser_token,
            "335558c5e9194253a20ba0edd65dd4a6"
        );
    }

    #[test]
    fn test_get_token_case_insensitive() {
        let header = r#"Token="test123", Version="8.3.4", DeviceId="test-id", Device="Test", Client="Test""#;
        let auth = InfuseAuthorization::from_header_str(header).unwrap();

        // Test case-insensitive access
        assert_eq!(auth.get("Token"), Some("test123".to_string()));
        assert_eq!(auth.get("token"), Some("test123".to_string()));
        assert_eq!(auth.get("TOKEN"), Some("test123".to_string()));
        assert_eq!(auth.get("MediaBrowser Token"), Some("test123".to_string()));
        assert_eq!(auth.get("mediabrowser token"), Some("test123".to_string()));
    }
}
